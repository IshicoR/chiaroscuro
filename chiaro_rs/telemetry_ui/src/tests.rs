use std::time::{Duration, Instant};

use chiaro_actions::Action;
use chiaro_irsdk::{
    OptionalTelemetryValues, SessionInfo, SessionScalar, TelemetryFrame, TelemetrySample,
    TelemetryValue,
};
use chiaro_telemetry::{
    IbtInfo, LAP_DISTANCE_AXIS_MAX, LoadedIbt, RecordingSource, Session, TimedSample,
};
use chiaro_time_series_chart::TimeSeriesMessage;
use iced::{Point, Rectangle, Theme, keyboard, mouse, widget::rule};

use super::formatting::{format_chart_time, format_lap_distance, parse_track_length_meters};
use super::interaction::select_drop_target;
use super::metadata::{format_session_time, session_metadata};
use super::readout::{InputReadout, steering_meter_progress};
use super::{
    CONTENT_PADDING, CardLayout, ChartColumns, ChartId, DATA_ROW_HEIGHT, LapAnalysisCardId,
    LiveChartNavigation, SetupCardId, TelemetryLayout, TelemetryLayoutFlag, TelemetryMessage,
    TelemetryState, TelemetrySyncScope, activate, chart_sync_targets, data_separator_style,
    deactivate, focus_at, format_gear, format_lap_time, format_position, format_recording_duration,
    format_track_position, input_readout, metadata_value, move_item_to, pedal_percent, refresh,
    reset_reference, reset_session as reset_screen_telemetry, selected_comparison,
    should_build_chart_plot, symmetric_y_limits, sync_telemetry, update as update_telemetry,
    update_chart_focus,
};

#[test]
fn data_rows_use_only_a_thin_surface_separator() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let separator = data_separator_style(&theme);

    assert_eq!(separator.color, palette.background.weaker.color);
    assert_eq!(separator.fill_mode, rule::FillMode::Full);
    assert_eq!(separator.radius, 0.0.into());
}

#[test]
fn workspace_screens_use_consistent_content_padding() {
    assert_eq!(CONTENT_PADDING, 24.0);
}

#[test]
fn telemetry_state_starts_with_the_default_layout() {
    let state = TelemetryState::default();
    assert_eq!(state.chart_order, ChartId::DEFAULT_ORDER);
    assert!(state.chart_visibility[ChartId::Speed.index()]);
    assert!(state.chart_visibility[ChartId::Pedal.index()]);
    assert!(state.chart_visibility[ChartId::Steering.index()]);
    for chart in ChartId::ALL {
        assert_eq!(
            state.chart_visibility[chart.index()],
            ChartId::DEFAULT_VISIBLE.contains(&chart)
        );
    }
    assert_eq!(state.layout_snapshot(), TelemetryLayout::default());
}

#[test]
fn persisted_layout_keys_are_stable_and_round_trip() {
    for chart in ChartId::ALL {
        assert_eq!(ChartId::from_key(chart.key()), Some(chart));
    }
    for card in LapAnalysisCardId::ALL {
        assert_eq!(LapAnalysisCardId::from_key(card.key()), Some(card));
    }
    for card in SetupCardId::ALL {
        assert_eq!(SetupCardId::from_key(card.key()), Some(card));
    }
    assert_eq!(ChartId::from_key("future_chart"), None);
    assert_eq!(LapAnalysisCardId::from_key("future_card"), None);
    assert_eq!(SetupCardId::from_key("future_card"), None);
}

#[test]
fn imported_layout_normalizes_unknown_duplicate_and_missing_values() {
    let mut state = TelemetryState {
        layout_revision: 41,
        maximized_chart: Some(ChartId::Speed),
        dragging_chart: Some(ChartId::Speed),
        dragging_lap_analysis_card: Some(LapAnalysisCardId::Cursor),
        dragging_setup_card: Some(SetupCardId::Session),
        ..TelemetryState::default()
    };
    state.chart_layouts[ChartId::Speed.index()] = Some(CardLayout {
        bounds: Rectangle::default(),
        visible_bounds: Some(Rectangle::default()),
    });
    state.chart_list_layouts[ChartId::Speed.index()] = state.chart_layouts[ChartId::Speed.index()];
    state.lap_analysis_layouts[LapAnalysisCardId::Cursor.index()] =
        state.chart_layouts[ChartId::Speed.index()];
    state.setup_card_layouts[SetupCardId::Session.index()] =
        state.chart_layouts[ChartId::Speed.index()];

    let layout = TelemetryLayout {
        chart_order: ["fuel", "future_chart", "fuel", "speed"]
            .map(str::to_owned)
            .to_vec(),
        chart_visibility: vec![
            TelemetryLayoutFlag {
                key: "future_chart".to_owned(),
                value: true,
            },
            TelemetryLayoutFlag {
                key: "speed".to_owned(),
                value: false,
            },
            TelemetryLayoutFlag {
                key: "speed".to_owned(),
                value: true,
            },
            TelemetryLayoutFlag {
                key: "abs".to_owned(),
                value: true,
            },
        ],
        chart_collapsed: vec![
            TelemetryLayoutFlag {
                key: "fuel".to_owned(),
                value: true,
            },
            TelemetryLayoutFlag {
                key: "fuel".to_owned(),
                value: false,
            },
        ],
        chart_columns: 99,
        setup_card_order: ["charts", "future_card", "charts"]
            .map(str::to_owned)
            .to_vec(),
        setup_card_collapsed: vec![TelemetryLayoutFlag {
            key: "reference".to_owned(),
            value: true,
        }],
        lap_analysis_order: ["wheels", "cursor", "wheels"].map(str::to_owned).to_vec(),
        lap_analysis_collapsed: vec![TelemetryLayoutFlag {
            key: "inputs".to_owned(),
            value: true,
        }],
    };

    state.apply_layout(&layout);

    assert_eq!(state.layout_revision(), 41);
    assert_eq!(
        &state.chart_order[..4],
        &[
            ChartId::Fuel,
            ChartId::Speed,
            ChartId::Delta,
            ChartId::Pedal
        ]
    );
    assert_eq!(state.chart_order.len(), ChartId::COUNT);
    assert!(!state.chart_visibility[ChartId::Speed.index()]);
    assert!(state.chart_visibility[ChartId::Abs.index()]);
    assert!(state.chart_visibility[ChartId::Pedal.index()]);
    assert!(state.chart_visibility[ChartId::Steering.index()]);
    assert!(!state.chart_visibility[ChartId::Fuel.index()]);
    assert!(state.chart_collapsed[ChartId::Fuel.index()]);
    assert_eq!(state.chart_columns, ChartColumns::One);
    assert_eq!(
        state.setup_card_order,
        [
            SetupCardId::Charts,
            SetupCardId::Session,
            SetupCardId::Reference,
            SetupCardId::Laps,
        ]
    );
    assert!(state.setup_card_collapsed[SetupCardId::Reference.index()]);
    assert_eq!(
        state.lap_analysis_order,
        [
            LapAnalysisCardId::Wheels,
            LapAnalysisCardId::Cursor,
            LapAnalysisCardId::ReferenceCursor,
            LapAnalysisCardId::Vehicle,
            LapAnalysisCardId::Inputs,
            LapAnalysisCardId::Dynamics,
            LapAnalysisCardId::Tyres,
        ]
    );
    assert!(state.lap_analysis_collapsed[LapAnalysisCardId::Inputs.index()]);
    assert_eq!(state.maximized_chart, None);
    assert!(!state.is_dragging_card());
    assert!(state.chart_layouts.iter().all(Option::is_none));
    assert!(state.chart_list_layouts.iter().all(Option::is_none));
    assert!(state.lap_analysis_layouts.iter().all(Option::is_none));
    assert!(state.setup_card_layouts.iter().all(Option::is_none));
}

