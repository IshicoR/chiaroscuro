use std::sync::Arc;

use iced::mouse;
use iced_plot::{AxisLink, Tick, TickWeight};

use super::{AxisSpec, TimeSeriesChart};

const PLOT_AUTOSCALE_PADDING_RATIO: f64 = 0.05;
const X_EDGE_PADDING_RATIO: f64 = PLOT_AUTOSCALE_PADDING_RATIO / 2.0;
const SCROLL_PIXELS_PER_STEP: f64 = 40.0;
const SCROLL_PAN_VIEWPORT_RATIO: f64 = 0.1;

pub(super) struct AxisState {
    pub(super) value_formatter: fn(f64) -> String,
    pub(super) x_link: AxisLink,
    pub(super) y_link: AxisLink,
    pub(super) x_label: &'static str,
    pub(super) x_formatter_scale: f64,
    pub(super) x_limits: (f64, f64),
    pub(super) y_limits: (f64, f64),
    pub(super) live_mode: bool,
}

impl TimeSeriesChart {
    pub fn set_x_axis(&mut self, axis: AxisSpec) {
        if self.axis.x_label == axis.label && self.axis.x_formatter_scale == axis.formatter_scale {
            return;
        }

        self.axis.x_label = axis.label;
        self.axis.x_formatter_scale = axis.formatter_scale;
        self.plot.set_x_axis_label(axis.label);
        let formatter_scale = axis.formatter_scale;
        self.plot.set_x_axis_formatter(Arc::new(move |tick| {
            (axis.formatter)(tick.value * formatter_scale)
        }));
    }

    pub fn set_x_limits(&mut self, min: f64, max: f64) {
        let was_live = self.axis.live_mode;
        self.axis.live_mode = false;
        if min < max {
            let limits = (min, max);
            let view_changed = was_live || self.axis.x_limits != limits;
            self.axis.x_limits = limits;
            if view_changed {
                // PlotWidget autoscales both axes when either fixed limit changes. Keep its
                // limits synchronized after the caller has finalized the current Y range.
                self.plot.set_x_lim(min, max);
                self.plot
                    .set_y_lim(self.axis.y_limits.0, self.axis.y_limits.1);
                self.reset_x_view();
            }
        }
    }

    pub fn set_live_x_limits(
        &mut self,
        min: f64,
        max: f64,
        visible_seconds: f64,
        follow_end: bool,
    ) {
        if !min.is_finite()
            || !max.is_finite()
            || !visible_seconds.is_finite()
            || min >= max
            || visible_seconds <= 0.0
        {
            return;
        }

        if !self.axis.live_mode {
            self.plot.clear_pick();
        }
        self.axis.live_mode = true;

        let previous_view = self.axis.x_link.get();
        let limits_changed = self.axis.x_limits != (min, max);
        if limits_changed {
            self.axis.x_limits = (min, max);
        }

        if follow_end {
            set_link_view_at_end(&self.axis.x_link, self.axis.x_limits, visible_seconds);
        } else if limits_changed {
            set_link_view(
                &self.axis.x_link,
                padded_x_limits(self.axis.x_limits),
                previous_view.0,
                previous_view.1,
            );
        }
    }

    pub fn set_y_limits(&mut self, min: f64, max: f64) {
        if min.is_finite() && max.is_finite() && min < max && self.axis.y_limits != (min, max) {
            self.axis.y_limits = (min, max);
            set_y_link_view(&self.axis.y_link, self.axis.y_limits);
        }
    }

