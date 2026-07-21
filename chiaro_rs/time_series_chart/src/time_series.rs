use std::{fmt, sync::Arc};

use chiaro_widgets::context_menu as chart_context_menu;
use iced::{
    Background, Element, Length, Point, Theme,
    alignment::{Horizontal, Vertical},
    keyboard, mouse,
    widget::{Column, Row, Space, button, container, mouse_area, text},
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
const TOOLTIP_WIDTH: f32 = 240.0;
const TOOLTIP_ROW_SPACING: f32 = 3.0;
const TOOLTIP_ITEM_SPACING: f32 = 6.0;
const TOOLTIP_MARKER_WIDTH: f32 = 12.0;
const TOOLTIP_MARKER_HEIGHT: f32 = 3.0;
const CONTEXT_MENU_WIDTH: f32 = 168.0;
const SCROLL_PIXELS_PER_STEP: f64 = 40.0;
const SCROLL_PAN_VIEWPORT_RATIO: f64 = 0.1;

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
    BeginCursorDrag,
    EndCursorDrag,
    PanX(mouse::ScrollDelta),
    ZoomX(mouse::ScrollDelta),
    ResetX,
    OpenContextMenu(Point),
    CloseContextMenu,
    ToggleTooltips,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollAction {
    PanX,
    ZoomX,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ChartContext {
    local_position: Point,
    data_position: Option<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
struct TooltipValue {
    position: [f64; 2],
    label: &'static str,
    value: String,
    color: Color,
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
    series_colors: Vec<Color>,
    series_lengths: Vec<usize>,
    value_formatter: fn(f64) -> String,
    focus_index: Option<usize>,
    x_axis_link: AxisLink,
    y_axis_link: AxisLink,
    x_axis_label: &'static str,
    x_limits: (f64, f64),
    y_limits: (f64, f64),
    cursor_position: Option<[f64; 2]>,
    cursor_dragging: bool,
    live_mode: bool,
    tooltips_visible: bool,
    context: Option<ChartContext>,
    #[cfg(test)]
    series_update_count: usize,
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
        let mut series_colors = Vec::new();
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
            series_colors.push(series.color);
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
            series_colors,
            series_lengths,
            value_formatter: y_axis.value_formatter,
            focus_index: None,
            x_axis_link,
            y_axis_link,
            x_axis_label: x_axis.label,
            x_limits,
            y_limits,
            cursor_position: None,
            cursor_dragging: false,
            live_mode: false,
            tooltips_visible: true,
            context: None,
            #[cfg(test)]
            series_update_count: 0,
        }
    }

    pub fn update(&mut self, message: TimeSeriesMessage) -> Option<f64> {
        match message {
            TimeSeriesMessage::Plot(mut message) => {
                let cursor_position = match &mut message {
                    PlotUiMessage::RenderUpdate(update) => update
                        .cursor_position_ui
                        .take()
                        .map(|cursor| [cursor.x, cursor.y]),
                    _ => None,
                };
                self.plot.update(message);

                cursor_position.and_then(|cursor| self.track_cursor(cursor))
            },
            TimeSeriesMessage::BeginCursorDrag => {
                self.cursor_dragging = true;
                self.cursor_position.map(|cursor| cursor[0])
            },
            TimeSeriesMessage::EndCursorDrag => {
                self.cursor_dragging = false;
                None
            },
            TimeSeriesMessage::PanX(delta) => {
                self.pan_x(delta);
                None
            },
            TimeSeriesMessage::ZoomX(delta) => {
                self.zoom_x(delta);
                None
            },
            TimeSeriesMessage::ResetX => {
                self.reset_x_view();
                None
            },
            TimeSeriesMessage::OpenContextMenu(local_position) => {
                self.context = Some(ChartContext {
                    local_position,
                    data_position: self.cursor_position,
                });
                None
            },
            TimeSeriesMessage::CloseContextMenu => {
                self.context = None;
                None
            },
            TimeSeriesMessage::ToggleTooltips => {
                self.tooltips_visible = !self.tooltips_visible;
                self.context = None;
                None
            },
        }
    }

    pub fn view(
        &self,
        focus_x: Option<f64>,
        modifiers: keyboard::Modifiers,
    ) -> Element<'_, TimeSeriesMessage> {
        let focus_line = focus_x.map(|x| {
            let line: Element<'_, TimeSeriesMessage> = container(Space::new())
                .width(Length::Fixed(2.0))
                .height(Length::Fill)
                .style(|theme: &Theme| {
                    container::Style::default().background(Background::Color(
                        theme.extended_palette().primary.base.color,
                    ))
                })
                .into();

            if self.live_mode {
                PlotOverlay::new(line, [self.x_axis_fraction(x), 0.5]).with_axes_transform()
            } else {
                PlotOverlay::new(line, [x, 0.5]).with_transform_y(Transform::axes())
            }
        });
        let tooltip_overlays = self.tooltip_overlays(focus_x);

        let plot = self.plot.view_with_shapes(
            focus_line.into_iter(),
            tooltip_overlays.into_iter(),
            TimeSeriesMessage::Plot,
        );

        let plot = mouse_area(plot)
            .on_press(TimeSeriesMessage::BeginCursorDrag)
            .on_release(TimeSeriesMessage::EndCursorDrag)
            .on_exit(TimeSeriesMessage::EndCursorDrag)
            .on_double_click(TimeSeriesMessage::ResetX);
        let plot = match scroll_action(modifiers) {
            Some(ScrollAction::PanX) => plot.on_scroll(TimeSeriesMessage::PanX),
            Some(ScrollAction::ZoomX) => plot.on_scroll(TimeSeriesMessage::ZoomX),
            None => plot,
        };

        let menu_label = if self.tooltips_visible {
            "Hide tooltips"
        } else {
            "Show tooltips"
        };
        let menu_item = container(text(menu_label).size(13))
            .width(Length::Fill)
            .align_x(Horizontal::Left);
        let menu = container(
            button(menu_item)
                .width(Length::Fill)
                .padding([6, 8])
                .style(style::context_menu_item)
                .on_press(TimeSeriesMessage::ToggleTooltips),
        )
        .width(Length::Fixed(CONTEXT_MENU_WIDTH))
        .padding(4)
        .style(style::context_menu);

        chart_context_menu(plot, menu)
            .open(self.context.is_some())
            .on_open(TimeSeriesMessage::OpenContextMenu)
            .on_close(TimeSeriesMessage::CloseContextMenu)
            .into()
    }

    pub fn cancel_interaction(&mut self) {
        self.cursor_dragging = false;
        self.context = None;
    }

    pub fn set_series_points(&mut self, index: usize, points: &[[f64; 2]]) {
        if let Some(id) = self.series_ids.get(index) {
            if points.is_empty() && self.series_lengths.get(index) == Some(&0) {
                return;
            }
            #[cfg(test)]
            {
                self.series_update_count += 1;
            }
            self.plot.set_series_positions(id, points);
            if let Some(length) = self.series_lengths.get_mut(index) {
                *length = points.len();
            }
            if !self.live_mode {
                self.focus_index = None;
            }
        }
    }

    /// Trims an existing live series and appends only the samples captured
    /// since its previous dashboard refresh.
    pub fn update_live_series_points(
        &mut self,
        index: usize,
        minimum_x: f64,
        appended: &[[f64; 2]],
    ) {
        let Some(id) = self.series_ids.get(index).copied() else {
            return;
        };

        let should_trim = minimum_x.is_finite()
            && self
                .plot
                .point_position(PointId {
                    series_id: id,
                    point_index: 0,
                })
                .is_some_and(|position| position[0] < minimum_x);
        if appended.is_empty() && !should_trim {
            return;
        }

        let mut updated_length = None;
        if self
            .plot
            .update_series(&id, |series| {
                let removed = if minimum_x.is_finite() {
                    series
                        .positions
                        .partition_point(|position| position[0] < minimum_x)
                } else {
                    0
                };
                let previous_length = series.positions.len();

                if let Some(colors) = &mut series.point_colors {
                    colors.resize(previous_length, series.color);
                    colors.drain(..removed);
                    colors.extend(std::iter::repeat_n(series.color, appended.len()));
                }

                series.positions.drain(..removed);
                series.positions.extend_from_slice(appended);
                updated_length = Some(series.positions.len());
            })
            .is_err()
        {
            return;
        }

        #[cfg(test)]
        {
            self.series_update_count += 1;
        }
        if let Some(length) = updated_length
            && let Some(series_length) = self.series_lengths.get_mut(index)
        {
            *series_length = length;
        }
        if !self.live_mode {
            self.focus_index = None;
        }
    }

    pub fn set_x_axis(&mut self, axis: AxisSpec) {
        if self.x_axis_label == axis.label {
            return;
        }

        self.x_axis_label = axis.label;
        self.plot.set_x_axis_label(axis.label);
        self.plot
            .set_x_axis_formatter(Arc::new(move |tick| (axis.formatter)(tick.value)));
    }

    pub fn set_x_limits(&mut self, min: f64, max: f64) {
        let was_live = self.live_mode;
        self.live_mode = false;
        if min < max {
            let limits = (min, max);
            let view_changed = was_live || self.x_limits != limits;
            self.x_limits = limits;
            if view_changed {
                // PlotWidget autoscales both axes when either fixed limit changes. Keep its
                // limits synchronized after the caller has finalized the current Y range.
                self.plot.set_x_lim(min, max);
                self.plot.set_y_lim(self.y_limits.0, self.y_limits.1);
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

    #[doc(hidden)]
    pub const fn x_axis_label(&self) -> &'static str {
        self.x_axis_label
    }

    #[doc(hidden)]
    pub const fn x_limits(&self) -> (f64, f64) {
        self.x_limits
    }

    #[doc(hidden)]
    pub fn series_length(&self, index: usize) -> Option<usize> {
        self.series_lengths.get(index).copied()
    }

    #[doc(hidden)]
    pub const fn is_cursor_dragging(&self) -> bool {
        self.cursor_dragging
    }

    #[doc(hidden)]
    pub const fn tooltips_visible(&self) -> bool {
        self.tooltips_visible
    }

    #[doc(hidden)]
    pub const fn context_menu_target(&self) -> Option<[f64; 2]> {
        match self.context {
            Some(context) => context.data_position,
            None => None,
        }
    }

    #[doc(hidden)]
    pub const fn is_context_menu_open(&self) -> bool {
        self.context.is_some()
    }

    fn tooltip_focus_index(&self) -> Option<usize> {
        self.focus_index
            .filter(|_| self.tooltips_visible && self.context.is_none())
    }

    fn tooltip_values(&self, point_index: usize) -> Vec<TooltipValue> {
        self.series_ids
            .iter()
            .zip(&self.series_labels)
            .zip(&self.series_colors)
            .filter_map(|((series_id, label), color)| {
                let position = self.plot.point_position(PointId {
                    series_id: *series_id,
                    point_index,
                })?;
                if !position[0].is_finite() || !position[1].is_finite() {
                    return None;
                }

                Some(TooltipValue {
                    position,
                    label,
                    value: (self.value_formatter)(position[1]),
                    color: *color,
                })
            })
            .collect()
    }

    fn tooltip_overlays(&self, focus_x: Option<f64>) -> Vec<PlotOverlay<'_, TimeSeriesMessage>> {
        let Some(point_index) = self.tooltip_focus_index() else {
            return Vec::new();
        };
        let values = self.tooltip_values(point_index);
        let Some(first) = values.first() else {
            return Vec::new();
        };
        let data_x = focus_x
            .filter(|x| x.is_finite())
            .unwrap_or(first.position[0]);
        let (anchor_x, horizontal) = if self.live_mode {
            let anchor_x = self.x_axis_fraction(data_x);
            let horizontal = if anchor_x > 0.5 {
                Horizontal::Left
            } else {
                Horizontal::Right
            };
            (anchor_x, horizontal)
        } else {
            let (x_center, _, _) = self.x_axis_link.get();
            let horizontal = if data_x > x_center {
                Horizontal::Left
            } else {
                Horizontal::Right
            };
            (data_x, horizontal)
        };
        let offset = if horizontal == Horizontal::Left {
            [-TOOLTIP_OFFSET, 0.0]
        } else {
            [TOOLTIP_OFFSET, 0.0]
        };
        let tooltip = combined_tooltip(values);
        let overlay = if self.live_mode {
            PlotOverlay::new(tooltip, [anchor_x, 0.5]).with_axes_transform()
        } else {
            PlotOverlay::new(tooltip, [anchor_x, 0.5]).with_transform_y(Transform::axes())
        };

        vec![
            overlay
                .with_anchor_offset(offset)
                .align_to_anchor(horizontal, Vertical::Center),
        ]
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

    fn pan_x(&self, delta: mouse::ScrollDelta) {
        let steps = scroll_steps(delta);
        if steps == 0.0 {
            return;
        }

        let (center, half_extent, _) = self.x_axis_link.get();
        set_link_view(
            &self.x_axis_link,
            padded_x_limits(self.x_limits),
            center - steps * half_extent * 2.0 * SCROLL_PAN_VIEWPORT_RATIO,
            half_extent,
        );
    }

    fn track_cursor(&mut self, cursor: [f64; 2]) -> Option<f64> {
        self.cursor_position = Some(cursor);
        self.cursor_dragging.then_some(cursor[0])
    }

    fn zoom_x(&self, delta: mouse::ScrollDelta) {
        let steps = scroll_steps(delta);
        if steps == 0.0 {
            return;
        }

        let (center, half_extent, _) = self.x_axis_link.get();
        let factor = 2.0_f64.powf(-steps * 0.2);
        let anchor = self
            .cursor_position
            .map(|cursor| cursor[0])
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

fn combined_tooltip(values: Vec<TooltipValue>) -> Element<'static, TimeSeriesMessage> {
    let content: Column<'static, TimeSeriesMessage> = values.into_iter().fold(
        Column::new().spacing(TOOLTIP_ROW_SPACING),
        |content, value| {
            let marker_color = value.color;
            let marker = container(Space::new())
                .width(Length::Fixed(TOOLTIP_MARKER_WIDTH))
                .height(Length::Fixed(TOOLTIP_MARKER_HEIGHT))
                .style(move |_| {
                    container::Style::default().background(Background::Color(marker_color))
                });
            let label = text(value.label)
                .size(TOOLTIP_TEXT_SIZE)
                .width(Length::Fill)
                .wrapping(iced::widget::text::Wrapping::None);
            let value = text(value.value)
                .size(TOOLTIP_TEXT_SIZE)
                .wrapping(iced::widget::text::Wrapping::None);
            let row = Row::new()
                .push(marker)
                .push(label)
                .push(value)
                .spacing(TOOLTIP_ITEM_SPACING)
                .align_y(Vertical::Center);

            content.push(row)
        },
    );

    container(content)
        .width(Length::Fixed(TOOLTIP_WIDTH))
        .padding(TOOLTIP_PADDING)
        .style(style::tooltip)
        .into()
}

fn scroll_action(modifiers: keyboard::Modifiers) -> Option<ScrollAction> {
    if modifiers.control() {
        Some(ScrollAction::ZoomX)
    } else if modifiers.shift() {
        Some(ScrollAction::PanX)
    } else {
        None
    }
}

fn scroll_steps(delta: mouse::ScrollDelta) -> f64 {
    match delta {
        mouse::ScrollDelta::Lines { y, .. } => f64::from(y),
        mouse::ScrollDelta::Pixels { y, .. } => f64::from(y) / SCROLL_PIXELS_PER_STEP,
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
    let (current_center, current_half_extent, _) = link.get();
    if current_center != center || current_half_extent != half_extent {
        link.set(center, half_extent);
    }
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

    fn multi_series_chart() -> TimeSeriesChart {
        TimeSeriesChart::new(
            TimeSeriesSpec::new(
                AxisSpec::new("Time", 0.0, 10.0, |value| value.to_string()),
                AxisSpec::new("Value", 0.0, 10.0, |value| format!("{value:.1}")),
            ),
            [
                LineSeries::new(
                    vec![[0.0, 1.0], [1.0, 2.0]],
                    "Primary",
                    Color::from_rgb(1.0, 0.0, 0.0),
                    LineStyle::solid(),
                ),
                LineSeries::new(
                    vec![[0.0, 3.0], [1.0, 4.0]],
                    "Reference",
                    Color::from_rgb(0.0, 1.0, 0.0),
                    LineStyle::solid(),
                ),
            ],
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
    fn identical_shared_axis_updates_do_not_advance_the_link_version() {
        let link = AxisLink::new();
        let limits = (-2.5, 102.5);

        set_link_view(&link, limits, 50.0, 52.5);
        let first_version = link.get().2;
        set_link_view(&link, limits, 50.0, 52.5);

        assert_eq!(link.get().2, first_version);
    }

    #[test]
    fn repeatedly_clearing_an_empty_series_is_a_no_op() {
        let mut chart = test_chart(vec![[0.0, 0.0]]);

        chart.set_series_points(0, &[]);
        assert_eq!(chart.series_update_count, 1);

        chart.set_series_points(0, &[]);
        assert_eq!(chart.series_update_count, 1);

        chart.set_series_points(0, &[[0.0, 1.0]]);
        chart.set_series_points(0, &[]);
        assert_eq!(chart.series_update_count, 3);

        chart.set_series_points(0, &[]);
        assert_eq!(chart.series_update_count, 3);
    }

    #[test]
    fn live_series_updates_only_append_and_trim_the_retained_window() {
        let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0], [2.0, 3.0]]);

        chart.update_live_series_points(0, 1.0, &[[3.0, 4.0]]);

        assert_eq!(chart.series_length(0), Some(3));
        assert_eq!(chart.series_update_count, 1);

        chart.update_live_series_points(0, 1.0, &[]);
        assert_eq!(chart.series_update_count, 1);

        chart.update_live_series_points(0, 2.5, &[]);
        assert_eq!(chart.series_length(0), Some(1));
        assert_eq!(chart.series_update_count, 2);
    }

    #[test]
    fn x_view_padding_is_stable_around_the_data_range() {
        assert_eq!(padded_x_limits((0.0, 100.0)), (-2.5, 102.5));
    }

    #[test]
    fn x_axis_spec_can_switch_without_rebuilding_the_chart() {
        let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);

        chart.set_x_axis(AxisSpec::new("Lap distance", 0.0, 1.0, |value| {
            format!("{:.0}%", value * 100.0)
        }));
        chart.set_x_limits(0.0, 1.0);

        assert_eq!(chart.x_axis_label(), "Lap distance");
        assert_eq!(chart.x_limits(), (0.0, 1.0));
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
    fn cancelling_an_interaction_stops_cursor_dragging() {
        let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);

        chart.update(TimeSeriesMessage::BeginCursorDrag);
        chart.update(TimeSeriesMessage::OpenContextMenu(Point::new(40.0, 30.0)));
        assert!(chart.is_cursor_dragging());
        assert!(chart.is_context_menu_open());

        chart.cancel_interaction();

        assert!(!chart.is_cursor_dragging());
        assert!(!chart.is_context_menu_open());
    }

    #[test]
    fn tooltips_are_visible_by_default_and_can_be_toggled() {
        let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);

        assert!(chart.tooltips_visible());

        chart.update(TimeSeriesMessage::ToggleTooltips);
        assert!(!chart.tooltips_visible());

        chart.update(TimeSeriesMessage::ToggleTooltips);
        assert!(chart.tooltips_visible());
    }

    #[test]
    fn multiple_series_are_combined_into_one_tooltip_overlay() {
        let mut chart = multi_series_chart();
        chart.set_focus_index(Some(1));

        let values = chart.tooltip_values(1);

        assert_eq!(values.len(), 2);
        assert_eq!(values[0].label, "Primary");
        assert_eq!(values[0].value, "2.0");
        assert_eq!(values[1].label, "Reference");
        assert_eq!(values[1].value, "4.0");
        assert_eq!(chart.tooltip_overlays(Some(1.0)).len(), 1);
    }

    #[test]
    fn combined_tooltip_omits_series_without_a_focused_value() {
        let mut chart = multi_series_chart();
        chart.set_series_points(1, &[]);
        chart.set_focus_index(Some(1));

        let values = chart.tooltip_values(1);

        assert_eq!(values.len(), 1);
        assert_eq!(values[0].label, "Primary");
        assert_eq!(chart.tooltip_overlays(Some(1.0)).len(), 1);

        chart.set_series_points(0, &[]);
        chart.set_focus_index(Some(1));

        assert!(chart.tooltip_values(1).is_empty());
        assert!(chart.tooltip_overlays(Some(1.0)).is_empty());
    }

    #[test]
    fn toggling_tooltips_preserves_focus_and_cursor_dragging() {
        let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);
        chart.set_focus_index(Some(1));
        chart.update(TimeSeriesMessage::BeginCursorDrag);

        chart.update(TimeSeriesMessage::ToggleTooltips);

        assert_eq!(chart.focus_index(), Some(1));
        assert!(chart.is_cursor_dragging());
    }

    #[test]
    fn context_menu_freezes_the_clicked_data_position_and_suppresses_tooltips() {
        let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);
        chart.set_focus_index(Some(1));
        chart.track_cursor([1.0, 2.0]);

        chart.update(TimeSeriesMessage::OpenContextMenu(Point::new(80.0, 60.0)));

        assert!(chart.is_context_menu_open());
        assert_eq!(chart.context_menu_target(), Some([1.0, 2.0]));
        assert_eq!(
            chart.context.expect("open context").local_position,
            Point::new(80.0, 60.0)
        );
        assert_eq!(chart.focus_index(), Some(1));
        assert_eq!(chart.tooltip_focus_index(), None);

        chart.update(TimeSeriesMessage::CloseContextMenu);

        assert!(!chart.is_context_menu_open());
        assert_eq!(chart.tooltip_focus_index(), Some(1));
    }

    #[test]
    fn tooltip_action_closes_the_context_menu() {
        let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);
        chart.update(TimeSeriesMessage::OpenContextMenu(Point::new(80.0, 60.0)));

        chart.update(TimeSeriesMessage::ToggleTooltips);

        assert!(!chart.tooltips_visible());
        assert!(!chart.is_context_menu_open());
    }

    #[test]
    fn cursor_is_only_published_while_left_drag_is_active() {
        let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);

        assert_eq!(chart.track_cursor([2.0, 5.0]), None);
        assert_eq!(chart.cursor_position, Some([2.0, 5.0]));

        assert_eq!(chart.update(TimeSeriesMessage::BeginCursorDrag), Some(2.0));
        assert_eq!(chart.track_cursor([3.0, 6.0]), Some(3.0));

        assert_eq!(chart.update(TimeSeriesMessage::EndCursorDrag), None);
        assert_eq!(chart.track_cursor([4.0, 7.0]), None);
        assert_eq!(chart.cursor_position, Some([4.0, 7.0]));
    }

    #[test]
    fn cursor_drag_does_not_pan_the_x_axis() {
        let mut chart = test_chart(vec![[0.0, 1.0], [10.0, 2.0]]);
        set_link_view(
            &chart.x_axis_link,
            padded_x_limits(chart.x_limits),
            5.0,
            2.0,
        );

        chart.update(TimeSeriesMessage::BeginCursorDrag);
        chart.track_cursor([3.0, 1.0]);
        chart.track_cursor([4.0, 1.0]);

        assert_eq!(chart.x_axis_link.get().0, 5.0);
        assert_eq!(chart.x_axis_link.get().1, 2.0);
    }

    #[test]
    fn shift_scroll_pans_the_x_axis_without_changing_zoom() {
        let mut chart = test_chart(vec![[0.0, 1.0], [10.0, 2.0]]);
        set_link_view(
            &chart.x_axis_link,
            padded_x_limits(chart.x_limits),
            5.0,
            2.0,
        );

        chart.update(TimeSeriesMessage::PanX(mouse::ScrollDelta::Lines {
            x: 0.0,
            y: 1.0,
        }));

        assert_close(chart.x_axis_link.get().0, 4.6);
        assert_close(chart.x_axis_link.get().1, 2.0);

        chart.update(TimeSeriesMessage::PanX(mouse::ScrollDelta::Pixels {
            x: 0.0,
            y: -40.0,
        }));

        assert_close(chart.x_axis_link.get().0, 5.0);
        assert_close(chart.x_axis_link.get().1, 2.0);
    }

    #[test]
    fn scroll_modifiers_select_pan_zoom_or_parent_scrolling() {
        assert_eq!(scroll_action(keyboard::Modifiers::NONE), None);
        assert_eq!(
            scroll_action(keyboard::Modifiers::SHIFT),
            Some(ScrollAction::PanX)
        );
        assert_eq!(
            scroll_action(keyboard::Modifiers::CTRL),
            Some(ScrollAction::ZoomX)
        );
        assert_eq!(
            scroll_action(keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT),
            Some(ScrollAction::ZoomX)
        );
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