#[test]
fn layout_revision_changes_only_for_persisted_edits_and_explicit_reset() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let default_layout = state.layout_snapshot();

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChart(ChartId::Speed, true),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::SetChartColumns(ChartColumns::One),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::BeginChartDrag(ChartId::Speed),
    );
    update(&mut state, &session, TelemetryMessage::FinishCardDrag);
    state.apply_layout(&default_layout);
    assert_eq!(state.layout_revision(), 0);

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChart(ChartId::Speed, false),
    );
    assert_eq!(state.layout_revision(), 1);
    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChart(ChartId::Speed, false),
    );
    assert_eq!(state.layout_revision(), 1);
    update(
        &mut state,
        &session,
        TelemetryMessage::SetChartColumns(ChartColumns::Two),
    );
    assert_eq!(state.layout_revision(), 2);
    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleLapAnalysisCardCollapsed(LapAnalysisCardId::Cursor),
    );
    assert_eq!(state.layout_revision(), 3);
    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleSetupCardCollapsed(SetupCardId::Session),
    );
    assert_eq!(state.layout_revision(), 4);

    assert!(TelemetryMessage::ResetTelemetryLayout.resets_layout());
    assert!(!TelemetryMessage::ToggleConnection.resets_layout());
    update(&mut state, &session, TelemetryMessage::ResetTelemetryLayout);
    assert_eq!(state.layout_revision(), 5);
    assert_eq!(state.layout_snapshot(), TelemetryLayout::default());
    update(&mut state, &session, TelemetryMessage::ResetTelemetryLayout);
    assert_eq!(state.layout_revision(), 6);
}

#[test]
fn optional_vehicle_channels_feed_their_charts_without_zero_fallbacks() {
    let started_at = Instant::now();
    let mut session = Session::default();
    for (offset, pressure, abs, torque) in [
        (0, [80.0, 82.0, 48.0, 50.0], false, -4.0),
        (1, [90.0, 91.0, 54.0, 55.0], true, 6.0),
    ] {
        session.record_sample_at(
            started_at + Duration::from_millis(offset * 16),
            TelemetrySample {
                brake_line_pressure_bar: OptionalTelemetryValues::from_options(pressure.map(Some)),
                abs_active: Some(abs),
                steering_wheel_torque_nm: OptionalTelemetryValues::from_options([Some(torque)]),
                ..TelemetrySample::default()
            },
        );
    }

    let mut state = TelemetryState::default();
    sync_telemetry(&mut state, &session, None);

    for series in 0..4 {
        assert_eq!(state.brake_pressure_chart.series_length(series), Some(2));
    }
    assert_eq!(state.abs_chart.series_length(0), Some(2));
    assert_eq!(state.steering_torque_chart.series_length(0), Some(2));

    let mut unavailable = Session::default();
    unavailable.record_sample(TelemetrySample::default());
    sync_telemetry(&mut state, &unavailable, None);
    assert_eq!(state.brake_pressure_chart.series_length(0), Some(0));
    assert_eq!(state.abs_chart.series_length(0), Some(0));
    assert_eq!(state.steering_torque_chart.series_length(0), Some(0));
}

#[test]
fn deactivating_telemetry_cancels_dragging_and_defers_hidden_chart_updates() {
    let started_at = Instant::now();
    let mut session = Session::default();
    session.record_sample_at(
        started_at,
        TelemetrySample {
            speed_kmh: 100.0,
            ..TelemetrySample::default()
        },
    );
    let mut state = TelemetryState::default();
    sync_telemetry(&mut state, &session, None);
    assert_eq!(state.speed_chart.series_length(0), Some(1));
    state.dragging_chart = Some(ChartId::Speed);
    state.modifiers = keyboard::Modifiers::SHIFT;

    deactivate(&mut state);

    assert_eq!(state.dragging_chart, None);
    assert_eq!(state.modifiers, keyboard::Modifiers::NONE);
    session.record_sample_at(
        started_at + Duration::from_millis(16),
        TelemetrySample {
            speed_kmh: 110.0,
            ..TelemetrySample::default()
        },
    );
    session.record_sample_at(
        started_at + Duration::from_millis(32),
        TelemetrySample {
            speed_kmh: 120.0,
            ..TelemetrySample::default()
        },
    );
    assert_eq!(state.rendered_packets, 1);
    assert_eq!(state.speed_chart.series_length(0), Some(1));

    activate(&mut state, &session, None);
    assert_eq!(state.rendered_packets, 3);
    assert_eq!(state.speed_chart.series_length(0), Some(3));
}

#[test]
fn resetting_a_session_while_hidden_defers_the_full_chart_rebuild() {
    let mut session = Session::default();
    session.record_sample(TelemetrySample {
        speed_kmh: 144.0,
        ..TelemetrySample::default()
    });
    let mut state = TelemetryState::default();
    deactivate(&mut state);
    state.rendered_packets = 99;
    let hidden_series_length = state.speed_chart.series_length(0);

    reset_screen_telemetry(&mut state, &session, None, false);

    assert_eq!(state.rendered_packets, 99);
    assert_eq!(state.speed_chart.series_length(0), hidden_series_length);

    activate(&mut state, &session, None);
    assert_eq!(state.rendered_packets, 1);
    assert_eq!(state.speed_chart.series_length(0), Some(1));
}

#[test]
fn missing_or_blank_session_metadata_keeps_a_placeholder_value() {
    assert_eq!(metadata_value(None), "--");
    assert_eq!(metadata_value(Some(String::new())), "--");
    assert_eq!(metadata_value(Some("  ".to_owned())), "--");
    assert_eq!(metadata_value(Some("Dynamic".to_owned())), "Dynamic");
}

fn update(
    state: &mut TelemetryState,
    session: &Session,
    message: TelemetryMessage,
) -> Option<Action> {
    update_telemetry(state, session, None, message)
}

fn chart_bounds(x: f32, y: f32) -> Rectangle {
    Rectangle {
        x,
        y,
        width: 100.0,
        height: 100.0,
    }
}

fn report_chart_layout(
    state: &mut TelemetryState,
    session: &Session,
    chart: ChartId,
    bounds: Rectangle,
) {
    update(
        state,
        session,
        TelemetryMessage::ChartLayoutChanged {
            chart,
            bounds,
            visible_bounds: Some(bounds),
        },
    );
}

fn begin_chart_drag(state: &mut TelemetryState, session: &Session, chart: ChartId, origin: Point) {
    update(state, session, TelemetryMessage::BeginChartDrag(chart));
    update(state, session, TelemetryMessage::DragCursor(origin));
}

fn report_chart_list_layout(
    state: &mut TelemetryState,
    session: &Session,
    chart: ChartId,
    bounds: Rectangle,
) {
    update(
        state,
        session,
        TelemetryMessage::ChartListLayoutChanged {
            chart,
            bounds,
            visible_bounds: Some(bounds),
        },
    );
}

fn begin_chart_list_drag(
    state: &mut TelemetryState,
    session: &Session,
    chart: ChartId,
    origin: Point,
) {
    update(state, session, TelemetryMessage::BeginChartListDrag(chart));
    update(state, session, TelemetryMessage::DragCursor(origin));
}

fn report_lap_analysis_layout(
    state: &mut TelemetryState,
    session: &Session,
    card: LapAnalysisCardId,
    bounds: Rectangle,
) {
    update(
        state,
        session,
        TelemetryMessage::LapAnalysisLayoutChanged {
            card,
            bounds,
            visible_bounds: Some(bounds),
        },
    );
}

fn begin_lap_analysis_drag(
    state: &mut TelemetryState,
    session: &Session,
    card: LapAnalysisCardId,
    origin: Point,
) {
    update(state, session, TelemetryMessage::BeginLapAnalysisDrag(card));
    update(state, session, TelemetryMessage::DragCursor(origin));
}

fn report_setup_card_layout(
    state: &mut TelemetryState,
    session: &Session,
    card: SetupCardId,
    bounds: Rectangle,
) {
    update(
        state,
        session,
        TelemetryMessage::SetupCardLayoutChanged {
            card,
            bounds,
            visible_bounds: Some(bounds),
        },
    );
}

fn begin_setup_card_drag(
    state: &mut TelemetryState,
    session: &Session,
    card: SetupCardId,
    origin: Point,
) {
    update(state, session, TelemetryMessage::BeginSetupCardDrag(card));
    update(state, session, TelemetryMessage::DragCursor(origin));
}

fn reset_telemetry(state: &mut TelemetryState, session: &Session) {
    reset_screen_telemetry(state, session, None, true);
}

