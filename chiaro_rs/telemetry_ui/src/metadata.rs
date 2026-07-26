//! Session-info parsing for the Telemetry screen.

use chiaro_irsdk::{SessionInfoDocument, SessionScalar};
use chiaro_telemetry::Session;

use super::{SessionMetadata, TelemetryState, formatting::format_recording_duration};

pub(super) fn sync_session_metadata(state: &mut TelemetryState, session: &Session) {
    let revision = session.session_info_revision();
    if state.cached_session_info_revision == Some(revision) {
        return;
    }

    state.cached_session_info_revision = Some(revision);
    match session.session_info().map(chiaro_irsdk::SessionInfo::parse) {
        Some(Ok(document)) => state.session_metadata = session_metadata(&document),
        Some(Err(_)) => {
            state.session_metadata = SessionMetadata::default();
        },
        None => state.session_metadata = SessionMetadata::default(),
    }
}

pub(super) fn session_metadata(document: &SessionInfoDocument) -> SessionMetadata {
    let weekend = document.weekend_info.as_ref();
    let options = weekend.and_then(|weekend| weekend.weekend_options.as_ref());
    let current_session = document.session_info.as_ref().and_then(|session_info| {
        session_info
            .current_session_num
            .and_then(|current| {
                session_info
                    .sessions
                    .iter()
                    .find(|session| session.session_num == Some(current))
            })
            .or_else(|| session_info.sessions.first())
    });
    let driver_info = document.driver_info.as_ref();
    let driver = driver_info.and_then(|driver_info| {
        driver_info
            .driver_car_idx
            .and_then(|player_car| {
                driver_info
                    .drivers
                    .iter()
                    .find(|driver| driver.car_idx == Some(player_car))
            })
            .or_else(|| driver_info.drivers.first())
    });

    SessionMetadata {
        track_name: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_display_name.as_deref()),
            weekend.and_then(|weekend| weekend.track_name.as_deref()),
        ]),
        track_config: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_config_name.as_deref())
        ]),
        track_length: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_length.as_deref())
        ]),
        track_turns: weekend.and_then(|weekend| weekend.track_num_turns),
        track_type: join_metadata_values(
            [
                weekend.and_then(|weekend| weekend.track_type.as_deref()),
                weekend.and_then(|weekend| weekend.track_direction.as_deref()),
            ],
            " · ",
        ),
        car_name: first_metadata_value([
            driver.and_then(|driver| driver.car_screen_name.as_deref()),
            driver.and_then(|driver| driver.car_screen_name_short.as_deref()),
            driver.and_then(|driver| driver.car_path.as_deref()),
        ]),
        car_class: first_metadata_value([
            driver.and_then(|driver| driver.car_class_short_name.as_deref())
        ]),
        session_type: first_metadata_value([
            current_session.and_then(|session| session.session_type.as_deref()),
            current_session.and_then(|session| session.session_name.as_deref()),
            weekend.and_then(|weekend| weekend.event_type.as_deref()),
        ]),
        session_time: current_session
            .and_then(|session| session.session_time.as_ref())
            .map(format_session_time),
        date_time: join_metadata_values(
            [
                options.and_then(|options| options.date.as_deref()),
                options.and_then(|options| options.time_of_day.as_deref()),
            ],
            " · ",
        ),
        weather: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_skies.as_deref()),
            options.and_then(|options| options.skies.as_deref()),
            weekend.and_then(|weekend| weekend.track_weather_type.as_deref()),
            options.and_then(|options| options.weather_type.as_deref()),
        ]),
        air_temperature: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_air_temp.as_deref()),
            options.and_then(|options| options.weather_temp.as_deref()),
        ])
        .map(format_temperature),
        surface_temperature: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_surface_temp.as_deref())
        ])
        .map(format_temperature),
        humidity: first_metadata_value([
            weekend.and_then(|weekend| weekend.track_relative_humidity.as_deref()),
            options.and_then(|options| options.relative_humidity.as_deref()),
        ]),
        wind: join_metadata_values(
            [
                options
                    .and_then(|options| options.wind_speed.as_deref())
                    .or_else(|| weekend.and_then(|weekend| weekend.track_wind_vel.as_deref())),
                options
                    .and_then(|options| options.wind_direction.as_deref())
                    .or_else(|| weekend.and_then(|weekend| weekend.track_wind_dir.as_deref())),
            ],
            " · ",
        ),
    }
}

fn first_metadata_value<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_owned)
}

fn join_metadata_values<'a>(
    values: impl IntoIterator<Item = Option<&'a str>>,
    separator: &str,
) -> Option<String> {
    let values = values
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    (!values.is_empty()).then(|| values.join(separator))
}

fn format_temperature(value: String) -> String {
    let trimmed = value.trim();
    let Some(number) = trimmed.strip_suffix('C') else {
        return value;
    };
    let number = number.trim_end().trim_end_matches('°').trim_end();

    if number.parse::<f64>().is_err() {
        return value;
    }

    format!("{number} °C")
}

pub(super) fn format_session_time(value: &SessionScalar) -> String {
    let seconds = match value {
        SessionScalar::Integer(seconds) => Some(*seconds as f64),
        SessionScalar::Float(seconds) => Some(*seconds),
        SessionScalar::Boolean(value) => return value.to_string(),
        SessionScalar::String(value) => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("unlimited") {
                return tr(Text::Unlimited).to_owned();
            }

            value
                .strip_suffix(" sec")
                .and_then(|seconds| seconds.parse::<f64>().ok())
                .or_else(|| value.parse::<f64>().ok())
        },
    };

    seconds
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
        .map_or_else(
            || match value {
                SessionScalar::Integer(value) => value.to_string(),
                SessionScalar::Float(value) => value.to_string(),
                SessionScalar::Boolean(value) => value.to_string(),
                SessionScalar::String(value) => value.trim().to_owned(),
            },
            format_recording_duration,
        )
}
use chiaro_i18n::{Text, tr};
