use chiaro_irsdk::TelemetrySample;
use chiaro_widgets::typography;
use iced::widget::{column, container, row, text};
use iced::{Background, Border, Color, Element, Length, Theme, alignment};

const TYRE_GAP: f32 = 8.0;
const SEGMENT_GAP: f32 = 2.0;
const TILE_RADIUS: f32 = 8.0;
const SEGMENT_RADIUS: f32 = 4.0;

pub fn view<Message: 'static>(sample: Option<TelemetrySample>) -> Element<'static, Message> {
    let sample = sample.unwrap_or_default();
    let front = axle(&sample, 0, 1);
    let rear = axle(&sample, 2, 3);

    container(
        column![
            axle_label("Front"),
            front,
            axle_label("Rear"),
            rear,
            container(
                text("Carcass I / M / O · hot pressure")
                    .size(10)
                    .color(Color::from_rgb(0.62, 0.62, 0.66)),
            )
            .width(Length::Fill)
            .align_x(alignment::Horizontal::Center),
        ]
        .spacing(6)
        .width(Length::Fill),
    )
    .padding([0, 8])
    .width(Length::Fill)
    .into()
}

fn axle<Message: 'static>(
    sample: &TelemetrySample,
    left: usize,
    right: usize,
) -> Element<'static, Message> {
    row![tyre(sample, left), tyre(sample, right)]
        .spacing(TYRE_GAP)
        .width(Length::Fill)
        .into()
}

fn axle_label<Message: 'static>(label: &'static str) -> Element<'static, Message> {
    container(
        text(label)
            .size(10)
            .color(Color::from_rgb(0.62, 0.62, 0.66)),
    )
    .width(Length::Fill)
    .align_x(alignment::Horizontal::Center)
    .into()
}

fn tyre_heading<Message: 'static>(value: String, size: f32) -> Element<'static, Message> {
    container(text(value).size(size).font(typography::MONO))
        .width(Length::FillPortion(1))
        .align_x(alignment::Horizontal::Center)
        .into()
}

fn tyre<Message: 'static>(sample: &TelemetrySample, wheel: usize) -> Element<'static, Message> {
    let label = ["LF", "RF", "LR", "RR"][wheel];
    let pressure = format_pressure(sample.tyre_pressure_kpa.get(wheel));
    let zones = if wheel.is_multiple_of(2) {
        [(2, "O"), (1, "M"), (0, "I")]
    } else {
        [(0, "I"), (1, "M"), (2, "O")]
    };
    let segments = zones.into_iter().fold(
        row![].spacing(SEGMENT_GAP).width(Length::Fill),
        |segments, (zone, zone_label)| {
            let temperature = sample.tyre_carcass_temperature_imo_c.get(wheel * 3 + zone);
            segments.push(temperature_segment(zone_label, temperature))
        },
    );

    container(
        column![
            row![
                tyre_heading(label.to_owned(), 12.0),
                tyre_heading(pressure, 11.0),
            ]
            .width(Length::Fill),
            segments,
        ]
        .spacing(6)
        .width(Length::Fill),
    )
    .padding(7)
    .width(Length::FillPortion(1))
    .style(tyre_tile_style)
    .into()
}

fn temperature_segment<Message: 'static>(
    label: &'static str,
    temperature: Option<f32>,
) -> Element<'static, Message> {
    let value = temperature.map_or_else(|| "--".to_owned(), |value| format!("{value:.0}°C"));

    container(
        column![
            text(label).size(9).font(typography::MONO),
            text(value).size(10).font(typography::MONO),
        ]
        .align_x(iced::Alignment::Center)
        .spacing(1),
    )
    .padding([4, 1])
    .width(Length::FillPortion(1))
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center)
    .style(move |theme| temperature_segment_style(theme, temperature))
    .into()
}

fn format_pressure(pressure_kpa: Option<f32>) -> String {
    pressure_kpa.map_or_else(|| "N/A".to_owned(), |value| format!("{value:.0} kPa"))
}

fn tyre_tile_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            theme.extended_palette().background.weak.color,
        )),
        border: Border {
            radius: TILE_RADIUS.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

fn temperature_segment_style(theme: &Theme, temperature: Option<f32>) -> container::Style {
    let background = temperature.map_or_else(
        || theme.extended_palette().background.strong.color,
        temperature_color,
    );

    container::Style {
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(background)),
        border: Border {
            radius: SEGMENT_RADIUS.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

fn temperature_color(value: f32) -> Color {
    let normalized = ((value - 20.0) / 100.0).clamp(0.0, 1.0);
    if normalized < 0.5 {
        blend(
            Color::from_rgb(0.16, 0.35, 0.78),
            Color::from_rgb(0.10, 0.62, 0.62),
            normalized * 2.0,
        )
    } else {
        blend(
            Color::from_rgb(0.10, 0.62, 0.62),
            Color::from_rgb(0.88, 0.28, 0.22),
            (normalized - 0.5) * 2.0,
        )
    }
}

fn blend(start: Color, end: Color, amount: f32) -> Color {
    Color::from_rgba(
        start.r + (end.r - start.r) * amount,
        start.g + (end.g - start.g) * amount,
        start.b + (end.b - start.b) * amount,
        start.a + (end.a - start.a) * amount,
    )
}

#[cfg(test)]
mod tests {
    use super::{format_pressure, temperature_color};

    #[test]
    fn pressure_readout_distinguishes_missing_from_zero() {
        assert_eq!(format_pressure(None), "N/A");
        assert_eq!(format_pressure(Some(0.0)), "0 kPa");
        assert_eq!(format_pressure(Some(168.4)), "168 kPa");
    }

    #[test]
    fn temperature_scale_moves_from_cool_to_hot() {
        let cool = temperature_color(20.0);
        let hot = temperature_color(120.0);

        assert!(cool.b > cool.r);
        assert!(hot.r > hot.b);
    }
}
