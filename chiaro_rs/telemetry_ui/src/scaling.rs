//! Dynamic Y-axis range calculations for Telemetry charts.

pub(super) fn symmetric_y_limits(
    values: impl Iterator<Item = f64>,
    minimum_half_range: f64,
) -> (f64, f64) {
    let maximum = values
        .filter(|value| value.is_finite())
        .map(f64::abs)
        .fold(0.0, f64::max);
    let padded = maximum * 1.2;
    let half_range = if padded.is_finite() {
        padded.max(minimum_half_range)
    } else {
        minimum_half_range
    };

    (-half_range, half_range)
}

pub(super) fn maximum_y(points: &[[f64; 2]], minimum: f64) -> f64 {
    points
        .iter()
        .map(|point| point[1])
        .filter(|value| value.is_finite())
        .fold(minimum, f64::max)
}

pub(super) fn padded_y_limits(
    values: impl Iterator<Item = f64>,
    minimum_limits: (f64, f64),
) -> (f64, f64) {
    let (minimum, maximum) = values.filter(|value| value.is_finite()).fold(
        (f64::INFINITY, f64::NEG_INFINITY),
        |(minimum, maximum), value| (minimum.min(value), maximum.max(value)),
    );
    if !minimum.is_finite() || !maximum.is_finite() {
        return minimum_limits;
    }

    let span = (maximum - minimum).max(1.0);
    let padding = span * 0.1;
    (
        (minimum - padding).min(minimum_limits.0),
        (maximum + padding).max(minimum_limits.1),
    )
}