fn loaded_test_session(track_name: &str, file_name: &str, speed_kmh: f32) -> Session {
    let sample =
        |elapsed_seconds, completed_laps, current_lap_ms, last_lap_ms, normalized_car_position| {
            TimedSample {
                elapsed_seconds,
                sample: TelemetrySample {
                    completed_laps,
                    current_lap_ms,
                    last_lap_ms,
                    normalized_car_position,
                    speed_kmh,
                    ..TelemetrySample::default()
                },
            }
        };
    let frame = TelemetryFrame::try_new(
        4,
        Vec::<chiaro_irsdk::VariableMetadata>::new(),
        Vec::<TelemetryValue>::new(),
    )
    .expect("valid empty frame");
    let mut session = Session::default();
    session.load_ibt(LoadedIbt {
        info: IbtInfo {
            source: RecordingSource::local_file(file_name),
            file_name: file_name.to_owned(),
            track_name: track_name.to_owned(),
            track_id: None,
            track_config_name: None,
            car_id: None,
            car_name: None,
            duration_seconds: 21.0,
            lap_count: 1,
            record_count: 4,
            tick_rate: 60,
        },
        samples: vec![
            sample(0.0, 0, 0, 0, 0.0),
            sample(10.0, 0, 10_000, 0, 0.5),
            sample(20.0, 0, 20_000, 0, 0.95),
            sample(21.0, 1, 0, 20_000, 0.0),
        ],
        latest_frame: frame,
        session_info: SessionInfo {
            update_count: 1,
            yaml: String::new(),
            raw: Vec::new(),
        },
    });
    session
}

#[test]
fn extracts_current_session_vehicle_course_and_conditions() {
    let yaml = r#"
WeekendInfo:
 TrackDisplayName: Test Circuit
 TrackConfigName: Grand Prix
 TrackLength: 4.20 km
 TrackNumTurns: 12
 TrackType: road course
 TrackDirection: clockwise
 TrackSkies: Clear
 TrackAirTemp: 24.0 C
 TrackSurfaceTemp: 38.0 C
 TrackRelativeHumidity: 55 %
 WeekendOptions:
  Date: 2026-07-16
  TimeOfDay: 3:30 pm
  WindSpeed: 12 km/h
  WindDirection: NW
SessionInfo:
 CurrentSessionNum: 1
 Sessions:
 - SessionNum: 0
   SessionType: Practice
 - SessionNum: 1
   SessionType: Race
   SessionTime: 3600.0 sec
   SessionTrackRubberState: high usage
DriverInfo:
 DriverCarIdx: 2
 Drivers:
 - CarIdx: 1
   CarScreenName: Other Car
 - CarIdx: 2
   CarScreenName: Test GT3
   CarClassShortName: GT3
"#;
    let document = SessionInfo {
        update_count: 1,
        yaml: yaml.to_owned(),
        raw: yaml.as_bytes().to_vec(),
    }
    .parse()
    .expect("valid session metadata");

    let metadata = session_metadata(&document);

    assert_eq!(metadata.track_name.as_deref(), Some("Test Circuit"));
    assert_eq!(metadata.track_config.as_deref(), Some("Grand Prix"));
    assert_eq!(metadata.track_length.as_deref(), Some("4.20 km"));
    assert_eq!(metadata.track_turns, Some(12));
    assert_eq!(
        metadata.track_type.as_deref(),
        Some("road course · clockwise")
    );
    assert_eq!(metadata.car_name.as_deref(), Some("Test GT3"));
    assert_eq!(metadata.car_class.as_deref(), Some("GT3"));
    assert_eq!(metadata.session_type.as_deref(), Some("Race"));
    assert_eq!(metadata.session_time.as_deref(), Some("1:00:00"));
    assert_eq!(metadata.date_time.as_deref(), Some("2026-07-16 · 3:30 pm"));
    assert_eq!(metadata.weather.as_deref(), Some("Clear"));
    assert_eq!(metadata.air_temperature.as_deref(), Some("24.0 °C"));
    assert_eq!(metadata.surface_temperature.as_deref(), Some("38.0 °C"));
    assert_eq!(metadata.humidity.as_deref(), Some("55 %"));
    assert_eq!(metadata.wind.as_deref(), Some("12 km/h · NW"));
}

#[test]
fn formats_configured_session_time_for_display() {
    assert_eq!(format_session_time(&SessionScalar::Float(90.0)), "1:30");
    assert_eq!(
        format_session_time(&SessionScalar::String("unlimited".to_owned())),
        "Unlimited"
    );
}

#[test]
fn formats_irsdk_gears() {
    assert_eq!(format_gear(-1), "R");
    assert_eq!(format_gear(0), "N");
    assert_eq!(format_gear(5), "5");
}

#[test]
fn formats_lap_times() {
    assert_eq!(format_lap_time(91_234), "1:31.234");
    assert_eq!(format_lap_time(0), "--:--.---");
}

#[test]
fn formats_recording_times_without_sixty_second_rollover() {
    assert_eq!(format_chart_time(5.2), "5s");
    assert_eq!(format_chart_time(65.0), "1:05");
    assert_eq!(format_chart_time(3_599.9), "1:00:00");
    assert_eq!(format_recording_duration(3_661.0), "1:01:01");
}

#[test]
fn formats_lap_distance_axis_units_as_meters() {
    assert_eq!(format_lap_distance(-0.0), "0 m");
    assert_eq!(format_lap_distance(0.0), "0 m");
    assert_eq!(format_lap_distance(840.0), "840 m");
    assert_eq!(format_lap_distance(4_200.0), "4200 m");
}

#[test]
fn parses_metric_track_lengths_for_the_lap_distance_axis() {
    assert_eq!(parse_track_length_meters("4.20 km"), Some(4_200.0));
    assert_eq!(parse_track_length_meters("4200 m"), Some(4_200.0));
    assert_eq!(parse_track_length_meters("4,28 km"), Some(4_280.0));
    assert_eq!(parse_track_length_meters("unknown"), None);
}

#[test]
fn formats_race_position() {
    assert_eq!(format_position(3), "P3");
    assert_eq!(format_position(0), "—");
}

#[test]
fn formats_normalized_track_position() {
    assert_eq!(format_track_position(0.425, 4_200.0), "1785 m");
    assert_eq!(format_track_position(-1.0, 4_200.0), "—");
}

#[test]
fn clamps_pedal_values_to_a_percentage() {
    assert_eq!(pedal_percent(-0.2), 0.0);
    assert_eq!(pedal_percent(0.425), 42.5);
    assert_eq!(pedal_percent(1.2), 100.0);
}

#[test]
fn input_readout_is_unavailable_without_a_sample() {
    let readout = input_readout(None, None);

    assert_eq!(readout.throttle, None);
    assert_eq!(readout.brake, None);
    assert_eq!(readout.steering, None);
}

#[test]
fn input_readout_formats_the_current_sample() {
    let readout = input_readout(
        Some(TelemetrySample {
            throttle: 0.425,
            brake: 0.059,
            steering_angle: (-90.0_f32).to_radians(),
            ..TelemetrySample::default()
        }),
        Some(180.0_f32.to_radians()),
    );
    let throttle = readout.throttle.expect("finite throttle");
    let brake = readout.brake.expect("finite brake");
    let steering = readout.steering.expect("finite steering");

    assert_eq!(throttle.text, "42.5%");
    assert_eq!(throttle.progress, Some(0.425));
    assert_eq!(brake.text, "5.9%");
    assert_eq!(brake.progress, Some(0.059));
    assert_eq!(steering.text, "-90.0°");
    assert_eq!(steering.progress, Some(0.5));
}

#[test]
fn input_readout_clamps_pedal_text_and_progress_together() {
    let readout = input_readout(
        Some(TelemetrySample {
            throttle: -0.2,
            brake: 1.2,
            ..TelemetrySample::default()
        }),
        None,
    );
    let throttle = readout.throttle.expect("throttle");
    let brake = readout.brake.expect("brake");

    assert_eq!(throttle.text, "0.0%");
    assert_eq!(throttle.progress, Some(0.0));
    assert_eq!(brake.text, "100.0%");
    assert_eq!(brake.progress, Some(1.0));
}

#[test]
fn input_readout_rejects_non_finite_values() {
    let readout = input_readout(
        Some(TelemetrySample {
            throttle: f32::NAN,
            brake: f32::INFINITY,
            steering_angle: f32::NEG_INFINITY,
            ..TelemetrySample::default()
        }),
        Some(1.0),
    );

    assert_eq!(readout, InputReadout::default());
}

#[test]
fn steering_meter_progress_uses_the_sdk_limit_and_matches_screen_direction() {
    let maximum = 180.0_f32.to_radians();

    assert_eq!(steering_meter_progress(-maximum, maximum), Some(1.0));
    assert_eq!(steering_meter_progress(-maximum / 2.0, maximum), Some(0.5));
    assert_eq!(steering_meter_progress(0.0, maximum), Some(-0.0));
    assert_eq!(steering_meter_progress(maximum / 2.0, maximum), Some(-0.5));
    assert_eq!(steering_meter_progress(maximum * 2.0, maximum), Some(-1.0));
    assert_eq!(steering_meter_progress(0.5, 0.0), None);
    assert_eq!(steering_meter_progress(0.5, f32::NAN), None);
}

