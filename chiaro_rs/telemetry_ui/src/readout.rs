//! Telemetry value conversion for the input analysis card and chart series.

use chiaro_irsdk::TelemetrySample;

const STEERING_METER_HALF_RANGE_RADIANS: f32 = std::f32::consts::PI;

pub(super) fn pedal_percent(value: f32) -> f32 {
    value.clamp(0.0, 1.0) * 100.0
}

pub(super) const fn abs_activity_percent(active: bool) -> f32 {
    if active { 100.0 } else { 0.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputMeter {
    Linear,
    Centered,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct InputMeterValue {
    pub(super) text: String,
    pub(super) progress: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct InputReadout {
    pub(super) throttle: Option<InputMeterValue>,
    pub(super) brake: Option<InputMeterValue>,
    pub(super) steering: Option<InputMeterValue>,
}

pub(super) fn input_readout(
    sample: Option<TelemetrySample>,
    steering_angle_max: Option<f32>,
) -> InputReadout {
    let Some(sample) = sample else {
        return InputReadout::default();
    };

    InputReadout {
        throttle: input_pedal_value(sample.throttle),
        brake: input_pedal_value(sample.brake),
        steering: sample.steering_angle.is_finite().then(|| InputMeterValue {
            text: format!("{:.1}°", sample.steering_angle.to_degrees()),
            progress: steering_angle_max
                .and_then(|maximum| steering_meter_progress(sample.steering_angle, maximum)),
        }),
    }
}

pub(super) fn input_pedal_value(value: f32) -> Option<InputMeterValue> {
    if !value.is_finite() {
        return None;
    }

    let progress = value.clamp(0.0, 1.0);
    Some(InputMeterValue {
        text: format!("{:.1}%", progress * 100.0),
        progress: Some(progress),
    })
}

pub(super) fn steering_meter_progress(angle: f32, maximum: f32) -> Option<f32> {
    let maximum = maximum.abs();
    if !angle.is_finite() || !maximum.is_finite() || maximum <= f32::EPSILON {
        return None;
    }
    let maximum = maximum.min(STEERING_METER_HALF_RANGE_RADIANS);

    // The centered Badge uses negative values for left and positive values for
    // right. Invert the SDK angle only for the visual meter so the fill follows
    // the physical steering direction while the displayed number stays raw.
    Some((-angle / maximum).clamp(-1.0, 1.0))
}
