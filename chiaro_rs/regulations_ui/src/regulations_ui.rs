//! Vehicle regulations and supporting specifications screen.

use chiaro_i18n::{Text, cylinder_count, gear_count, idle_rpm, redline_rpm, setup_value, tr};
use chiaro_irsdk::{Driver, DriverInfo, SdkBool, SessionInfoDocument};
use chiaro_telemetry::Session;
use chiaro_widgets::{callout, card, typography};
use iced::{
    Element, Length,
    alignment::Vertical,
    widget::{column, container, row, scrollable, text},
};
use iced_fonts::lucide;

const CONTENT_PADDING: f32 = 16.0;
const CARD_SPACING: f32 = 10.0;
const LABEL_SIZE: u32 = 13;
const VALUE_SIZE: u32 = 14;

struct Section {
    title: &'static str,
    icon: iced::widget::Text<'static>,
    fields: Vec<(&'static str, String)>,
}

/// Builds the read-only vehicle information workspace from the current session.
pub fn view<'a, Message: 'a + 'static>(session: &Session) -> Element<'a, Message> {
    let sections = session
        .session_info()
        .and_then(|info| info.parse().ok())
        .map(vehicle_sections)
        .unwrap_or_default();

    let content: Element<'_, Message> = if sections.is_empty() {
        callout(
            column![
                text(tr(Text::WaitingForCarData))
                    .size(16)
                    .font(typography::SANS_SEMIBOLD),
                text(tr(Text::NoSetupDataDescription)).size(VALUE_SIZE),
            ]
            .spacing(6),
        )
        .padding(16)
        .width(Length::Fill)
        .into()
    } else {
        let mut cards = column![].spacing(CARD_SPACING).width(Length::Fill);
        for section in sections {
            cards = cards.push(section_card(section));
        }
        scrollable(cards)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    container(content)
        .padding(CONTENT_PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn section_card<Message: 'static>(section: Section) -> Element<'static, Message> {
    let mut fields = column![].spacing(0).width(Length::Fill);
    for (label, value) in section.fields {
        fields = fields.push(
            row![
                text(label).size(LABEL_SIZE).font(typography::SANS),
                text(value).size(VALUE_SIZE).font(typography::SANS_SEMIBOLD),
            ]
            .spacing(16)
            .align_y(Vertical::Center)
            .width(Length::Fill)
            .padding([7, 10]),
        );
    }
    card(
        column![
            row![
                section.icon,
                text(section.title).size(16).font(typography::SANS_SEMIBOLD)
            ]
            .spacing(8)
            .align_y(Vertical::Center),
            fields,
        ]
        .spacing(10),
    )
    .padding(12)
    .width(Length::Fill)
    .into()
}

fn vehicle_sections(document: SessionInfoDocument) -> Vec<Section> {
    let info = document.driver_info.as_ref();
    let driver = info.and_then(player_driver);
    let fixed_setup = document
        .weekend_info
        .as_ref()
        .and_then(|weekend| weekend.weekend_options.as_ref())
        .and_then(|options| options.is_fixed_setup)
        .and_then(SdkBool::as_bool);
    let mut sections = Vec::new();

    if let Some(info) = info
        && has_specifications(info, driver)
    {
        sections.push(Section {
            title: tr(Text::VehicleSpecifications),
            icon: lucide::car_front().size(16),
            fields: vec![
                (
                    tr(Text::Version),
                    optional_text(info.driver_car_version.as_deref()),
                ),
                (tr(Text::Powertrain), format_powertrain(info, driver)),
                (tr(Text::Transmission), format_transmission(info)),
                (tr(Text::ShiftLights), format_shift_lights(info)),
                (tr(Text::FuelSystem), format_fuel_system(info)),
                (tr(Text::TyreCompounds), format_tyre_compounds(info)),
            ],
        });
    }
    if has_regulations(info, driver, fixed_setup) {
        sections.push(Section {
            title: tr(Text::Regulations),
            icon: lucide::clipboard_check().size(16),
            fields: vec![
                (
                    tr(Text::Class),
                    optional_text(driver.and_then(|driver| driver.car_class_short_name.as_deref())),
                ),
                (
                    tr(Text::SetupRules),
                    match fixed_setup {
                        Some(true) => tr(Text::FixedSetup).to_owned(),
                        Some(false) => tr(Text::OpenSetup).to_owned(),
                        None => "--".to_owned(),
                    },
                ),
                (tr(Text::FuelAllowance), format_fuel_allowance(info, driver)),
                (
                    tr(Text::WeightPenalty),
                    optional_text(
                        driver.and_then(|driver| driver.car_class_weight_penalty.as_deref()),
                    ),
                ),
                (
                    tr(Text::PowerAdjustment),
                    optional_text(
                        driver.and_then(|driver| driver.car_class_power_adjust.as_deref()),
                    ),
                ),
                (
                    tr(Text::DryTyreSetLimit),
                    format_dry_tyre_set_limit(
                        driver.and_then(|driver| driver.car_class_dry_tire_set_limit.as_deref()),
                    ),
                ),
            ],
        });
    }
    sections
}

fn player_driver(info: &DriverInfo) -> Option<&Driver> {
    let index = info.driver_car_idx?;
    info.drivers
        .iter()
        .find(|driver| driver.car_idx == Some(index))
}

fn has_specifications(info: &DriverInfo, driver: Option<&Driver>) -> bool {
    info.driver_car_version.is_some()
        || info.driver_car_eng_cylinder_count.is_some()
        || info.driver_car_fuel_max_ltr.is_some()
        || !info.driver_tires.is_empty()
        || driver.is_some()
}

fn has_regulations(
    info: Option<&DriverInfo>,
    driver: Option<&Driver>,
    fixed: Option<bool>,
) -> bool {
    fixed.is_some()
        || info.is_some_and(|info| info.driver_car_max_fuel_pct.is_some())
        || driver.is_some_and(|driver| {
            driver.car_class_short_name.is_some()
                || driver.car_class_max_fuel_pct.is_some()
                || driver.car_class_weight_penalty.is_some()
                || driver.car_class_power_adjust.is_some()
                || driver.car_class_dry_tire_set_limit.is_some()
        })
}

fn format_powertrain(info: &DriverInfo, driver: Option<&Driver>) -> String {
    let electric = info
        .driver_car_is_electric
        .and_then(SdkBool::as_bool)
        .or_else(|| {
            driver
                .and_then(|driver| driver.car_is_electric)
                .and_then(SdkBool::as_bool)
        });
    let mut parts = Vec::new();
    match electric {
        Some(true) => parts.push(tr(Text::Electric).to_owned()),
        Some(false) => parts.push(tr(Text::Combustion).to_owned()),
        None => {},
    }
    if electric != Some(true)
        && let Some(cylinders) = info.driver_car_eng_cylinder_count
    {
        parts.push(cylinder_count(cylinders));
    }
    if let Some(range) = format_rpm_range(info.driver_car_idle_rpm, info.driver_car_red_line) {
        parts.push(range);
    }
    joined(parts)
}

fn format_transmission(info: &DriverInfo) -> String {
    let mut parts = Vec::new();
    if let Some(gears) = info.driver_car_gear_num_forward {
        parts.push(gear_count(gears));
    }
    if info.driver_car_gear_neutral.is_some_and(|value| value != 0) {
        parts.push("N".to_owned());
    }
    if info.driver_car_gear_reverse.is_some_and(|value| value != 0) {
        parts.push("R".to_owned());
    }
    joined(parts)
}

fn format_shift_lights(info: &DriverInfo) -> String {
    let values: Vec<_> = [
        ("F", info.driver_car_shift_light_first_rpm),
        ("S", info.driver_car_shift_light_shift_rpm),
        ("L", info.driver_car_shift_light_last_rpm),
        ("B", info.driver_car_shift_light_blink_rpm),
    ]
    .into_iter()
    .filter_map(|(label, value)| {
        value
            .filter(|value| value.is_finite())
            .map(|value| format!("{label} {value:.0}"))
    })
    .collect();
    if values.is_empty() {
        "--".to_owned()
    } else {
        format!("{} rpm", values.join(" · "))
    }
}

fn format_fuel_system(info: &DriverInfo) -> String {
    let mut parts = Vec::new();
    if let Some(value) = info
        .driver_car_fuel_max_ltr
        .filter(|value| value.is_finite())
    {
        parts.push(format!("{value:.1} L"));
    }
    if let Some(value) = info
        .driver_car_fuel_kg_per_ltr
        .filter(|value| value.is_finite())
    {
        parts.push(format!("{value:.3} kg/L"));
    }
    joined(parts)
}

fn format_tyre_compounds(info: &DriverInfo) -> String {
    let mut values = Vec::new();
    for tyre in &info.driver_tires {
        if let Some(compound) = tyre.tire_compound_type.as_deref().and_then(non_blank) {
            let value = tyre.tire_index.map_or_else(
                || setup_value(compound),
                |index| format!("{index} {}", setup_value(compound)),
            );
            if !values.contains(&value) {
                values.push(value);
            }
        }
    }
    joined(values)
}

fn format_rpm_range(idle: Option<f64>, redline: Option<f64>) -> Option<String> {
    match (
        idle.filter(|value| value.is_finite()),
        redline.filter(|value| value.is_finite()),
    ) {
        (Some(idle), Some(redline)) => Some(format!("{idle:.0}–{redline:.0} rpm")),
        (Some(idle), None) => Some(idle_rpm(idle)),
        (None, Some(redline)) => Some(redline_rpm(redline)),
        (None, None) => None,
    }
}

fn format_fuel_allowance(info: Option<&DriverInfo>, driver: Option<&Driver>) -> String {
    let ratio = driver
        .and_then(|driver| driver.car_class_max_fuel_pct.as_deref())
        .and_then(parse_ratio)
        .or_else(|| {
            info.and_then(|info| info.driver_car_max_fuel_pct)
                .filter(|value| value.is_finite() && *value >= 0.0)
        });
    let capacity = info
        .and_then(|info| info.driver_car_fuel_max_ltr)
        .filter(|value| value.is_finite() && *value >= 0.0);
    match (capacity, ratio) {
        (Some(capacity), Some(ratio)) => {
            format!("{:.1} L · {:.1}%", capacity * ratio, ratio * 100.0)
        },
        (Some(capacity), None) => format!("{capacity:.1} L"),
        (None, Some(ratio)) => format!("{:.1}%", ratio * 100.0),
        (None, None) => "--".to_owned(),
    }
}

fn parse_ratio(value: &str) -> Option<f64> {
    let value = leading_number(value)?;
    value.is_finite().then_some(if value.abs() <= 1.0 {
        value
    } else {
        value / 100.0
    })
}
fn format_dry_tyre_set_limit(value: Option<&str>) -> String {
    value
        .and_then(non_blank)
        .and_then(|value| {
            leading_number(value).map(|number| {
                if number.fract().abs() < f64::EPSILON {
                    format!("{number:.0}")
                } else {
                    number.to_string()
                }
            })
        })
        .unwrap_or_else(|| optional_text(value))
}
fn leading_number(value: &str) -> Option<f64> {
    value
        .split_whitespace()
        .next()?
        .trim_end_matches('%')
        .parse()
        .ok()
}
fn optional_text(value: Option<&str>) -> String {
    value.and_then(non_blank).unwrap_or("--").to_owned()
}
fn non_blank(value: &str) -> Option<&str> {
    (!value.trim().is_empty()).then_some(value)
}
fn joined(values: Vec<String>) -> String {
    if values.is_empty() {
        "--".to_owned()
    } else {
        values.join(" · ")
    }
}