#[test]
fn steering_meter_caps_large_sdk_ranges_for_a_more_responsive_display() {
    let sdk_maximum = 450.0_f32.to_radians();
    let right_angle = -90.0_f32.to_radians();
    let left_angle = 90.0_f32.to_radians();

    assert_eq!(steering_meter_progress(right_angle, sdk_maximum), Some(0.5));
    assert_eq!(steering_meter_progress(left_angle, sdk_maximum), Some(-0.5));
}

#[test]
fn steering_text_remains_available_without_an_sdk_limit() {
    let readout = input_readout(
        Some(TelemetrySample {
            steering_angle: 12.5_f32.to_radians(),
            ..TelemetrySample::default()
        }),
        None,
    );
    let steering = readout.steering.expect("finite steering angle");

    assert_eq!(steering.text, "12.5°");
    assert_eq!(steering.progress, None);
}

#[test]
fn expands_symmetric_y_limits_to_fit_outlying_values() {
    assert_eq!(
        symmetric_y_limits([1.0, -2.0].into_iter(), 3.0),
        (-3.0, 3.0)
    );

    let expanded = symmetric_y_limits([2.0, -4.0].into_iter(), 3.0);
    assert!((expanded.0 + 4.8).abs() < 1e-9);
    assert!((expanded.1 - 4.8).abs() < 1e-9);
}

#[test]
fn refreshes_charts_only_after_new_packets() {
    let mut state = TelemetryState::default();
    let mut session = Session::default();
    session.record_sample(TelemetrySample::default());

    refresh(&mut state, &session, None);

    assert_eq!(state.rendered_packets, 1);
    assert_eq!(state.speed_chart.x_axis_label(), "Time");
}

#[test]
fn refresh_is_deferred_while_any_telemetry_card_is_dragged() {
    let begin_messages = [
        TelemetryMessage::BeginChartDrag(ChartId::Speed),
        TelemetryMessage::BeginChartListDrag(ChartId::Speed),
        TelemetryMessage::BeginLapAnalysisDrag(LapAnalysisCardId::Cursor),
        TelemetryMessage::BeginSetupCardDrag(SetupCardId::Session),
    ];

    for begin in begin_messages {
        let mut state = TelemetryState::default();
        let mut session = Session::default();
        session.record_sample(TelemetrySample::default());

        update(&mut state, &session, begin);
        refresh(&mut state, &session, None);

        assert_eq!(state.rendered_packets, 0);

        update(&mut state, &session, TelemetryMessage::FinishCardDrag);
        assert_eq!(state.rendered_packets, 0);

        refresh(&mut state, &session, None);
        assert_eq!(state.rendered_packets, 1);
    }
}

#[test]
fn collapsed_and_dragged_chart_cards_skip_plot_construction() {
    let mut state = TelemetryState::default();
    let session = Session::default();

    assert!(should_build_chart_plot(&state, ChartId::Speed));

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChartCollapsed(ChartId::Speed),
    );
    assert!(!should_build_chart_plot(&state, ChartId::Speed));

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChartCollapsed(ChartId::Speed),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::BeginChartDrag(ChartId::Speed),
    );
    assert!(!should_build_chart_plot(&state, ChartId::Speed));
    assert!(should_build_chart_plot(&state, ChartId::Pedal));
}

#[test]
fn live_sync_targets_only_visible_expanded_charts() {
    let mut state = TelemetryState::default();
    let bounds = chart_bounds(0.0, 0.0);
    let initial_targets = chart_sync_targets(&state, true, TelemetrySyncScope::LiveVisible);
    assert!(initial_targets.into_iter().all(|target| !target));

    for chart in ChartId::ALL {
        state.chart_layouts[chart.index()] = Some(CardLayout {
            bounds,
            visible_bounds: None,
        });
    }
    state.chart_layouts[ChartId::Speed.index()] = Some(CardLayout {
        bounds,
        visible_bounds: Some(bounds),
    });

    let targets = chart_sync_targets(&state, true, TelemetrySyncScope::LiveVisible);
    assert!(targets[ChartId::Speed.index()]);
    assert!(!targets[ChartId::Pedal.index()]);
    let ibt_targets = chart_sync_targets(&state, false, TelemetrySyncScope::LiveVisible);
    assert!(ibt_targets.into_iter().all(|target| target));

    state.chart_collapsed[ChartId::Speed.index()] = true;
    let targets = chart_sync_targets(&state, true, TelemetrySyncScope::LiveVisible);
    assert!(!targets[ChartId::Speed.index()]);

    state.chart_collapsed[ChartId::Speed.index()] = false;
    state.chart_layouts[ChartId::Pedal.index()] = Some(CardLayout {
        bounds,
        visible_bounds: Some(bounds),
    });
    state.maximized_chart = Some(ChartId::Pedal);
    let targets = chart_sync_targets(&state, true, TelemetrySyncScope::LiveVisible);
    assert!(targets[ChartId::Pedal.index()]);
    assert!(!targets[ChartId::Speed.index()]);

    state.chart_visibility[ChartId::Pedal.index()] = false;
    let targets = chart_sync_targets(&state, true, TelemetrySyncScope::LiveVisible);
    assert!(!targets[ChartId::Pedal.index()]);
}

#[test]
fn live_refresh_skips_offscreen_charts_but_full_sync_keeps_reset_correct() {
    let mut state = TelemetryState::default();
    let mut session = Session::default();
    let bounds = chart_bounds(0.0, 0.0);
    for chart in ChartId::ALL {
        state.chart_layouts[chart.index()] = Some(CardLayout {
            bounds,
            visible_bounds: None,
        });
    }
    state.chart_layouts[ChartId::Speed.index()] = Some(CardLayout {
        bounds,
        visible_bounds: Some(bounds),
    });
    session.record_sample(TelemetrySample::default());
    session.record_sample(TelemetrySample::default());

    refresh(&mut state, &session, None);

    assert_eq!(state.speed_chart.series_length(0), Some(2));
    assert_eq!(state.speed_chart.series_length(1), Some(0));
    assert_eq!(state.pedal_chart.series_length(0), Some(1));
    assert_eq!(state.chart_packet_cursors[ChartId::Speed.index()], Some(2));
    assert_eq!(state.chart_packet_cursors[ChartId::Pedal.index()], None);
    assert_eq!(state.rendered_packets, 2);

    state.chart_layouts[ChartId::Pedal.index()] = Some(CardLayout {
        bounds,
        visible_bounds: Some(bounds),
    });
    refresh(&mut state, &session, None);

    assert_eq!(state.pedal_chart.series_length(0), Some(2));
    assert_eq!(state.chart_packet_cursors[ChartId::Pedal.index()], Some(2));

    session.record_sample(TelemetrySample::default());
    refresh(&mut state, &session, None);

    assert_eq!(state.speed_chart.series_length(0), Some(3));
    assert_eq!(state.pedal_chart.series_length(0), Some(3));
    assert_eq!(state.rpm_chart.series_length(0), Some(1));

    sync_telemetry(&mut state, &session, None);

    assert_eq!(state.rpm_chart.series_length(0), Some(3));
    assert_eq!(state.pedal_chart.series_length(2), Some(0));
    assert!(
        state
            .chart_packet_cursors
            .into_iter()
            .all(|cursor| cursor == Some(3))
    );
}

#[test]
fn incremental_live_refresh_trims_points_before_the_retained_history() {
    let mut state = TelemetryState::default();
    let mut session = Session::default();
    let bounds = chart_bounds(0.0, 0.0);
    state.chart_layouts[ChartId::Speed.index()] = Some(CardLayout {
        bounds,
        visible_bounds: Some(bounds),
    });
    let started_at = Instant::now();
    session.record_sample_at(started_at, TelemetrySample::default());
    session.record_sample_at(
        started_at + Duration::from_secs(1),
        TelemetrySample::default(),
    );

    refresh(&mut state, &session, None);
    assert_eq!(state.speed_chart.series_length(0), Some(2));

    session.record_sample_at(
        started_at + Duration::from_secs(25),
        TelemetrySample::default(),
    );
    refresh(&mut state, &session, None);

    assert_eq!(state.speed_chart.series_length(0), Some(1));
    assert_eq!(state.chart_packet_cursors[ChartId::Speed.index()], Some(3));
}

#[test]
fn full_ibt_sync_resets_every_live_chart_cursor() {
    let mut state = TelemetryState::default();
    let session = loaded_test_session("Test Circuit", "test.ibt", 120.0);

    sync_telemetry(&mut state, &session, None);

    assert!(
        state
            .chart_packet_cursors
            .into_iter()
            .all(|cursor| cursor.is_none())
    );
}

