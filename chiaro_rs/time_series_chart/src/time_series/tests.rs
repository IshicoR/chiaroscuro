use iced::Point;
use iced_plot::{Color, LineStyle, TickWeight};

use super::axis::set_link_view_at_end;
use super::interaction::{ScrollAction, scroll_action};
use super::*;

fn test_chart(points: Vec<[f64; 2]>) -> TimeSeriesChart {
    test_chart_with_link(points, AxisLink::new())
}

fn test_chart_with_link(points: Vec<[f64; 2]>, x_axis_link: AxisLink) -> TimeSeriesChart {
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
        x_axis_link,
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
fn charts_with_a_shared_axis_link_keep_the_same_x_view() {
    let link = AxisLink::new();
    let mut primary = test_chart_with_link(vec![[0.0, 1.0], [10.0, 2.0]], link.clone());
    let secondary = test_chart_with_link(vec![[0.0, 3.0], [10.0, 4.0]], link);

    primary.update(TimeSeriesMessage::ZoomX(mouse::ScrollDelta::Lines {
        x: 0.0,
        y: 1.0,
    }));
    assert_eq!(primary.axis.x_link.get(), secondary.axis.x_link.get());

    primary.update(TimeSeriesMessage::PanX(mouse::ScrollDelta::Lines {
        x: 0.0,
        y: -1.0,
    }));
    assert_eq!(primary.axis.x_link.get(), secondary.axis.x_link.get());

    primary.update(TimeSeriesMessage::ResetX);
    assert_eq!(primary.axis.x_link.get(), secondary.axis.x_link.get());
}

#[test]
fn repeatedly_clearing_an_empty_series_is_a_no_op() {
    let mut chart = test_chart(vec![[0.0, 0.0]]);

    chart.set_series_points(0, &[]);
    assert_eq!(chart.series.update_count, 1);

    chart.set_series_points(0, &[]);
    assert_eq!(chart.series.update_count, 1);

    chart.set_series_points(0, &[[0.0, 1.0]]);
    chart.set_series_points(0, &[]);
    assert_eq!(chart.series.update_count, 3);

    chart.set_series_points(0, &[]);
    assert_eq!(chart.series.update_count, 3);
}

#[test]
fn live_series_updates_only_append_and_trim_the_retained_window() {
    let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0], [2.0, 3.0]]);

    chart.update_live_series_points(0, 1.0, &[[3.0, 4.0]]);

    assert_eq!(chart.series_length(0), Some(3));
    assert_eq!(chart.series.update_count, 1);

    chart.update_live_series_points(0, 1.0, &[]);
    assert_eq!(chart.series.update_count, 1);

    chart.update_live_series_points(0, 2.5, &[]);
    assert_eq!(chart.series_length(0), Some(1));
    assert_eq!(chart.series.update_count, 2);
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
fn x_axis_formatter_scale_updates_without_changing_the_axis_label() {
    let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);

    chart.set_x_axis(
        AxisSpec::new("Lap distance", 0.0, 10_000.0, |meters| {
            format!("{meters:.0} m")
        })
        .with_formatter_scale(0.42),
    );

    assert_eq!(chart.axis.x_formatter_scale, 0.42);

    chart.set_x_axis(
        AxisSpec::new("Lap distance", 0.0, 10_000.0, |meters| {
            format!("{meters:.0} m")
        })
        .with_formatter_scale(0.428),
    );

    assert_eq!(chart.axis.x_formatter_scale, 0.428);
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

    let (center, half_extent, _) = chart.axis.x_link.get();
    assert_close(center - half_extent, 87.7);
    assert_close(center + half_extent, 100.3);
    assert_close(chart.x_axis_fraction(100.0), 1.025 / 1.05);
    assert_eq!(chart.axis.x_limits, (0.0, 100.0));

    chart.set_x_limits(0.0, 100.0);

    let (center, half_extent, _) = chart.axis.x_link.get();
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
fn sector_markers_are_visible_by_default_and_can_be_toggled() {
    let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);
    chart.set_markers(&[ChartMarker::new(
        0.5,
        "S1",
        Color::from_rgb(0.75, 0.42, 1.0),
    )]);

    assert!(chart.markers_visible());
    assert_eq!(chart.marker_line_overlays().len(), 1);
    assert_eq!(chart.marker_label_overlays().len(), 1);

    chart.update(TimeSeriesMessage::ToggleMarkers);

    assert!(!chart.markers_visible());
    assert!(chart.marker_line_overlays().is_empty());
    assert!(chart.marker_label_overlays().is_empty());
}

