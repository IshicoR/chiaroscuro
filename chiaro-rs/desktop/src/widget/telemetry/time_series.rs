use std::fmt;

use iced::Element;
use iced_plot::{
    Color, LineStyle, MarkerStyle, PlotUiMessage, PlotWidget, PlotWidgetBuilder, Series, ShapeId,
};

use super::style;

#[derive(Debug, Clone, Copy)]
pub struct AxisSpec {
    label: &'static str,
    min: f64,
    max: f64,
    formatter: fn(f64) -> String,
}

impl AxisSpec {
    pub fn new(label: &'static str, min: f64, max: f64, formatter: fn(f64) -> String) -> Self {
        Self {
            label,
            min,
            max,
            formatter,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TimeSeriesSpec {
    x_axis: AxisSpec,
    y_axis: AxisSpec,
    tooltip: fn(&str, f64, f64) -> String,
}

impl TimeSeriesSpec {
    pub fn new(x_axis: AxisSpec, y_axis: AxisSpec, tooltip: fn(&str, f64, f64) -> String) -> Self {
        Self {
            x_axis,
            y_axis,
            tooltip,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LineSeries {
    points: Vec<[f64; 2]>,
    label: &'static str,
    color: Color,
    style: LineStyle,
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
    pub fn new(spec: TimeSeriesSpec, series: impl IntoIterator<Item = LineSeries>) -> Self {
        let x_axis = spec.x_axis;
        let y_axis = spec.y_axis;
        let tooltip = spec.tooltip;
        let mut series_ids = Vec::new();
        let mut builder = PlotWidgetBuilder::new()
            .with_x_lim(x_axis.min, x_axis.max)
            .with_y_lim(y_axis.min, y_axis.max)
            .with_x_label(x_axis.label)
            .with_y_label(y_axis.label)
            .with_x_tick_formatter(move |tick| (x_axis.formatter)(tick.value))
            .with_y_tick_formatter(move |tick| (y_axis.formatter)(tick.value))
            .with_tick_label_size(11.0)
            .with_axis_label_size(13.0)
            .with_crosshairs(true)
            .with_cursor_overlay(false)
            .disable_controls_help()
            .with_style(style::plot)
            .with_hover_highlight_provider(move |context, point| {
                point.marker_style = Some(MarkerStyle::circle(5.0));
                Some(tooltip(context.series_label, point.x, point.y))
            });

        for series in series {
            let series = series.into_series();
            series_ids.push(series.id);
            builder = builder.add_series(series);
        }

        Self {
            plot: builder
                .build()
                .expect("time-series chart specification must be valid"),
            series_ids,
        }
    }

    pub fn update(&mut self, message: PlotUiMessage) {
        self.plot.update(message);
    }

    pub fn view(&self) -> Element<'_, PlotUiMessage> {
        self.plot.view()
    }

    pub fn set_series_points(&mut self, index: usize, points: &[[f64; 2]]) {
        if let Some(id) = self.series_ids.get(index) {
            self.plot.set_series_positions(id, points);
        }
    }
}