#[test]
fn live_chart_browsing_pauses_and_reset_restores_latest_following() {
    let mut state = TelemetryState::default();
    let session = Session::default();

    let _action = update(
        &mut state,
        &session,
        TelemetryMessage::SpeedPlot(TimeSeriesMessage::PanX(mouse::ScrollDelta::Lines {
            x: 0.0,
            y: -1.0,
        })),
    );
    assert!(!state.live_follow);

    let _action = update(
        &mut state,
        &session,
        TelemetryMessage::SpeedPlot(TimeSeriesMessage::ResetX),
    );
    assert!(state.live_follow);
}

#[test]
fn tooltip_visibility_is_independent_for_each_chart() {
    let mut state = TelemetryState::default();
    let session = Session::default();

    update(
        &mut state,
        &session,
        TelemetryMessage::SpeedPlot(TimeSeriesMessage::ToggleTooltips),
    );

    assert!(!state.speed_chart.tooltips_visible());
    assert!(state.pedal_chart.tooltips_visible());
}

#[test]
fn live_chart_ignores_hover_time_and_keeps_focus_on_the_latest_sample() {
    let mut state = TelemetryState::default();
    let mut session = Session::default();
    session.record_sample(TelemetrySample::default());

    update_chart_focus(&mut state, &session, Some(999.0), LiveChartNavigation::None);

    assert_eq!(state.focus_x, Some(0.0));
    assert!(!state.focus_from_cursor);
}

#[test]
fn requests_the_ibt_file_picker() {
    let mut state = TelemetryState::default();
    let session = Session::default();

    let action = update(&mut state, &session, TelemetryMessage::OpenIbt);

    assert_eq!(action, Some(Action::OpenIbt));
}

#[test]
fn loads_and_selects_a_reference_lap_without_replacing_the_main_session() {
    let session = loaded_test_session("Test Circuit", "main.ibt", 100.0);
    let reference = loaded_test_session("Test Circuit", "reference.ibt", 110.0);
    let mut state = TelemetryState::default();

    let action = update_telemetry(
        &mut state,
        &session,
        Some(&reference),
        TelemetryMessage::OpenReferenceIbt,
    );
    assert_eq!(action, Some(Action::OpenReferenceIbt));

    reset_screen_telemetry(&mut state, &session, Some(&reference), true);
    reset_reference(&mut state, &session, Some(&reference), true);

    assert_eq!(state.reference_lap_choices.len(), 2);
    assert_eq!(state.selected_reference_lap_index, Some(0));
    assert!(selected_comparison(&state, &session, Some(&reference)).is_some());

    let different_track = loaded_test_session("Other Circuit", "other.ibt", 110.0);
    reset_reference(&mut state, &session, Some(&different_track), true);
    assert!(selected_comparison(&state, &session, Some(&different_track)).is_none());
}

#[test]
fn overlapping_card_reorders_even_when_cursor_is_outside_target() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let source = chart_bounds(0.0, 0.0);
    let target = chart_bounds(0.0, 116.0);

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChart(ChartId::Fuel, false),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChart(ChartId::Gear, true),
    );
    assert!(!state.chart_visibility[ChartId::Fuel.index()]);
    report_chart_layout(&mut state, &session, ChartId::Speed, source);
    report_chart_layout(&mut state, &session, ChartId::Gear, target);

    begin_chart_drag(&mut state, &session, ChartId::Speed, Point::new(50.0, 20.0));
    let cursor = Point::new(50.0, 40.0);
    update(&mut state, &session, TelemetryMessage::DragCursor(cursor));

    assert!(!target.contains(cursor));
    assert_eq!(state.drop_target, Some(ChartId::Gear));

    update(&mut state, &session, TelemetryMessage::FinishCardDrag);

    assert_eq!(
        &state.chart_order[..8],
        &[
            ChartId::Delta,
            ChartId::Pedal,
            ChartId::BrakePressure,
            ChartId::Abs,
            ChartId::Steering,
            ChartId::SteeringTorque,
            ChartId::Gear,
            ChartId::Speed,
        ]
    );
    assert_eq!(state.dragging_chart, None);
    assert_eq!(state.drop_target, None);
    assert_eq!(state.drag_source_bounds, None);
}

#[test]
fn chart_list_reorders_hidden_items_without_dragging_the_chart_card() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let visibility = state.chart_visibility;
    let source = Rectangle {
        width: 200.0,
        height: DATA_ROW_HEIGHT,
        ..Rectangle::default()
    };
    let target = Rectangle {
        y: DATA_ROW_HEIGHT,
        ..source
    };

    assert!(!state.chart_visibility[ChartId::Abs.index()]);
    report_chart_layout(&mut state, &session, ChartId::Speed, chart_bounds(0.0, 0.0));
    report_chart_list_layout(&mut state, &session, ChartId::Speed, source);
    report_chart_list_layout(&mut state, &session, ChartId::Abs, target);
    begin_chart_list_drag(
        &mut state,
        &session,
        ChartId::Speed,
        Point::new(100.0, 10.0),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(100.0, 40.0)),
    );

    assert_eq!(state.dragging_chart, None);
    assert_eq!(state.chart_list_drop_target, Some(ChartId::Abs));
    update(&mut state, &session, TelemetryMessage::FinishCardDrag);

    assert_eq!(
        &state.chart_order[..5],
        &[
            ChartId::Delta,
            ChartId::Pedal,
            ChartId::BrakePressure,
            ChartId::Abs,
            ChartId::Speed,
        ]
    );
    assert_eq!(state.chart_visibility, visibility);
    assert!(state.chart_layouts.iter().all(Option::is_none));
    assert!(state.chart_list_layouts.iter().all(Option::is_none));
    assert_eq!(state.chart_layout_generation, 1);
    assert_eq!(state.chart_list_layout_generation, 1);
    assert_eq!(state.dragging_chart_list_item, None);
    assert_eq!(state.chart_list_drop_target, None);

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChart(ChartId::Abs, true),
    );
    assert_eq!(state.chart_order[3], ChartId::Abs);
}

#[test]
fn moving_a_chart_list_item_away_cancels_reordering() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let original_order = state.chart_order.clone();
    let source = Rectangle {
        width: 200.0,
        height: DATA_ROW_HEIGHT,
        ..Rectangle::default()
    };
    let target = Rectangle {
        y: DATA_ROW_HEIGHT,
        ..source
    };
    let origin = Point::new(100.0, 10.0);

    report_chart_list_layout(&mut state, &session, ChartId::Speed, source);
    report_chart_list_layout(&mut state, &session, ChartId::Pedal, target);
    begin_chart_list_drag(&mut state, &session, ChartId::Speed, origin);
    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(100.0, 40.0)),
    );
    assert_eq!(state.chart_list_drop_target, Some(ChartId::Pedal));

    update(&mut state, &session, TelemetryMessage::DragCursor(origin));
    assert_eq!(state.chart_list_drop_target, None);
    update(&mut state, &session, TelemetryMessage::FinishCardDrag);

    assert_eq!(state.chart_order, original_order);
}

#[test]
fn moving_a_chart_up_inserts_it_at_the_target_position() {
    let mut order = ChartId::ALL.to_vec();

    assert!(move_item_to(&mut order, ChartId::Fuel, ChartId::Pedal));

    assert_eq!(order[1], ChartId::Fuel);
    assert_eq!(order[2], ChartId::Pedal);
}

#[test]
fn lap_analysis_cards_reorder_by_card_overlap() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let chart_order = state.chart_order.clone();
    let target_bounds = chart_bounds(0.0, 116.0);

    assert_eq!(state.lap_analysis_order, LapAnalysisCardId::ALL);
    report_lap_analysis_layout(
        &mut state,
        &session,
        LapAnalysisCardId::Cursor,
        chart_bounds(0.0, 0.0),
    );
    report_lap_analysis_layout(
        &mut state,
        &session,
        LapAnalysisCardId::Vehicle,
        target_bounds,
    );
    begin_lap_analysis_drag(
        &mut state,
        &session,
        LapAnalysisCardId::Cursor,
        Point::new(50.0, 20.0),
    );
    let cursor = Point::new(50.0, 40.0);
    update(&mut state, &session, TelemetryMessage::DragCursor(cursor));

    assert!(!target_bounds.contains(cursor));
    assert_eq!(
        state.lap_analysis_drop_target,
        Some(LapAnalysisCardId::Vehicle)
    );

    update(&mut state, &session, TelemetryMessage::FinishCardDrag);

    assert_eq!(
        &state.lap_analysis_order[..3],
        &[
            LapAnalysisCardId::ReferenceCursor,
            LapAnalysisCardId::Vehicle,
            LapAnalysisCardId::Cursor,
        ]
    );
    assert_eq!(state.chart_order, chart_order);
    assert_eq!(state.dragging_lap_analysis_card, None);
    assert_eq!(state.lap_analysis_drop_target, None);
}

