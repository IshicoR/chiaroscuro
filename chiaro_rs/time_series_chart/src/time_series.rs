use std::fmt;

use iced::{keyboard, mouse};
use iced_plot::{
    AxisLink, MarkerStyle, PlotControls, PlotUiMessage, PlotWidget, PlotWidgetBuilder,
};

use super::style;

mod axis;
mod data;
mod interaction;
mod model;
mod view;

use axis::{AxisState, padded_x_limits, readable_ticks, set_link_view, set_y_link_view};
use data::SeriesState;
use interaction::InteractionState;
pub use interaction::TimeSeriesMessage;
pub use model::{AxisSpec, LineSeries, TimeSeriesSpec};

pub struct TimeSeriesChart {
    plot: PlotWidget,
    series: SeriesState,
    axis: AxisState,
    interaction: InteractionState,
}

impl fmt::Debug for TimeSeriesChart {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TimeSeriesChart")
            .field("plot", &"PlotWidget")
            .field("series_count", &self.series.ids.len())
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
            series: SeriesState {
                ids: series_ids,
                labels: series_labels,
                colors: series_colors,
                lengths: series_lengths,
                #[cfg(test)]
                update_count: 0,
            },
            axis: AxisState {
                value_formatter: y_axis.value_formatter,
                x_link: x_axis_link,
                y_link: y_axis_link,
                x_label: x_axis.label,
                x_limits,
                y_limits,
                live_mode: false,
            },
            interaction: InteractionState {
                focus_index: None,
                cursor_position: None,
                cursor_dragging: false,
                tooltips_visible: true,
                context: None,
            },
        }
    }
}

#[cfg(test)]
mod tests;