    #[doc(hidden)]
    pub const fn x_axis_label(&self) -> &'static str {
        self.axis.x_label
    }

    #[doc(hidden)]
    pub const fn x_limits(&self) -> (f64, f64) {
        self.axis.x_limits
    }

    pub fn x_view_max(&self) -> f64 {
        let (center, half_extent, _) = self.axis.x_link.get();
        center + half_extent
    }

    pub(super) fn x_axis_fraction(&self, x: f64) -> f64 {
        let (center, half_extent, _) = self.axis.x_link.get();
        let span = half_extent * 2.0;
        if !x.is_finite() || !span.is_finite() || span <= f64::EPSILON {
            return 0.5;
        }

        ((x - (center - half_extent)) / span).clamp(0.0, 1.0)
    }

    pub(super) fn pan_x(&self, delta: mouse::ScrollDelta) {
        let steps = scroll_steps(delta);
        if steps == 0.0 {
            return;
        }

        let (center, half_extent, _) = self.axis.x_link.get();
        set_link_view(
            &self.axis.x_link,
            padded_x_limits(self.axis.x_limits),
            center - steps * half_extent * 2.0 * SCROLL_PAN_VIEWPORT_RATIO,
            half_extent,
        );
    }

    pub(super) fn zoom_x(&self, delta: mouse::ScrollDelta) {
        let steps = scroll_steps(delta);
        if steps == 0.0 {
            return;
        }

        let (center, half_extent, _) = self.axis.x_link.get();
        let factor = 2.0_f64.powf(-steps * 0.2);
        let anchor = self
            .interaction
            .cursor_position
            .map(|cursor| cursor[0])
            .unwrap_or(center)
            .clamp(self.axis.x_limits.0, self.axis.x_limits.1);
        set_link_view(
            &self.axis.x_link,
            padded_x_limits(self.axis.x_limits),
            anchor + (center - anchor) * factor,
            half_extent * factor,
        );
    }

    pub(super) fn reset_x_view(&self) {
        let view_limits = padded_x_limits(self.axis.x_limits);
        set_link_view(
            &self.axis.x_link,
            view_limits,
            (view_limits.0 + view_limits.1) / 2.0,
            (view_limits.1 - view_limits.0) / 2.0,
        );
    }
}

pub(super) fn scroll_steps(delta: mouse::ScrollDelta) -> f64 {
    match delta {
        mouse::ScrollDelta::Lines { y, .. } => f64::from(y),
        mouse::ScrollDelta::Pixels { y, .. } => f64::from(y) / SCROLL_PIXELS_PER_STEP,
    }
}

pub(super) fn padded_x_limits(limits: (f64, f64)) -> (f64, f64) {
    let padding = (limits.1 - limits.0) * X_EDGE_PADDING_RATIO;
    (limits.0 - padding, limits.1 + padding)
}

pub(super) fn set_link_view_at_end(link: &AxisLink, limits: (f64, f64), visible_seconds: f64) {
    let span = limits.1 - limits.0;
    let visible_seconds = visible_seconds.min(span).max(span / 1_000.0);
    let padding = visible_seconds * X_EDGE_PADDING_RATIO;
    let half_extent = visible_seconds / 2.0 + padding;
    set_link_view(
        link,
        padded_x_limits(limits),
        limits.1 - visible_seconds / 2.0,
        half_extent,
    );
}

pub(super) fn set_y_link_view(link: &AxisLink, limits: (f64, f64)) {
    let span = limits.1 - limits.0;
    let padding = span * PLOT_AUTOSCALE_PADDING_RATIO;
    link.set((limits.0 + limits.1) / 2.0, span / 2.0 + padding);
}

pub(super) fn set_link_view(link: &AxisLink, limits: (f64, f64), center: f64, half_extent: f64) {
    let full_half_extent = (limits.1 - limits.0) / 2.0;
    let half_extent = half_extent.clamp(full_half_extent / 1_000.0, full_half_extent);
    let min_center = limits.0 + half_extent;
    let max_center = limits.1 - half_extent;
    let center = if min_center <= max_center {
        center.clamp(min_center, max_center)
    } else {
        (limits.0 + limits.1) / 2.0
    };
    let (current_center, current_half_extent, _) = link.get();
    if current_center != center || current_half_extent != half_extent {
        link.set(center, half_extent);
    }
}

pub(super) fn readable_ticks(min: f64, max: f64, target_count: usize) -> Vec<Tick> {
    let span = max - min;
    if !span.is_finite() || span <= 0.0 {
        return Vec::new();
    }

    let interval_count = target_count.saturating_sub(1).max(1) as f64;
    let step = nice_tick_step(span / interval_count);
    let mut value = (min / step).ceil() * step;
    let mut ticks = Vec::with_capacity(target_count);

    while value <= max + step * 1e-9 {
        ticks.push(Tick::new(value, step, TickWeight::Major));
        value += step;
    }

    ticks
}

pub(super) fn nice_tick_step(raw_step: f64) -> f64 {
    if !raw_step.is_finite() || raw_step <= 0.0 {
        return 1.0;
    }

    let magnitude = 10.0_f64.powf(raw_step.log10().floor());
    let normalized = raw_step / magnitude;
    let multiplier = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    multiplier * magnitude
}