#[test]
fn moving_a_lap_analysis_card_away_cancels_reordering() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let original_order = state.lap_analysis_order.clone();

    report_lap_analysis_layout(
        &mut state,
        &session,
        LapAnalysisCardId::Inputs,
        chart_bounds(0.0, 0.0),
    );
    report_lap_analysis_layout(
        &mut state,
        &session,
        LapAnalysisCardId::Dynamics,
        chart_bounds(0.0, 116.0),
    );
    begin_lap_analysis_drag(
        &mut state,
        &session,
        LapAnalysisCardId::Inputs,
        Point::new(50.0, 20.0),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(50.0, 40.0)),
    );
    assert_eq!(
        state.lap_analysis_drop_target,
        Some(LapAnalysisCardId::Dynamics)
    );

    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(50.0, 20.0)),
    );
    assert_eq!(state.lap_analysis_drop_target, None);
    update(&mut state, &session, TelemetryMessage::FinishCardDrag);

    assert_eq!(state.lap_analysis_order, original_order);
}

#[test]
fn setup_cards_reorder_by_card_overlap() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let chart_order = state.chart_order.clone();
    let analysis_order = state.lap_analysis_order.clone();
    let target_bounds = chart_bounds(0.0, 116.0);

    assert_eq!(state.setup_card_order, SetupCardId::ALL);
    report_setup_card_layout(
        &mut state,
        &session,
        SetupCardId::Session,
        chart_bounds(0.0, 0.0),
    );
    report_setup_card_layout(&mut state, &session, SetupCardId::Laps, target_bounds);
    begin_setup_card_drag(
        &mut state,
        &session,
        SetupCardId::Session,
        Point::new(50.0, 20.0),
    );
    let cursor = Point::new(50.0, 40.0);
    update(&mut state, &session, TelemetryMessage::DragCursor(cursor));

    assert!(!target_bounds.contains(cursor));
    assert_eq!(state.setup_card_drop_target, Some(SetupCardId::Laps));
    update(&mut state, &session, TelemetryMessage::FinishCardDrag);

    assert_eq!(
        &state.setup_card_order[..3],
        &[
            SetupCardId::Reference,
            SetupCardId::Laps,
            SetupCardId::Session,
        ]
    );
    assert_eq!(state.chart_order, chart_order);
    assert_eq!(state.lap_analysis_order, analysis_order);
    assert_eq!(state.dragging_setup_card, None);
    assert_eq!(state.setup_card_drop_target, None);
    assert_eq!(state.setup_card_drag_source_bounds, None);
}

#[test]
fn moving_a_setup_card_away_cancels_reordering() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let original_order = state.setup_card_order.clone();

    report_setup_card_layout(
        &mut state,
        &session,
        SetupCardId::Reference,
        chart_bounds(0.0, 0.0),
    );
    report_setup_card_layout(
        &mut state,
        &session,
        SetupCardId::Laps,
        chart_bounds(0.0, 116.0),
    );
    begin_setup_card_drag(
        &mut state,
        &session,
        SetupCardId::Reference,
        Point::new(50.0, 20.0),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(50.0, 40.0)),
    );
    assert_eq!(state.setup_card_drop_target, Some(SetupCardId::Laps));

    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(50.0, 20.0)),
    );
    assert_eq!(state.setup_card_drop_target, None);
    update(&mut state, &session, TelemetryMessage::FinishCardDrag);

    assert_eq!(state.setup_card_order, original_order);
}

#[test]
fn telemetry_card_drags_are_mutually_exclusive() {
    let mut state = TelemetryState::default();
    let session = Session::default();

    update(
        &mut state,
        &session,
        TelemetryMessage::BeginChartDrag(ChartId::Speed),
    );
    assert_eq!(state.dragging_chart, Some(ChartId::Speed));
    assert_eq!(state.dragging_chart_list_item, None);
    assert_eq!(state.dragging_lap_analysis_card, None);
    assert_eq!(state.dragging_setup_card, None);

    update(
        &mut state,
        &session,
        TelemetryMessage::BeginChartListDrag(ChartId::Abs),
    );
    assert_eq!(state.dragging_chart, None);
    assert_eq!(state.dragging_chart_list_item, Some(ChartId::Abs));
    assert_eq!(state.dragging_lap_analysis_card, None);
    assert_eq!(state.dragging_setup_card, None);

    update(
        &mut state,
        &session,
        TelemetryMessage::BeginLapAnalysisDrag(LapAnalysisCardId::Vehicle),
    );
    assert_eq!(state.dragging_chart, None);
    assert_eq!(state.dragging_chart_list_item, None);
    assert_eq!(
        state.dragging_lap_analysis_card,
        Some(LapAnalysisCardId::Vehicle)
    );
    assert_eq!(state.dragging_setup_card, None);

    update(
        &mut state,
        &session,
        TelemetryMessage::BeginSetupCardDrag(SetupCardId::Reference),
    );
    assert_eq!(state.dragging_chart, None);
    assert_eq!(state.dragging_chart_list_item, None);
    assert_eq!(state.dragging_lap_analysis_card, None);
    assert_eq!(state.dragging_setup_card, Some(SetupCardId::Reference));

    update(
        &mut state,
        &session,
        TelemetryMessage::BeginChartDrag(ChartId::Pedal),
    );
    assert_eq!(state.dragging_chart, Some(ChartId::Pedal));
    assert_eq!(state.dragging_chart_list_item, None);
    assert_eq!(state.dragging_lap_analysis_card, None);
    assert_eq!(state.dragging_setup_card, None);
}

#[test]
fn cancelling_pointer_interactions_aborts_lap_analysis_dragging() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let original_order = state.lap_analysis_order.clone();

    report_lap_analysis_layout(
        &mut state,
        &session,
        LapAnalysisCardId::Tyres,
        chart_bounds(0.0, 0.0),
    );
    report_lap_analysis_layout(
        &mut state,
        &session,
        LapAnalysisCardId::Wheels,
        chart_bounds(0.0, 116.0),
    );
    begin_lap_analysis_drag(
        &mut state,
        &session,
        LapAnalysisCardId::Tyres,
        Point::new(50.0, 20.0),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(50.0, 40.0)),
    );
    assert_eq!(
        state.lap_analysis_drop_target,
        Some(LapAnalysisCardId::Wheels)
    );

    update(
        &mut state,
        &session,
        TelemetryMessage::CancelPointerInteractions {
            reset_modifiers: false,
        },
    );

    assert_eq!(state.lap_analysis_order, original_order);
    assert_eq!(state.dragging_lap_analysis_card, None);
    assert_eq!(state.lap_analysis_drop_target, None);
    assert_eq!(state.lap_analysis_drag_source_bounds, None);
}

#[test]
fn cancelling_pointer_interactions_aborts_setup_card_dragging() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let original_order = state.setup_card_order.clone();

    report_setup_card_layout(
        &mut state,
        &session,
        SetupCardId::Laps,
        chart_bounds(0.0, 0.0),
    );
    report_setup_card_layout(
        &mut state,
        &session,
        SetupCardId::Charts,
        chart_bounds(0.0, 116.0),
    );
    begin_setup_card_drag(
        &mut state,
        &session,
        SetupCardId::Laps,
        Point::new(50.0, 20.0),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(50.0, 40.0)),
    );
    assert_eq!(state.setup_card_drop_target, Some(SetupCardId::Charts));

    update(
        &mut state,
        &session,
        TelemetryMessage::CancelPointerInteractions {
            reset_modifiers: false,
        },
    );

    assert_eq!(state.setup_card_order, original_order);
    assert_eq!(state.dragging_setup_card, None);
    assert_eq!(state.setup_card_drop_target, None);
    assert_eq!(state.setup_card_drag_source_bounds, None);
}