#[test]
fn sector_ranges_follow_the_shared_sector_visibility() {
    let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);
    chart.set_ranges(&[ChartRange::new(
        0.0,
        0.5,
        Color::from_rgba(0.75, 0.42, 1.0, 0.075),
    )]);

    assert_eq!(chart.ranges.len(), 1);
    assert!(chart.range_background_overlay().is_some());

    chart.update(TimeSeriesMessage::ToggleMarkers);
    chart.set_ranges(&[
        ChartRange::new(0.0, 0.5, Color::from_rgba(0.75, 0.42, 1.0, 0.075)),
        ChartRange::new(0.5, 1.0, Color::from_rgba(0.95, 0.76, 0.11, 0.055)),
    ]);

    assert!(chart.range_background_overlay().is_none());

    chart.update(TimeSeriesMessage::ToggleMarkers);

    assert!(chart.markers_visible());
    assert!(chart.range_background_overlay().is_some());
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
        chart
            .interaction
            .context
            .expect("open context")
            .local_position,
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
fn series_action_toggles_visibility_and_keeps_the_context_menu_open() {
    let mut chart = multi_series_chart();
    chart.update(TimeSeriesMessage::OpenContextMenu(Point::new(80.0, 60.0)));

    chart.update(TimeSeriesMessage::ToggleSeriesVisibility(1));

    assert!(!chart.series.visible[1]);
    assert!(chart.is_context_menu_open());
    assert_eq!(chart.tooltip_values(1).len(), 1);

    chart.update(TimeSeriesMessage::ToggleSeriesVisibility(1));

    assert!(chart.series.visible[1]);
    assert_eq!(chart.tooltip_values(1).len(), 2);
}

#[test]
fn unknown_series_visibility_action_is_ignored() {
    let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);

    chart.update(TimeSeriesMessage::ToggleSeriesVisibility(usize::MAX));

    assert_eq!(chart.series.visible, vec![true]);
}

#[test]
fn cursor_is_only_published_while_left_drag_is_active() {
    let mut chart = test_chart(vec![[0.0, 1.0], [1.0, 2.0]]);

    assert_eq!(chart.track_cursor([2.0, 5.0]), None);
    assert_eq!(chart.interaction.cursor_position, Some([2.0, 5.0]));

    assert_eq!(chart.update(TimeSeriesMessage::BeginCursorDrag), Some(2.0));
    assert_eq!(chart.track_cursor([3.0, 6.0]), Some(3.0));

    assert_eq!(chart.update(TimeSeriesMessage::EndCursorDrag), None);
    assert_eq!(chart.track_cursor([4.0, 7.0]), None);
    assert_eq!(chart.interaction.cursor_position, Some([4.0, 7.0]));
}

#[test]
fn cursor_drag_does_not_pan_the_x_axis() {
    let mut chart = test_chart(vec![[0.0, 1.0], [10.0, 2.0]]);
    set_link_view(
        &chart.axis.x_link,
        padded_x_limits(chart.axis.x_limits),
        5.0,
        2.0,
    );

    chart.update(TimeSeriesMessage::BeginCursorDrag);
    chart.track_cursor([3.0, 1.0]);
    chart.track_cursor([4.0, 1.0]);

    assert_eq!(chart.axis.x_link.get().0, 5.0);
    assert_eq!(chart.axis.x_link.get().1, 2.0);
}

#[test]
fn shift_scroll_pans_the_x_axis_without_changing_zoom() {
    let mut chart = test_chart(vec![[0.0, 1.0], [10.0, 2.0]]);
    set_link_view(
        &chart.axis.x_link,
        padded_x_limits(chart.axis.x_limits),
        5.0,
        2.0,
    );

    chart.update(TimeSeriesMessage::PanX(mouse::ScrollDelta::Lines {
        x: 0.0,
        y: 1.0,
    }));

    assert_close(chart.axis.x_link.get().0, 4.6);
    assert_close(chart.axis.x_link.get().1, 2.0);

    chart.update(TimeSeriesMessage::PanX(mouse::ScrollDelta::Pixels {
        x: 0.0,
        y: -40.0,
    }));

    assert_close(chart.axis.x_link.get().0, 5.0);
    assert_close(chart.axis.x_link.get().1, 2.0);
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
