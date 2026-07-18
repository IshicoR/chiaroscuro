use std::fmt;

use iced::{
    Background, Element, Length,
    alignment::{Horizontal, Vertical},
    keyboard, mouse,
    widget::{Space, container, mouse_area, text},
};
use iced_plot::{
    AxisLink, Color, LineStyle, MarkerStyle, PlotControls, PlotOverlay, PlotUiMessage, PlotWidget,
    PlotWidgetBuilder, PointId, Series, ShapeId, Tick, TickWeight, Transform,
};

use super::style;

const PLOT_AUTOSCALE_PADDING_RATIO: f64 = 0.05;
const X_EDGE_PADDING_RATIO: f64 = PLOT_AUTOSCALE_PADDING_RATIO / 2.0;
const TOOLTIP_TEXT_SIZE: f32 = 12.0;
const TOOLTIP_PADDING: f32 = 5.0;
const TOOLTIP_OFFSET: f32 = 6.0;
const LIVE_TOOLTIP_WIDTH: f32 = 176.0;

#[derive(Debug, Clone, Copy)]
pub struct AxisSpec {
    label: &'static str,
    min: f64,
    max: f64,
    formatter: fn(f64) -> String,
    value_formatter: fn(f64) -> String,
    tick_count: usize,
}

impl AxisSpec {
    pub fn new(label: &'static str, min: f64, max: f64, formatter: fn(f64) -> String) -> Self {
        Self {
            label,
            min,
            max,
            formatter,
            value_formatter: formatter,
            tick_count: 6,
        }
    }

    pub const fn with_value_formatter(mut self, formatter: fn(f64) -> String) -> Self {
        self.value_formatter = formatter;
        self
    }

    pub const fn with_tick_count(mut self, tick_count: usize) -> Self {
        self.tick_count = tick_count;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TimeSeriesSpec {
    x_axis: AxisSpec,
    y_axis: AxisSpec,
}

impl TimeSeriesSpec {
    pub fn new(x_axis: AxisSpec, y_axis: AxisSpec) -> Self {
        Self { x_axis, y_axis }
    }
}

#[derive(Debug, Clone)]
pub struct LineSeries {
    points: Vec<[f64; 2]>,
    label: &'static str,
    color: Color,
    style: LineStyle,
}

#[derive(Debug, Clone)]
pub enum TimeSeriesMessage {
    Plot(PlotUiMessage),
    BeginPan,
    EndPan,
    Scroll(mouse::ScrollDelta),
    ResetX,
}

impl LineSeries {
    pub fn new(points: Vec<[f64; 2]>, label: &'static str, color: Color, style: LineStyle) -> Self {
        Self {
            points,
            label,
            color,
            style,
        }
    }

    fn into_series(self) -> Series {
        Series::line_only(self.points, self.style)
            .with_label(self.label)
            .with_color(self.color)
    }
}

pub struct TimeSeriesChart {
    plot: PlotWidget,
    series_ids: Vec<ShapeId>,
    series_labels: Vec<&'static str>,
    series_lengths: Vec<usize>,
    value_formatter: fn(f64) -> String,
    focus_index: Option<usize>,
    x_axis_link: AxisLink,
    y_axis_link: AxisLink,
    x_limits: (f64, f64),
    y_limits: (f64, f64),
    cursor_x: Option<f64>,
    panning: bool,
    drag_anchor_x: Option<f64>,
    live_mode: bool,
}

impl fmt::Debug for TimeSeriesChart {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TimeSeriesChart")
            .field("plot", &"PlotWidget")
            .field("series_count", &self.series_ids.len())
            .finish()
    }
}

impl TimeSeriesChart {
    pub fn new(
        spec: TimeSeriesSpec,
        series: impl IntoIterator<Item = LineSeries>,
        x_axis_link: AxisLink,
    ) -> Self {
        let x_axis = spec.x_axis;
        let y_axis = spec.y_axis;
        let x_limits = (x_axis.min, x_axis.max);
        let x_view_limits = padded_x_limits(x_limits);
        let y_limits = (y_axis.min, y_axis.max);
        set_link_view(
            &x_axis_link,
            x_view_limits,
            (x_view_limits.0 + x_view_limits.1) / 2.0,
            (x_view_limits.1 - x_view_limits.0) / 2.0,
        );
        let mut series_ids = Vec::new();
        let mut series_labels = Vec::new();
        let mut series_lengths = Vec::new();
        let y_axis_link = AxisLink::new();
        set_y_link_view(&y_axis_link, y_limits);
        let mut controls = PlotControls::default();
        controls.unbind_drag(mouse::Button::Left);
        controls.unbind_drag(mouse::Button::Right);
        controls.unbind_scroll(keyboard::Modifiers::NONE);
        controls.unbind_scroll(keyboard::Modifiers::CTRL);
        controls.unbind_click(mouse::Button::Left);
        controls.unbind_double_click(mouse::Button::Left);
        controls.unbind_arrow_pan();
        let mut builder = PlotWidgetBuilder::new()
            .with_x_lim(x_limits.0, x_limits.1)
            .with_y_lim(y_axis.min, y_axis.max)
            .with_x_axis_link(x_axis_link.clone())
            .with_y_axis_link(y_axis_link.clone())
            .with_autoscale_on_updates(false)
            .with_x_label(x_axis.label)
            .with_y_label(y_axis.label)
            .with_x_tick_formatter(move |tick| (x_axis.formatter)(tick.value))
            .with_y_tick_formatter(move |tick| (y_axis.formatter)(tick.value))
            .with_x_tick_producer(|min, max| readable_ticks(min, max, 7))
            .with_y_tick_producer(move |min, max| readable_ticks(min, max, y_axis.tick_count))
            .with_tick_label_size(11.0)
            .with_axis_label_size(12.0)
            .with_crosshairs(false)
            .with_highlight_on_hover(false)
            .with_cursor_overlay(true)
            .with_cursor_provider(|_, _| String::new())
            .with_controls(controls)
            .disable_controls_help()
            .with_style(style::plot)
            .with_pick_highlight_provider(|_, point| {
                point.marker_style = Some(MarkerStyle::circle(5.0));
                point.mask_padding = None;
                None
            });

        for series in series {
            series_labels.push(series.label);
            series_lengths.push(series.points.len());
            let series = series.into_series();
            series_ids.push(series.id);
            builder = builder.add_series(series);
        }

        let mut plot = builder
            .build()
            .expect("time-series chart specification must be valid");
        plot.update(PlotUiMessage::ToggleLegend);

        Self {
            plot,
            series_ids,
            series_labels,
            series_lengths,
            value_formatter: y_axis.value_formatter,
            focus_index: None,
            x_axis_link,
            y_axis_link,
            x_limits,
            y_limits,
            cursor_x: None,
            panning: false,
            drag_anchor_x: None,
            live_mode: false,
        }
    }