#[test]
fn moving_the_card_away_cancels_the_drop_target() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChart(ChartId::Gear, true),
    );
    let original_order = state.chart_order.clone();

    report_chart_layout(&mut state, &session, ChartId::Speed, chart_bounds(0.0, 0.0));
    report_chart_layout(
        &mut state,
        &session,
        ChartId::Gear,
        chart_bounds(0.0, 116.0),
    );
    begin_chart_drag(&mut state, &session, ChartId::Speed, Point::new(50.0, 20.0));

    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(50.0, 40.0)),
    );
    assert_eq!(state.drop_target, Some(ChartId::Gear));

    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(50.0, 20.0)),
    );
    assert_eq!(state.drop_target, None);

    update(&mut state, &session, TelemetryMessage::FinishCardDrag);

    assert_eq!(state.chart_order, original_order);
}

#[test]
fn dropping_without_a_target_does_not_reorder_charts() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let original_order = state.chart_order.clone();

    update(
        &mut state,
        &session,
        TelemetryMessage::BeginChartDrag(ChartId::Speed),
    );
    update(&mut state, &session, TelemetryMessage::FinishCardDrag);

    assert_eq!(state.chart_order, original_order);
}

#[test]
fn cancelling_pointer_interactions_aborts_chart_and_cursor_dragging() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let original_order = state.chart_order.clone();

    report_chart_layout(&mut state, &session, ChartId::Speed, chart_bounds(0.0, 0.0));
    report_chart_layout(
        &mut state,
        &session,
        ChartId::Gear,
        chart_bounds(0.0, 116.0),
    );
    begin_chart_drag(&mut state, &session, ChartId::Speed, Point::new(50.0, 20.0));
    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(50.0, 40.0)),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::SpeedPlot(TimeSeriesMessage::BeginCursorDrag),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::KeyboardModifiersChanged(keyboard::Modifiers::CTRL),
    );

    update(
        &mut state,
        &session,
        TelemetryMessage::CancelPointerInteractions {
            reset_modifiers: false,
        },
    );

    assert_eq!(state.chart_order, original_order);
    assert_eq!(state.dragging_chart, None);
    assert_eq!(state.drop_target, None);
    assert_eq!(state.drag_source_bounds, None);
    assert!(!state.speed_chart.is_cursor_dragging());
    assert_eq!(state.modifiers, keyboard::Modifiers::CTRL);

    update(
        &mut state,
        &session,
        TelemetryMessage::CancelPointerInteractions {
            reset_modifiers: true,
        },
    );
    update(&mut state, &session, TelemetryMessage::FinishCardDrag);

    assert_eq!(state.modifiers, keyboard::Modifiers::NONE);
}

#[test]
fn the_target_with_the_largest_visible_overlap_is_selected() {
    let mut layouts = [None; ChartId::COUNT];
    layouts[ChartId::Pedal.index()] = Some(CardLayout {
        bounds: chart_bounds(80.0, 100.0),
        visible_bounds: Some(chart_bounds(80.0, 100.0)),
    });
    layouts[ChartId::Gear.index()] = Some(CardLayout {
        bounds: chart_bounds(160.0, 100.0),
        visible_bounds: Some(chart_bounds(160.0, 100.0)),
    });
    let mut visibility = [true; ChartId::COUNT];
    let dragged = chart_bounds(100.0, 100.0);

    assert_eq!(
        select_drop_target(ChartId::Speed, dragged, &ChartId::ALL, |chart| {
            visibility[chart.index()]
                .then(|| layouts[chart.index()].and_then(|layout| layout.visible_bounds))
                .flatten()
        }),
        Some(ChartId::Pedal)
    );

    visibility[ChartId::Pedal.index()] = false;
    assert_eq!(
        select_drop_target(ChartId::Speed, dragged, &ChartId::ALL, |chart| {
            visibility[chart.index()]
                .then(|| layouts[chart.index()].and_then(|layout| layout.visible_bounds))
                .flatten()
        }),
        Some(ChartId::Gear)
    );
}

#[test]
fn clipped_or_edge_only_overlap_is_not_a_drop_target() {
    let mut layouts = [None; ChartId::COUNT];
    let target = chart_bounds(0.0, 100.0);
    layouts[ChartId::Pedal.index()] = Some(CardLayout {
        bounds: target,
        visible_bounds: Some(Rectangle {
            y: 150.0,
            height: 50.0,
            ..target
        }),
    });

    assert_eq!(
        select_drop_target(
            ChartId::Speed,
            chart_bounds(0.0, 50.0),
            &ChartId::ALL,
            |chart| layouts[chart.index()].and_then(|layout| layout.visible_bounds),
        ),
        None
    );
    assert_eq!(
        select_drop_target(
            ChartId::Speed,
            chart_bounds(0.0, 51.0),
            &ChartId::ALL,
            |chart| layouts[chart.index()].and_then(|layout| layout.visible_bounds),
        ),
        Some(ChartId::Pedal)
    );
}

#[test]
fn chart_layout_and_drag_state_are_updated() {
    let mut state = TelemetryState::default();
    let session = Session::default();

    update(
        &mut state,
        &session,
        TelemetryMessage::SetChartColumns(ChartColumns::Two),
    );
    assert_eq!(state.chart_columns, ChartColumns::Two);
    report_chart_layout(&mut state, &session, ChartId::Speed, chart_bounds(0.0, 0.0));
    report_chart_layout(
        &mut state,
        &session,
        ChartId::Pedal,
        chart_bounds(0.0, 116.0),
    );

    begin_chart_drag(&mut state, &session, ChartId::Speed, Point::new(10.0, 20.0));
    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(10.0, 40.0)),
    );
    assert_eq!(state.drag_origin, Some(Point::new(10.0, 20.0)));
    assert_eq!(state.drag_cursor, Some(Point::new(10.0, 40.0)));
    assert_eq!(state.drop_target, Some(ChartId::Pedal));

    update(&mut state, &session, TelemetryMessage::FinishCardDrag);
    assert_eq!(state.dragging_chart, None);
    assert_eq!(state.drag_origin, None);
    assert_eq!(
        &state.chart_order[..3],
        &[ChartId::Delta, ChartId::Pedal, ChartId::Speed]
    );
}

#[test]
fn chart_maximization_toggles_and_clears_when_hidden() {
    let mut state = TelemetryState::default();
    let session = Session::default();

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChartMaximized(ChartId::Speed),
    );
    assert_eq!(state.maximized_chart, Some(ChartId::Speed));

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChartMaximized(ChartId::Speed),
    );
    assert_eq!(state.maximized_chart, None);

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChartMaximized(ChartId::Pedal),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChart(ChartId::Pedal, false),
    );
    assert_eq!(state.maximized_chart, None);
}

#[test]
fn every_telemetry_card_starts_expanded_and_toggles_independently() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let chart_order = state.chart_order.clone();
    let analysis_order = state.lap_analysis_order.clone();
    let setup_order = state.setup_card_order.clone();

    assert!(state.chart_collapsed.iter().all(|collapsed| !collapsed));
    assert!(
        state
            .lap_analysis_collapsed
            .iter()
            .all(|collapsed| !collapsed)
    );
    assert!(
        state
            .setup_card_collapsed
            .iter()
            .all(|collapsed| !collapsed)
    );

    for chart in ChartId::ALL {
        update(
            &mut state,
            &session,
            TelemetryMessage::ToggleChartCollapsed(chart),
        );
        assert!(state.chart_collapsed[chart.index()]);
        update(
            &mut state,
            &session,
            TelemetryMessage::ToggleChartCollapsed(chart),
        );
        assert!(!state.chart_collapsed[chart.index()]);
    }
    for card in LapAnalysisCardId::ALL {
        update(
            &mut state,
            &session,
            TelemetryMessage::ToggleLapAnalysisCardCollapsed(card),
        );
        assert!(state.lap_analysis_collapsed[card.index()]);
        update(
            &mut state,
            &session,
            TelemetryMessage::ToggleLapAnalysisCardCollapsed(card),
        );
        assert!(!state.lap_analysis_collapsed[card.index()]);
    }
    for card in SetupCardId::ALL {
        update(
            &mut state,
            &session,
            TelemetryMessage::ToggleSetupCardCollapsed(card),
        );
        assert!(state.setup_card_collapsed[card.index()]);
        update(
            &mut state,
            &session,
            TelemetryMessage::ToggleSetupCardCollapsed(card),
        );
        assert!(!state.setup_card_collapsed[card.index()]);
    }

    assert_eq!(state.chart_order, chart_order);
    assert_eq!(state.lap_analysis_order, analysis_order);
    assert_eq!(state.setup_card_order, setup_order);
}

