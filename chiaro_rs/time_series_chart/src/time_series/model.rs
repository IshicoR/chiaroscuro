use iced_plot::{Color, LineStyle, Series};

/// A labeled vertical marker drawn over a time-series plot.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartMarker {
    pub(super) x: f64,
    pub(super) label: String,
    pub(super) color: Color,
}

impl ChartMarker {
    pub fn new(x: f64, label: impl Into<String>, color: Color) -> Self {
        Self {
            x,
            label: label.into(),
            color,
        }
    }
}

/// A colored vertical range drawn behind the plot series.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartRange {
    pub(super) start_x: f64,
    pub(super) end_x: f64,
    pub(super) color: Color,
}

impl ChartRange {
    pub fn new(start_x: f64, end_x: f64, color: Color) -> Self {
        Self {
            start_x,
            end_x,
            color,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AxisSpec {
    pub(super) label: &'static str,
    pub(super) min: f64,
    pub(super) max: f64,
    pub(super) formatter: fn(f64) -> String,
    pub(super) formatter_scale: f64,
    pub(super) value_formatter: fn(f64) -> String,
    pub(super) tick_count: usize,
}

impl AxisSpec {
    pub fn new(label: &'static str, min: f64, max: f64, formatter: fn(f64) -> String) -> Self {
        Self {
            label,
            min,
            max,
            formatter,
            formatter_scale: 1.0,
            value_formatter: formatter,
            tick_count: 6,
        }
    }

    pub const fn with_value_formatter(mut self, formatter: fn(f64) -> String) -> Self {
        self.value_formatter = formatter;
        self
    }

    pub const fn with_formatter_scale(mut self, scale: f64) -> Self {
        self.formatter_scale = scale;
        self
    }

    pub const fn with_tick_count(mut self, tick_count: usize) -> Self {
        self.tick_count = tick_count;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TimeSeriesSpec {
    pub(super) x_axis: AxisSpec,
    pub(super) y_axis: AxisSpec,
}

impl TimeSeriesSpec {
    pub fn new(x_axis: AxisSpec, y_axis: AxisSpec) -> Self {
        Self { x_axis, y_axis }
    }
}

#[derive(Debug, Clone)]
pub struct LineSeries {
    pub(super) points: Vec<[f64; 2]>,
    pub(super) label: &'static str,
    pub(super) color: Color,
    pub(super) style: LineStyle,
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

    pub(super) fn into_series(self) -> Series {
        Series::line_only(self.points, self.style)
            .with_label(self.label)
            .with_color(self.color)
    }
}