    pub fn update(&mut self, message: TimeSeriesMessage) -> Option<f64> {
        match message {
            TimeSeriesMessage::Plot(mut message) => {
                let cursor_x = match &mut message {
                    PlotUiMessage::RenderUpdate(update) => {
                        update.cursor_position_ui.take().map(|cursor| cursor.x)
                    },
                    _ => None,
                };
                self.plot.update(message);

                if let Some(cursor_x) = cursor_x {
                    if self.panning {
                        if let Some(anchor_x) = self.drag_anchor_x {
                            self.pan_x(anchor_x, cursor_x);
                            self.drag_anchor_x = Some(cursor_x);
                        } else {
                            self.drag_anchor_x = Some(cursor_x);
                        }
                    }
                    self.cursor_x = Some(cursor_x);
                }

                cursor_x
            },
            TimeSeriesMessage::BeginPan => {
                self.panning = true;
                self.drag_anchor_x = None;
                None
            },
            TimeSeriesMessage::EndPan => {
                self.panning = false;
                self.drag_anchor_x = None;
                None
            },
            TimeSeriesMessage::Scroll(delta) => {
                self.zoom_x(delta);
                None
            },
            TimeSeriesMessage::ResetX => {
                self.reset_x_view();
                None
            },
        }
    }

    pub fn view(&self, focus_x: Option<f64>) -> Element<'_, TimeSeriesMessage> {
        let focus_line = focus_x.map(|x| {
            let line: Element<'_, TimeSeriesMessage> = container(Space::new())
                .width(Length::Fixed(2.0))
                .height(Length::Fill)
                .style(|_| {
                    container::Style::default()
                        .background(Background::Color(Color::from_rgb(0.23, 0.55, 0.95)))
                })
                .into();

            if self.live_mode {
                PlotOverlay::new(line, [self.x_axis_fraction(x), 0.5]).with_axes_transform()
            } else {
                PlotOverlay::new(line, [x, 0.5]).with_transform_y(Transform::axes())
            }
        });
        let tooltip_overlays = self.focus_index.map_or_else(Vec::new, |point_index| {
            let (x_center, _, _) = self.x_axis_link.get();
            let y_center = (self.y_limits.0 + self.y_limits.1) / 2.0;

            if self.live_mode {
                let values = self
                    .series_ids
                    .iter()
                    .zip(&self.series_labels)
                    .filter_map(|(series_id, label)| {
                        let position = self.plot.point_position(PointId {
                            series_id: *series_id,
                            point_index,
                        })?;
                        Some(format!("{label}  {}", (self.value_formatter)(position[1])))
                    })
                    .collect::<Vec<_>>();

                if values.is_empty() {
                    Vec::new()
                } else {
                    let tooltip_x = focus_x.map_or(1.0, |x| self.x_axis_fraction(x));
                    let horizontal = if tooltip_x > 0.5 {
                        Horizontal::Left
                    } else {
                        Horizontal::Right
                    };
                    let offset = if horizontal == Horizontal::Left {
                        [-TOOLTIP_OFFSET, 0.0]
                    } else {
                        [TOOLTIP_OFFSET, 0.0]
                    };
                    let tooltip: Element<'_, TimeSeriesMessage> = container(
                        text(values.join("\n"))
                            .size(TOOLTIP_TEXT_SIZE)
                            .wrapping(iced::widget::text::Wrapping::None),
                    )
                    .width(Length::Fixed(LIVE_TOOLTIP_WIDTH))
                    .padding(TOOLTIP_PADDING)
                    .style(style::tooltip)
                    .into();

                    vec![
                        PlotOverlay::new(tooltip, [tooltip_x, 0.5])
                            .with_axes_transform()
                            .with_anchor_offset(offset)
                            .align_to_anchor(horizontal, Vertical::Center),
                    ]
                }
            } else {
                self.series_ids
                    .iter()
                    .zip(&self.series_labels)
                    .filter_map(|(series_id, label)| {
                        let position = self.plot.point_position(PointId {
                            series_id: *series_id,
                            point_index,
                        })?;
                        let horizontal = if position[0] > x_center {
                            Horizontal::Left
                        } else {
                            Horizontal::Right
                        };
                        let vertical = if position[1] < y_center {
                            Vertical::Top
                        } else {
                            Vertical::Bottom
                        };
                        let offset = [
                            if horizontal == Horizontal::Left {
                                -TOOLTIP_OFFSET
                            } else {
                                TOOLTIP_OFFSET
                            },
                            if vertical == Vertical::Top {
                                TOOLTIP_OFFSET
                            } else {
                                -TOOLTIP_OFFSET
                            },
                        ];
                        let tooltip: Element<'_, TimeSeriesMessage> = container(
                            text(format!("{label}\n{}", (self.value_formatter)(position[1])))
                                .size(TOOLTIP_TEXT_SIZE)
                                .wrapping(iced::widget::text::Wrapping::None),
                        )
                        .padding(TOOLTIP_PADDING)
                        .style(style::tooltip)
                        .into();

                        Some(
                            PlotOverlay::new(tooltip, position)
                                .with_anchor_offset(offset)
                                .align_to_anchor(horizontal, vertical),
                        )
                    })
                    .collect()
            }
        });

        let plot = self.plot.view_with_shapes(
            focus_line.into_iter(),
            tooltip_overlays.into_iter(),
            TimeSeriesMessage::Plot,
        );

        mouse_area(plot)
            .on_press(TimeSeriesMessage::BeginPan)
            .on_release(TimeSeriesMessage::EndPan)
            .on_exit(TimeSeriesMessage::EndPan)
            .on_scroll(TimeSeriesMessage::Scroll)
            .on_double_click(TimeSeriesMessage::ResetX)
            .into()
    }

    pub fn set_series_points(&mut self, index: usize, points: &[[f64; 2]]) {
        if let Some(id) = self.series_ids.get(index) {
            self.plot.set_series_positions(id, points);
            if let Some(length) = self.series_lengths.get_mut(index) {
                *length = points.len();
            }
            if !self.live_mode {
                self.focus_index = None;
            }
        }
    }

    pub fn set_x_limits(&mut self, min: f64, max: f64) {
        // Changing PlotWidget limits autoscales both axes and overwrites the shared camera links.
        let was_live = self.live_mode;
        self.live_mode = false;
        if min < max {
            let limits = (min, max);
            let view_changed = was_live || self.x_limits != limits;
            self.x_limits = limits;
            if view_changed {
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

        if !self.live_mode {
            self.plot.clear_pick();
        }
        self.live_mode = true;

        let previous_view = self.x_axis_link.get();
        let limits_changed = self.x_limits != (min, max);
        if limits_changed {
            self.x_limits = (min, max);
        }

        if follow_end {
            set_link_view_at_end(&self.x_axis_link, self.x_limits, visible_seconds);
        } else if limits_changed {
            set_link_view(
                &self.x_axis_link,
                padded_x_limits(self.x_limits),
                previous_view.0,
                previous_view.1,
            );
        }
    }

    pub fn set_y_limits(&mut self, min: f64, max: f64) {
        if min.is_finite() && max.is_finite() && min < max && self.y_limits != (min, max) {
            self.y_limits = (min, max);
            set_y_link_view(&self.y_axis_link, self.y_limits);
        }
    }

    pub fn set_focus_index(&mut self, point_index: Option<usize>) {
        if self.focus_index == point_index {
            return;
        }

        if !self.live_mode {
            self.plot.clear_pick();
            if let Some(point_index) = point_index {
                for (series_id, series_length) in self.series_ids.iter().zip(&self.series_lengths) {
                    if point_index >= *series_length {
                        continue;
                    }
                    self.plot.add_pick_point(PointId {
                        series_id: *series_id,
                        point_index,
                    });
                }
            }
        }
        self.focus_index = point_index;
    }

    #[doc(hidden)]
    pub const fn focus_index(&self) -> Option<usize> {
        self.focus_index
    }

    pub fn x_view_max(&self) -> f64 {
        let (center, half_extent, _) = self.x_axis_link.get();
        center + half_extent
    }

    fn x_axis_fraction(&self, x: f64) -> f64 {
        let (center, half_extent, _) = self.x_axis_link.get();
        let span = half_extent * 2.0;
        if !x.is_finite() || !span.is_finite() || span <= f64::EPSILON {
            return 0.5;
        }

        ((x - (center - half_extent)) / span).clamp(0.0, 1.0)
    }

    fn pan_x(&self, anchor_x: f64, cursor_x: f64) {
        let (center, half_extent, _) = self.x_axis_link.get();
        set_link_view(
            &self.x_axis_link,
            padded_x_limits(self.x_limits),
            center + anchor_x - cursor_x,
            half_extent,
        );
    }

    fn zoom_x(&self, delta: mouse::ScrollDelta) {
        let steps = match delta {
            mouse::ScrollDelta::Lines { y, .. } => f64::from(y),
            mouse::ScrollDelta::Pixels { y, .. } => f64::from(y) / 40.0,
        };
        if steps == 0.0 {
            return;
        }

        let (center, half_extent, _) = self.x_axis_link.get();
        let factor = 2.0_f64.powf(-steps * 0.2);
        let anchor = self
            .cursor_x
            .unwrap_or(center)
            .clamp(self.x_limits.0, self.x_limits.1);
        set_link_view(
            &self.x_axis_link,
            padded_x_limits(self.x_limits),
            anchor + (center - anchor) * factor,
            half_extent * factor,
        );
    }

    fn reset_x_view(&self) {
        let view_limits = padded_x_limits(self.x_limits);
        set_link_view(
            &self.x_axis_link,
            view_limits,
            (view_limits.0 + view_limits.1) / 2.0,
            (view_limits.1 - view_limits.0) / 2.0,
        );
    }
}

fn padded_x_limits(limits: (f64, f64)) -> (f64, f64) {
    let padding = (limits.1 - limits.0) * X_EDGE_PADDING_RATIO;
    (limits.0 - padding, limits.1 + padding)
}

fn set_link_view_at_end(link: &AxisLink, limits: (f64, f64), visible_seconds: f64) {
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

fn set_y_link_view(link: &AxisLink, limits: (f64, f64)) {
    let span = limits.1 - limits.0;
    let padding = span * PLOT_AUTOSCALE_PADDING_RATIO;
    link.set((limits.0 + limits.1) / 2.0, span / 2.0 + padding);
}

fn set_link_view(link: &AxisLink, limits: (f64, f64), center: f64, half_extent: f64) {
    let full_half_extent = (limits.1 - limits.0) / 2.0;
    let half_extent = half_extent.clamp(full_half_extent / 1_000.0, full_half_extent);
    let min_center = limits.0 + half_extent;
    let max_center = limits.1 - half_extent;
    let center = if min_center <= max_center {
        center.clamp(min_center, max_center)
    } else {
        (limits.0 + limits.1) / 2.0
    };
    link.set(center, half_extent);
}

fn readable_ticks(min: f64, max: f64, target_count: usize) -> Vec<Tick> {
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

fn nice_tick_step(raw_step: f64) -> f64 {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_chart(points: Vec<[f64; 2]>) -> TimeSeriesChart {
        TimeSeriesChart::new(
            TimeSeriesSpec::new(
                AxisSpec::new("Time", 0.0, 10.0, |value| value.to_string()),
                AxisSpec::new("Value", 0.0, 10.0, |value| value.to_string()),
            ),
            [LineSeries::new(
                points,
                "Value",
                Color::from_rgb(1.0, 1.0, 1.0),
                LineStyle::solid(),
            )],
            AxisLink::new(),
        )
    }

    fn assert_close(left: f64, right: f64) {
        assert!((left - right).abs() < 1e-9, "{left} != {right}");
    }

    #[test]
    fn x_view_is_clamped_to_the_data_range() {
        let link = AxisLink::new();
        let limits = (0.0, 100.0);

        set_link_view(&link, limits, -50.0, 10.0);
        assert_eq!(link.get().0, 10.0);
        assert_eq!(link.get().1, 10.0);

        set_link_view(&link, limits, 150.0, 10.0);
        assert_eq!(link.get().0, 90.0);
        assert_eq!(link.get().1, 10.0);

        set_link_view(&link, limits, 10.0, 500.0);
        assert_eq!(link.get().0, 50.0);
        assert_eq!(link.get().1, 50.0);
    }

    #[test]
    fn x_view_padding_is_stable_around_the_data_range() {
        assert_eq!(padded_x_limits((0.0, 100.0)), (-2.5, 102.5));
    }

    #[test]
    fn live_x_view_keeps_offline_style_padding_after_the_latest_time() {
        let link = AxisLink::new();

        set_link_view_at_end(&link, (0.0, 100.0), 12.0);

        let (center, half_extent, _) = link.get();
        assert_close(center - half_extent, 87.7);
        assert_close(center + half_extent, 100.3);
    }

    #[test]
    fn live_data_bounds_keep_a_fixed_width_camera_window() {
        let mut chart = test_chart(vec![[0.0, 1.0], [100.0, 2.0]]);

        chart.set_live_x_limits(0.0, 100.0, 12.0, true);

        let (center, half_extent, _) = chart.x_axis_link.get();
        assert_close(center - half_extent, 87.7);
        assert_close(center + half_extent, 100.3);
        assert_close(chart.x_axis_fraction(100.0), 1.025 / 1.05);
        assert_eq!(chart.x_limits, (0.0, 100.0));

        chart.set_x_limits(0.0, 100.0);

        let (center, half_extent, _) = chart.x_axis_link.get();
        assert_eq!(center - half_extent, -2.5);
        assert_eq!(center + half_extent, 102.5);
    }

    #[test]
    fn live_focus_screen_position_is_stable_as_time_advances() {
        let mut chart = test_chart(vec![[0.0, 1.0], [100.0, 2.0]]);
        chart.set_live_x_limits(0.0, 100.0, 12.0, true);
        let first_position = chart.x_axis_fraction(100.0);

        chart.set_live_x_limits(0.0, 101.0, 12.0, true);
        let next_position = chart.x_axis_fraction(101.0);

        assert_close(first_position, next_position);
        assert_close(next_position, 1.025 / 1.05);
    }

    #[test]
    fn live_series_updates_keep_the_focus_active() {
        let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);
        chart.set_live_x_limits(0.0, 2.0, 1.0, true);
        chart.set_focus_index(Some(1));

        chart.set_series_points(0, &[[1.0, 2.0], [2.0, 3.0]]);

        assert_eq!(chart.focus_index(), Some(1));
    }

    #[test]
    fn readable_ticks_keep_axis_labels_sparse() {
        let ticks = readable_ticks(0.0, 100.0, 6);

        assert_eq!(
            ticks.iter().map(|tick| tick.value).collect::<Vec<_>>(),
            vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0]
        );
        assert!(ticks.iter().all(|tick| tick.line_type == TickWeight::Major));
    }

    #[test]
    fn axis_can_format_tooltip_values_separately_from_ticks() {
        let axis = AxisSpec::new("Speed", 0.0, 400.0, |value| format!("{value:.0}"))
            .with_value_formatter(|value| format!("{value:.1} km/h"))
            .with_tick_count(9);

        assert_eq!((axis.formatter)(123.45), "123");
        assert_eq!((axis.value_formatter)(123.45), "123.5 km/h");
        assert_eq!(axis.tick_count, 9);
    }
}