#[test]
fn chart_collapse_and_maximize_keep_a_single_visible_mode() {
    let mut state = TelemetryState::default();
    let session = Session::default();

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChartMaximized(ChartId::Speed),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChartCollapsed(ChartId::Speed),
    );
    assert!(state.chart_collapsed[ChartId::Speed.index()]);
    assert_eq!(state.maximized_chart, None);

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleChartMaximized(ChartId::Speed),
    );
    assert!(!state.chart_collapsed[ChartId::Speed.index()]);
    assert_eq!(state.maximized_chart, Some(ChartId::Speed));
}

#[test]
fn collapsing_during_a_drag_cancels_the_pending_reorder() {
    let mut state = TelemetryState::default();
    let session = Session::default();
    let original_order = state.setup_card_order.clone();
    let source_bounds = Rectangle {
        width: 100.0,
        height: 42.0,
        ..Rectangle::default()
    };
    let target_bounds = Rectangle {
        y: 50.0,
        width: 100.0,
        height: 100.0,
        ..Rectangle::default()
    };

    report_setup_card_layout(&mut state, &session, SetupCardId::Laps, source_bounds);
    report_setup_card_layout(&mut state, &session, SetupCardId::Charts, target_bounds);
    begin_setup_card_drag(
        &mut state,
        &session,
        SetupCardId::Laps,
        Point::new(10.0, 10.0),
    );
    update(
        &mut state,
        &session,
        TelemetryMessage::DragCursor(Point::new(10.0, 70.0)),
    );
    assert_eq!(state.setup_card_drop_target, Some(SetupCardId::Charts));

    update(
        &mut state,
        &session,
        TelemetryMessage::ToggleSetupCardCollapsed(SetupCardId::Laps),
    );
    update(&mut state, &session, TelemetryMessage::FinishCardDrag);

    assert!(state.setup_card_collapsed[SetupCardId::Laps.index()]);
    assert_eq!(state.setup_card_order, original_order);
    assert_eq!(state.dragging_setup_card, None);
    assert_eq!(state.setup_card_drop_target, None);
    assert_eq!(state.setup_card_drag_origin, None);
    assert_eq!(state.setup_card_drag_cursor, None);
    assert_eq!(state.setup_card_drag_source_bounds, None);
}

#[test]
fn collapsed_cards_remain_draggable_with_header_sized_bounds() {
    let session = Session::default();
    let source_bounds = Rectangle {
        width: 100.0,
        height: 42.0,
        ..Rectangle::default()
    };
    let target_bounds = Rectangle {
        y: 50.0,
        width: 100.0,
        height: 42.0,
        ..Rectangle::default()
    };
    let origin = Point::new(10.0, 10.0);
    let target = Point::new(10.0, 60.0);

    let mut charts = TelemetryState::default();
    update(
        &mut charts,
        &session,
        TelemetryMessage::ToggleChartCollapsed(ChartId::Speed),
    );
    report_chart_layout(&mut charts, &session, ChartId::Speed, source_bounds);
    report_chart_layout(&mut charts, &session, ChartId::Pedal, target_bounds);
    begin_chart_drag(&mut charts, &session, ChartId::Speed, origin);
    update(&mut charts, &session, TelemetryMessage::DragCursor(target));
    update(&mut charts, &session, TelemetryMessage::FinishCardDrag);
    assert_eq!(
        &charts.chart_order[..3],
        &[ChartId::Delta, ChartId::Pedal, ChartId::Speed]
    );
    assert!(charts.chart_collapsed[ChartId::Speed.index()]);

    let mut analysis = TelemetryState::default();
    update(
        &mut analysis,
        &session,
        TelemetryMessage::ToggleLapAnalysisCardCollapsed(LapAnalysisCardId::Cursor),
    );
    report_lap_analysis_layout(
        &mut analysis,
        &session,
        LapAnalysisCardId::Cursor,
        source_bounds,
    );
    report_lap_analysis_layout(
        &mut analysis,
        &session,
        LapAnalysisCardId::ReferenceCursor,
        target_bounds,
    );
    begin_lap_analysis_drag(&mut analysis, &session, LapAnalysisCardId::Cursor, origin);
    update(
        &mut analysis,
        &session,
        TelemetryMessage::DragCursor(target),
    );
    update(&mut analysis, &session, TelemetryMessage::FinishCardDrag);
    assert_eq!(
        &analysis.lap_analysis_order[..2],
        &[
            LapAnalysisCardId::ReferenceCursor,
            LapAnalysisCardId::Cursor,
        ]
    );
    assert!(analysis.lap_analysis_collapsed[LapAnalysisCardId::Cursor.index()]);

    let mut setup = TelemetryState::default();
    update(
        &mut setup,
        &session,
        TelemetryMessage::ToggleSetupCardCollapsed(SetupCardId::Session),
    );
    report_setup_card_layout(&mut setup, &session, SetupCardId::Session, source_bounds);
    report_setup_card_layout(&mut setup, &session, SetupCardId::Reference, target_bounds);
    begin_setup_card_drag(&mut setup, &session, SetupCardId::Session, origin);
    update(&mut setup, &session, TelemetryMessage::DragCursor(target));
    update(&mut setup, &session, TelemetryMessage::FinishCardDrag);
    assert_eq!(
        &setup.setup_card_order[..2],
        &[SetupCardId::Reference, SetupCardId::Session]
    );
    assert!(setup.setup_card_collapsed[SetupCardId::Session.index()]);
}

#[test]
fn resets_to_the_latest_complete_lap_and_switches_laps() {
    let timed_sample =
        |elapsed_seconds, completed_laps, current_lap_ms, last_lap_ms, normalized_car_position| {
            TimedSample {
                elapsed_seconds,
                sample: TelemetrySample {
                    completed_laps,
                    current_lap_ms,
                    last_lap_ms,
                    normalized_car_position,
                    ..TelemetrySample::default()
                },
            }
        };
    let frame = TelemetryFrame::try_new(
        3,
        Vec::<chiaro_irsdk::VariableMetadata>::new(),
        Vec::<TelemetryValue>::new(),
    )
    .expect("valid empty frame");
    let mut session = Session::default();
    session.load_ibt(LoadedIbt {
        info: IbtInfo {
            source: RecordingSource::local_file("laps.ibt"),
            file_name: "laps.ibt".to_owned(),
            track_name: "Test Circuit".to_owned(),
            track_id: None,
            track_config_name: None,
            car_id: None,
            car_name: None,
            duration_seconds: 60.0,
            lap_count: 1,
            record_count: 4,
            tick_rate: 60,
        },
        samples: vec![
            timed_sample(0.0, 0, 0, 0, 0.0),
            timed_sample(50.0, 0, 50_000, 0, 0.9),
            timed_sample(51.0, 1, 0, 51_000, 0.0),
            timed_sample(60.0, 1, 9_000, 51_000, 0.2),
        ],
        latest_frame: frame,
        session_info: SessionInfo {
            update_count: 1,
            yaml: String::new(),
            raw: Vec::new(),
        },
    });
    let mut state = TelemetryState::default();

    reset_telemetry(&mut state, &session);

    assert_eq!(state.lap_choices.len(), 2);
    assert_eq!(state.selected_lap_index, Some(0));
    assert_eq!(
        state.focus_x,
        Some(f64::from(0.9_f32) * LAP_DISTANCE_AXIS_MAX)
    );
    assert_eq!(state.speed_chart.x_axis_label(), "Lap distance");
    assert_eq!(state.speed_chart.x_limits(), (0.0, LAP_DISTANCE_AXIS_MAX));

    let action = update(&mut state, &session, TelemetryMessage::SelectLap(1));

    assert_eq!(action, None);
    assert_eq!(state.selected_lap_index, Some(1));
    assert_eq!(
        state.focus_x,
        Some(f64::from(0.2_f32) * LAP_DISTANCE_AXIS_MAX)
    );

    state.focus_from_cursor = true;
    focus_at(&mut state, &session, 1_800.0);

    assert_eq!(
        state.focus_x,
        Some(f64::from(0.2_f32) * LAP_DISTANCE_AXIS_MAX)
    );
    assert_eq!(state.focused.map(|point| point.elapsed_seconds), Some(9.0));
    assert_eq!(state.steering_chart.focus_index(), Some(1));
    assert_eq!(state.rpm_chart.focus_index(), Some(1));
    assert_eq!(state.gear_chart.focus_index(), Some(1));
    assert_eq!(state.yaw_chart.focus_index(), Some(1));
    assert_eq!(state.wheel_slip_chart.focus_index(), Some(1));
    assert_eq!(state.suspension_chart.focus_index(), Some(1));
    assert_eq!(state.fuel_chart.focus_index(), Some(1));
}
