use chiaro_irsdk::{Driver, DriverInfo, SdkBool, SessionInfoDocument};
use chiaro_telemetry::{ConnectionStatus, LiveTelemetrySourceInfo, Session};
use chiaro_widgets::{BadgeVariant, badge, callout, typography};
use iced::{
    Element, Length, Padding, Theme,
    alignment::{Horizontal, Vertical},
    widget::{Space, column, container, grid, responsive, row, rule, text},
};
use iced_fonts::lucide;
use serde_yaml_ng::Value;

use super::CarSetupMessage;

const SECTION_TITLE_SIZE: u32 = 16;
const BODY_SIZE: u32 = 14;
const LABEL_SIZE: u32 = 12;
const ROW_VERTICAL_PADDING: f32 = 8.0;
const ROW_HORIZONTAL_PADDING: f32 = 12.0;
const NESTED_INDENT: f32 = 14.0;
const MAX_VISUAL_INDENT_DEPTH: usize = 6;
const CORNER_TABLE_BREAKPOINT: f32 = 560.0;
const CORNER_LABEL_PORTION: u16 = 6;
const CORNER_VALUE_PORTION: u16 = 2;
const TABLE_BODY_SIZE: u32 = 13;
const TABLE_CELL_VERTICAL_PADDING: f32 = 7.0;
const TABLE_CELL_HORIZONTAL_PADDING: f32 = 8.0;
const SUMMARY_CARD_KEY: &str = "summary";
const STATUS_CARD_KEY: &str = "status";
const VEHICLE_SPECIFICATIONS_CARD_KEY: &str = "vehicle:specifications";
const REGULATIONS_CARD_KEY: &str = "vehicle:regulations";
const GENERAL_SETUP_CARD_KEY: &str = "setup:general";
const VALUE_SETUP_CARD_KEY: &str = "setup:value";
const SETUP_SECTION_CARD_PREFIX: &str = "setup:section:";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SetupViewData {
    status: SetupDataStatus,
    car_name: Option<String>,
    car_path: Option<String>,
    setup_name: Option<String>,
    load_type: Option<String>,
    modified: Option<bool>,
    passed_tech: Option<bool>,
    fixed_setup: Option<bool>,
    update_count: Option<String>,
    vehicle_sections: Vec<SetupSection>,
    sections: Vec<SetupSection>,
}

impl SetupViewData {
    pub(super) fn from_document(document: &SessionInfoDocument) -> Self {
        let driver_info = document.driver_info.as_ref();
        let driver = driver_info.and_then(|driver_info| {
            driver_info.driver_car_idx.and_then(|player_car| {
                driver_info
                    .drivers
                    .iter()
                    .find(|driver| driver.car_idx == Some(player_car))
            })
        });
        let fixed_setup = document
            .weekend_info
            .as_ref()
            .and_then(|weekend| weekend.weekend_options.as_ref())
            .and_then(|options| options.is_fixed_setup)
            .and_then(SdkBool::as_bool);
        let (update_count, sections) = document.car_setup.as_ref().map_or_else(
            || (None, Vec::new()),
            |setup| (setup_update_count(setup), setup_sections(setup)),
        );
        let vehicle_sections = vehicle_sections(driver_info, driver, fixed_setup);

        Self {
            status: if document.car_setup.is_some() {
                SetupDataStatus::Available
            } else {
                SetupDataStatus::Missing
            },
            car_name: first_non_blank([
                driver.and_then(|driver| driver.car_screen_name.as_deref()),
                driver.and_then(|driver| driver.car_screen_name_short.as_deref()),
                driver.and_then(|driver| driver.car_path.as_deref()),
            ]),
            car_path: driver
                .and_then(|driver| driver.car_path.as_deref())
                .and_then(non_blank)
                .map(str::to_owned),
            setup_name: driver_info
                .and_then(|info| info.driver_setup_name.as_deref())
                .and_then(non_blank)
                .map(str::to_owned),
            load_type: driver_info
                .and_then(|info| info.driver_setup_load_type_name.as_deref())
                .and_then(non_blank)
                .map(str::to_owned),
            modified: driver_info
                .and_then(|info| info.driver_setup_is_modified)
                .and_then(SdkBool::as_bool),
            passed_tech: driver_info
                .and_then(|info| info.driver_setup_passed_tech)
                .and_then(SdkBool::as_bool),
            fixed_setup,
            update_count,
            vehicle_sections,
            sections,
        }
    }

    pub(super) fn parse_error(error: impl Into<String>) -> Self {
        Self {
            status: SetupDataStatus::ParseError(error.into()),
            ..Self::default()
        }
    }

    pub(super) fn card_keys(&self) -> Vec<String> {
        let mut keys = Vec::with_capacity(
            2 + self
                .vehicle_sections
                .len()
                .saturating_add(self.sections.len()),
        );
        keys.push(SUMMARY_CARD_KEY.to_owned());
        keys.extend(
            self.vehicle_sections
                .iter()
                .map(|section| section.key.clone()),
        );
        if matches!(self.status, SetupDataStatus::Available) && !self.sections.is_empty() {
            keys.extend(self.sections.iter().map(|section| section.key.clone()));
        } else {
            keys.push(STATUS_CARD_KEY.to_owned());
        }
        keys
    }

    pub(super) fn card_title(&self, session: &Session, key: &str) -> String {
        if key == SUMMARY_CARD_KEY {
            return self
                .car_name
                .as_deref()
                .or_else(|| session.ibt_info().and_then(|info| info.car_name.as_deref()))
                .unwrap_or("Waiting for car data")
                .to_owned();
        }
        if key == STATUS_CARD_KEY {
            return "Setup data".to_owned();
        }

        self.section(key)
            .map_or_else(|| "Setup".to_owned(), |section| section.title.clone())
    }

    pub(super) fn card_icon(key: &str) -> iced::widget::Text<'static> {
        match key {
            SUMMARY_CARD_KEY | VEHICLE_SPECIFICATIONS_CARD_KEY => lucide::car_front(),
            STATUS_CARD_KEY => lucide::info(),
            REGULATIONS_CARD_KEY => lucide::shield_check(),
            _ => lucide::sliders_horizontal(),
        }
    }

    pub(super) fn card_content<'a>(
        &'a self,
        session: &'a Session,
        live_source: LiveTelemetrySourceInfo,
        key: &str,
    ) -> Element<'a, CarSetupMessage> {
        match key {
            SUMMARY_CARD_KEY => summary_content(session, live_source, self),
            STATUS_CARD_KEY => status_content(self),
            _ => self.section(key).map_or_else(
                || {
                    empty_state(
                        "Setup unavailable",
                        "This setup section is no longer available.",
                    )
                },
                section_content,
            ),
        }
    }

    pub(super) fn card_title_trailing<'a>(
        &'a self,
        key: &str,
    ) -> Option<Element<'a, CarSetupMessage>> {
        (key == SUMMARY_CARD_KEY).then(|| {
            row(status_badges(self))
                .spacing(8)
                .align_y(Vertical::Center)
                .into()
        })
    }

    pub(super) fn card_weight(&self, key: &str) -> usize {
        match key {
            SUMMARY_CARD_KEY => 8,
            STATUS_CARD_KEY => 4,
            _ => self
                .section(key)
                .map_or(2, |section| section.rows.len().saturating_add(2)),
        }
    }

    pub(super) fn card_is_full_width(key: &str) -> bool {
        matches!(key, SUMMARY_CARD_KEY | STATUS_CARD_KEY)
    }

    fn section(&self, key: &str) -> Option<&SetupSection> {
        self.vehicle_sections
            .iter()
            .chain(&self.sections)
            .find(|section| section.key == key)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum SetupDataStatus {
    #[default]
    Waiting,
    Missing,
    Available,
    ParseError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SetupSection {
    key: String,
    title: String,
    rows: Vec<SetupRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SetupRow {
    Group {
        label: String,
        depth: usize,
    },
    Value {
        label: String,
        value: String,
        depth: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum SetupCorner {
    LeftFront,
    RightFront,
    LeftRear,
    RightRear,
}

impl SetupCorner {
    const ALL: [Self; 4] = [
        Self::LeftFront,
        Self::RightFront,
        Self::LeftRear,
        Self::RightRear,
    ];

    const fn index(self) -> usize {
        self as usize
    }

    const fn short_label(self) -> &'static str {
        match self {
            Self::LeftFront => "LF",
            Self::RightFront => "RF",
            Self::LeftRear => "LR",
            Self::RightRear => "RR",
        }
    }

    const fn is_front(self) -> bool {
        matches!(self, Self::LeftFront | Self::RightFront)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComparisonFieldKey {
    path: String,
    occurrence: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CornerComparisonRow {
    key: ComparisonFieldKey,
    values: [Option<String>; SetupCorner::ALL.len()],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CornerComparison {
    columns: Vec<SetupCorner>,
    rows: Vec<CornerComparisonRow>,
}

#[derive(Debug)]
enum SetupContentBlock<'a> {
    Rows(Vec<&'a SetupRow>),
    CornerComparison(CornerComparison),
}

#[derive(Debug, Clone, Copy)]
struct RootSetupEntry<'a> {
    rows: &'a [SetupRow],
    corner: Option<SetupCorner>,
}

fn vehicle_sections(
    driver_info: Option<&DriverInfo>,
    driver: Option<&Driver>,
    fixed_setup: Option<bool>,
) -> Vec<SetupSection> {
    let mut sections = Vec::new();

    if let Some(info) = driver_info
        && has_vehicle_specifications(info, driver)
    {
        sections.push(info_section(
            VEHICLE_SPECIFICATIONS_CARD_KEY,
            "Vehicle specifications",
            [
                ("Version", optional_text(info.driver_car_version.as_deref())),
                ("Powertrain", format_powertrain(info, driver)),
                ("Transmission", format_transmission(info)),
                ("Shift lights", format_shift_lights(info)),
                ("Fuel system", format_fuel_system(info)),
                ("Tyre compounds", format_tyre_compounds(info)),
            ],
        ));
    }

    if has_regulations(driver_info, driver, fixed_setup) {
        sections.push(info_section(
            REGULATIONS_CARD_KEY,
            "Regulations",
            [
                (
                    "Class",
                    optional_text(driver.and_then(|driver| driver.car_class_short_name.as_deref())),
                ),
                (
                    "Setup rules",
                    match fixed_setup {
                        Some(true) => "Fixed setup".to_owned(),
                        Some(false) => "Open setup".to_owned(),
                        None => "--".to_owned(),
                    },
                ),
                ("Fuel allowance", format_fuel_allowance(driver_info, driver)),
                (
                    "Weight penalty",
                    optional_text(
                        driver.and_then(|driver| driver.car_class_weight_penalty.as_deref()),
                    ),
                ),
                (
                    "Power adjustment",
                    optional_text(
                        driver.and_then(|driver| driver.car_class_power_adjust.as_deref()),
                    ),
                ),
                (
                    "Dry tyre set limit",
                    format_dry_tyre_set_limit(
                        driver.and_then(|driver| driver.car_class_dry_tire_set_limit.as_deref()),
                    ),
                ),
            ],
        ));
    }

    sections
}

fn info_section<const N: usize>(
    key: &'static str,
    title: &'static str,
    rows: [(&'static str, String); N],
) -> SetupSection {
    SetupSection {
        key: key.to_owned(),
        title: title.to_owned(),
        rows: rows
            .into_iter()
            .map(|(label, value)| SetupRow::Value {
                label: label.to_owned(),
                value,
                depth: 0,
            })
            .collect(),
    }
}

fn has_vehicle_specifications(info: &DriverInfo, driver: Option<&Driver>) -> bool {
    info.driver_car_version
        .as_deref()
        .and_then(non_blank)
        .is_some()
        || info.driver_car_is_electric.is_some()
        || info.driver_car_eng_cylinder_count.is_some()
        || info.driver_car_idle_rpm.is_some()
        || info.driver_car_red_line.is_some()
        || info.driver_car_gear_num_forward.is_some()
        || info.driver_car_gear_neutral.is_some()
        || info.driver_car_gear_reverse.is_some()
        || info.driver_car_shift_light_first_rpm.is_some()
        || info.driver_car_shift_light_shift_rpm.is_some()
        || info.driver_car_shift_light_last_rpm.is_some()
        || info.driver_car_shift_light_blink_rpm.is_some()
        || info.driver_car_fuel_max_ltr.is_some()
        || info.driver_car_fuel_kg_per_ltr.is_some()
        || !info.driver_tires.is_empty()
        || driver.is_some_and(|driver| driver.car_is_electric.is_some())
}

fn has_regulations(
    driver_info: Option<&DriverInfo>,
    driver: Option<&Driver>,
    fixed_setup: Option<bool>,
) -> bool {
    fixed_setup.is_some()
        || driver_info.is_some_and(|info| info.driver_car_max_fuel_pct.is_some())
        || driver.is_some_and(|driver| {
            driver
                .car_class_short_name
                .as_deref()
                .and_then(non_blank)
                .is_some()
                || driver
                    .car_class_max_fuel_pct
                    .as_deref()
                    .and_then(non_blank)
                    .is_some()
                || driver
                    .car_class_weight_penalty
                    .as_deref()
                    .and_then(non_blank)
                    .is_some()
                || driver
                    .car_class_power_adjust
                    .as_deref()
                    .and_then(non_blank)
                    .is_some()
                || driver
                    .car_class_dry_tire_set_limit
                    .as_deref()
                    .and_then(non_blank)
                    .is_some()
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
        Some(true) => parts.push("Electric".to_owned()),
        Some(false) => parts.push("Combustion".to_owned()),
        None => {},
    }
    if electric != Some(true)
        && let Some(cylinders) = info.driver_car_eng_cylinder_count
    {
        parts.push(format!("{cylinders} cyl"));
    }
    if let Some(range) = format_rpm_range(info.driver_car_idle_rpm, info.driver_car_red_line) {
        parts.push(range);
    }

    joined_or_placeholder(parts)
}

fn format_transmission(info: &DriverInfo) -> String {
    let mut parts = Vec::new();
    if let Some(gears) = info.driver_car_gear_num_forward {
        parts.push(format!("{gears}-speed"));
    }
    if info
        .driver_car_gear_neutral
        .is_some_and(|available| available != 0)
    {
        parts.push("N".to_owned());
    }
    if info
        .driver_car_gear_reverse
        .is_some_and(|available| available != 0)
    {
        parts.push("R".to_owned());
    }

    joined_or_placeholder(parts)
}

fn format_shift_lights(info: &DriverInfo) -> String {
    let stages = [
        ("F", info.driver_car_shift_light_first_rpm),
        ("S", info.driver_car_shift_light_shift_rpm),
        ("L", info.driver_car_shift_light_last_rpm),
        ("B", info.driver_car_shift_light_blink_rpm),
    ];
    let values = stages
        .into_iter()
        .filter_map(|(label, value)| {
            value
                .filter(|value| value.is_finite())
                .map(|value| format!("{label} {value:.0}"))
        })
        .collect::<Vec<_>>();

    if values.is_empty() {
        "--".to_owned()
    } else {
        format!("{} rpm", values.join(" · "))
    }
}

fn format_fuel_system(info: &DriverInfo) -> String {
    let mut parts = Vec::new();
    if let Some(capacity) = info
        .driver_car_fuel_max_ltr
        .filter(|value| value.is_finite())
    {
        parts.push(format!("{capacity:.1} L"));
    }
    if let Some(density) = info
        .driver_car_fuel_kg_per_ltr
        .filter(|value| value.is_finite())
    {
        parts.push(format!("{density:.3} kg/L"));
    }

    joined_or_placeholder(parts)
}

fn format_tyre_compounds(info: &DriverInfo) -> String {
    let mut compounds = Vec::new();
    for tyre in &info.driver_tires {
        let Some(compound) = tyre.tire_compound_type.as_deref().and_then(non_blank) else {
            continue;
        };
        let value = tyre.tire_index.map_or_else(
            || compound.to_owned(),
            |index| format!("{index} {compound}"),
        );
        if !compounds.contains(&value) {
            compounds.push(value);
        }
    }

    joined_or_placeholder(compounds)
}

fn format_rpm_range(idle: Option<f64>, redline: Option<f64>) -> Option<String> {
    match (
        idle.filter(|value| value.is_finite()),
        redline.filter(|value| value.is_finite()),
    ) {
        (Some(idle), Some(redline)) => Some(format!("{idle:.0}–{redline:.0} rpm")),
        (Some(idle), None) => Some(format!("{idle:.0} rpm idle")),
        (None, Some(redline)) => Some(format!("{redline:.0} rpm redline")),
        (None, None) => None,
    }
}

fn format_fuel_allowance(driver_info: Option<&DriverInfo>, driver: Option<&Driver>) -> String {
    let ratio = driver
        .and_then(|driver| driver.car_class_max_fuel_pct.as_deref())
        .and_then(parse_fuel_ratio)
        .or_else(|| {
            driver_info
                .and_then(|info| info.driver_car_max_fuel_pct)
                .filter(|value| value.is_finite() && *value >= 0.0)
        });
    let capacity = driver_info
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

fn parse_fuel_ratio(value: &str) -> Option<f64> {
    let value = leading_number(value)?;
    value.is_finite().then_some(if value.abs() <= 1.0 {
        value
    } else {
        value / 100.0
    })
}

fn format_dry_tyre_set_limit(value: Option<&str>) -> String {
    let Some(value) = value.and_then(non_blank) else {
        return "--".to_owned();
    };
    let Some(limit) = leading_number(value) else {
        return value.to_owned();
    };

    if limit.fract().abs() < f64::EPSILON {
        format!("{limit:.0}")
    } else {
        limit.to_string()
    }
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

fn joined_or_placeholder(values: Vec<String>) -> String {
    if values.is_empty() {
        "--".to_owned()
    } else {
        values.join(" · ")
    }
}

fn setup_content_blocks(section: &SetupSection) -> Vec<SetupContentBlock<'_>> {
    let Some(entries) = root_setup_entries(&section.rows) else {
        return vec![SetupContentBlock::Rows(section.rows.iter().collect())];
    };
    let mut blocks = Vec::new();
    let mut plain_rows = Vec::new();
    let mut index = 0;

    while index < entries.len() {
        if entries[index].corner.is_none() {
            plain_rows.extend(entries[index].rows);
            index += 1;
            continue;
        }

        let mut end = index + 1;
        while end < entries.len() && entries[end].corner.is_some() {
            end += 1;
        }

        if let Some(comparison) = corner_comparison(&entries[index..end]) {
            push_plain_setup_block(&mut blocks, &mut plain_rows);
            blocks.push(SetupContentBlock::CornerComparison(comparison));
        } else {
            for entry in &entries[index..end] {
                plain_rows.extend(entry.rows);
            }
        }
        index = end;
    }

    push_plain_setup_block(&mut blocks, &mut plain_rows);
    blocks
}

fn push_plain_setup_block<'a>(
    blocks: &mut Vec<SetupContentBlock<'a>>,
    rows: &mut Vec<&'a SetupRow>,
) {
    if !rows.is_empty() {
        blocks.push(SetupContentBlock::Rows(std::mem::take(rows)));
    }
}

fn root_setup_entries(rows: &[SetupRow]) -> Option<Vec<RootSetupEntry<'_>>> {
    if rows.is_empty() {
        return Some(Vec::new());
    }

    let mut starts = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let depth = setup_row_depth(row);
        if depth == 0 {
            starts.push(index);
            continue;
        }

        let previous = index.checked_sub(1).and_then(|index| rows.get(index))?;
        let previous_depth = setup_row_depth(previous);
        if starts.is_empty()
            || depth > previous_depth.saturating_add(1)
            || (depth > previous_depth && !matches!(previous, SetupRow::Group { .. }))
        {
            return None;
        }
    }

    let mut entries = Vec::with_capacity(starts.len());
    for (position, start) in starts.iter().copied().enumerate() {
        let end = starts.get(position + 1).copied().unwrap_or(rows.len());
        let entry_rows = &rows[start..end];
        let corner = match entry_rows.first() {
            Some(SetupRow::Group { label, depth: 0 }) => setup_corner(label),
            _ => None,
        };
        entries.push(RootSetupEntry {
            rows: entry_rows,
            corner,
        });
    }

    Some(entries)
}

fn corner_comparison(entries: &[RootSetupEntry<'_>]) -> Option<CornerComparison> {
    if entries.len() < 2 {
        return None;
    }

    let mut present = [false; SetupCorner::ALL.len()];
    let mut corner_fields = Vec::with_capacity(entries.len());
    for entry in entries {
        let corner = entry.corner?;
        if present[corner.index()] {
            return None;
        }
        present[corner.index()] = true;

        let fields = setup_corner_fields(entry.rows.get(1..)?)?;
        if fields.is_empty() {
            return None;
        }
        corner_fields.push((corner, fields));
    }

    let columns = SetupCorner::ALL
        .into_iter()
        .filter(|corner| present[corner.index()])
        .collect::<Vec<_>>();
    let mut rows: Vec<CornerComparisonRow> = Vec::new();

    for (corner, fields) in corner_fields {
        for (key, value) in fields {
            if let Some(row) = rows.iter_mut().find(|row| row.key == key) {
                row.values[corner.index()] = Some(value);
            } else {
                let mut values = std::array::from_fn(|_| None);
                values[corner.index()] = Some(value);
                rows.push(CornerComparisonRow { key, values });
            }
        }
    }

    rows.iter()
        .any(|row| row.values.iter().flatten().count() >= 2)
        .then_some(CornerComparison { columns, rows })
}

fn setup_corner_fields(rows: &[SetupRow]) -> Option<Vec<(ComparisonFieldKey, String)>> {
    let mut parents: Vec<&str> = Vec::new();
    let mut fields: Vec<(ComparisonFieldKey, String)> = Vec::new();

    for row in rows {
        match row {
            SetupRow::Group { label, depth } => {
                let relative_depth = depth.checked_sub(1)?;
                if relative_depth > parents.len() {
                    return None;
                }
                parents.truncate(relative_depth);
                parents.push(label);
            },
            SetupRow::Value {
                label,
                value,
                depth,
            } => {
                let relative_depth = depth.checked_sub(1)?;
                if relative_depth > parents.len() {
                    return None;
                }
                parents.truncate(relative_depth);

                let path = if parents.is_empty() {
                    label.clone()
                } else {
                    format!("{} · {label}", parents.join(" · "))
                };
                let occurrence = fields.iter().filter(|(key, _)| key.path == path).count();
                fields.push((ComparisonFieldKey { path, occurrence }, value.clone()));
            },
        }
    }

    Some(fields)
}

fn setup_row_depth(row: &SetupRow) -> usize {
    match row {
        SetupRow::Group { depth, .. } | SetupRow::Value { depth, .. } => *depth,
    }
}

fn setup_corner(label: &str) -> Option<SetupCorner> {
    let normalized = label
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect::<String>();

    match normalized.as_str() {
        "lf" | "leftfront" | "frontleft" | "leftfrontcorner" | "frontleftcorner" => {
            Some(SetupCorner::LeftFront)
        },
        "rf" | "rightfront" | "frontright" | "rightfrontcorner" | "frontrightcorner" => {
            Some(SetupCorner::RightFront)
        },
        "lr" | "leftrear" | "rearleft" | "leftrearcorner" | "rearleftcorner" => {
            Some(SetupCorner::LeftRear)
        },
        "rr" | "rightrear" | "rearright" | "rightrearcorner" | "rearrightcorner" => {
            Some(SetupCorner::RightRear)
        },
        _ => None,
    }
}

fn summary_content<'a>(
    session: &'a Session,
    live_source: LiveTelemetrySourceInfo,
    data: &'a SetupViewData,
) -> Element<'a, CarSetupMessage> {
    let details = grid([
        summary_item("Setup", data.setup_name.as_deref().unwrap_or("--")),
        summary_item("Load type", data.load_type.as_deref().unwrap_or("--")),
        summary_item("Car path", data.car_path.as_deref().unwrap_or("--")),
        summary_item("Source", source_label(session, live_source)),
        summary_item("Revision", data.update_count.as_deref().unwrap_or("--")),
        summary_item("Access", "Read only"),
    ])
    .columns(3)
    .spacing(1)
    .height(Length::Shrink);
    column![container(details).padding(4).width(Length::Fill)]
        .spacing(10)
        .width(Length::Fill)
        .into()
}

fn status_badges<'a>(data: &'a SetupViewData) -> Vec<Element<'a, CarSetupMessage>> {
    let mut statuses = Vec::new();

    if let Some(modified) = data.modified {
        statuses.push(
            badge(if modified { "Modified" } else { "Saved" })
                .variant(if modified {
                    BadgeVariant::Primary
                } else {
                    BadgeVariant::Success
                })
                .into(),
        );
    }
    if let Some(passed) = data.passed_tech {
        statuses.push(
            badge(if passed { "Tech passed" } else { "Tech failed" })
                .variant(if passed {
                    BadgeVariant::Success
                } else {
                    BadgeVariant::Danger
                })
                .into(),
        );
    }

    statuses
}

fn summary_item<'a>(
    label: &'static str,
    value: impl text::IntoFragment<'a>,
) -> Element<'a, CarSetupMessage> {
    container(
        column![
            text(label)
                .size(LABEL_SIZE)
                .font(typography::SANS)
                .style(secondary_text_style),
            text(value).size(BODY_SIZE).font(typography::SANS_SEMIBOLD),
        ]
        .spacing(3),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .into()
}

fn section_content(section: &SetupSection) -> Element<'_, CarSetupMessage> {
    let blocks = setup_content_blocks(section);
    let mut content = column![].spacing(8);

    if blocks.is_empty() {
        content = content.push(setup_value_row("Value", "--", 0));
    } else {
        for block in blocks {
            content = content.push(match block {
                SetupContentBlock::Rows(rows) => setup_rows_view(rows),
                SetupContentBlock::CornerComparison(comparison) => {
                    corner_comparison_view(comparison)
                },
            });
        }
    }

    container(content).width(Length::Fill).clip(true).into()
}

fn setup_rows_view(rows: Vec<&SetupRow>) -> Element<'_, CarSetupMessage> {
    let mut content = column![];

    for (index, setup_row) in rows.iter().copied().enumerate() {
        let display_depth = setup_row_depth(setup_row).saturating_sub(1);
        match setup_row {
            SetupRow::Group { label, .. } => {
                if index > 0 {
                    content =
                        content.push(Space::new().height(if display_depth == 0 { 8 } else { 4 }));
                }
                content = content.push(setup_group_row(label, display_depth));
            },
            SetupRow::Value { label, value, .. } => {
                content = content.push(setup_value_row(label, value, display_depth));
                if rows
                    .get(index + 1)
                    .is_some_and(|row| matches!(row, SetupRow::Value { .. }))
                {
                    content = content.push(rule::horizontal(1).style(separator_style));
                }
            },
        }
    }

    content.width(Length::Fill).into()
}

fn corner_comparison_view(comparison: CornerComparison) -> Element<'static, CarSetupMessage> {
    responsive(move |size| {
        corner_comparison_layout(&comparison, size.width < CORNER_TABLE_BREAKPOINT)
    })
    .width(Length::Fill)
    .height(Length::Shrink)
    .into()
}

fn corner_comparison_layout(
    comparison: &CornerComparison,
    compact: bool,
) -> Element<'static, CarSetupMessage> {
    let title = container(
        text("Corner comparison")
            .size(TABLE_BODY_SIZE)
            .font(typography::SANS_SEMIBOLD),
    )
    .padding(Padding {
        top: 10.0,
        right: ROW_HORIZONTAL_PADDING,
        bottom: 5.0,
        left: ROW_HORIZONTAL_PADDING,
    })
    .width(Length::Fill);
    let front = comparison
        .columns
        .iter()
        .copied()
        .filter(|corner| corner.is_front())
        .collect::<Vec<_>>();
    let rear = comparison
        .columns
        .iter()
        .copied()
        .filter(|corner| !corner.is_front())
        .collect::<Vec<_>>();

    if compact && comparison.columns.len() > 2 && !front.is_empty() && !rear.is_empty() {
        return column![
            title,
            corner_table_group("Front", comparison, &front),
            corner_table_group("Rear", comparison, &rear),
        ]
        .spacing(6)
        .width(Length::Fill)
        .into();
    }

    column![title, corner_table(comparison, &comparison.columns)]
        .width(Length::Fill)
        .into()
}

fn corner_table_group(
    label: &'static str,
    comparison: &CornerComparison,
    columns: &[SetupCorner],
) -> Element<'static, CarSetupMessage> {
    column![
        container(
            text(label)
                .size(LABEL_SIZE)
                .font(typography::SANS_SEMIBOLD)
                .style(secondary_text_style),
        )
        .padding(Padding {
            top: 3.0,
            right: ROW_HORIZONTAL_PADDING,
            bottom: 2.0,
            left: ROW_HORIZONTAL_PADDING,
        }),
        corner_table(comparison, columns),
    ]
    .width(Length::Fill)
    .into()
}

fn corner_table(
    comparison: &CornerComparison,
    columns: &[SetupCorner],
) -> Element<'static, CarSetupMessage> {
    let visible_rows = comparison
        .rows
        .iter()
        .filter(|row| {
            columns
                .iter()
                .any(|corner| row.values[corner.index()].is_some())
        })
        .collect::<Vec<_>>();
    let mut table = column![corner_table_header(columns)];

    if !visible_rows.is_empty() {
        table = table.push(rule::horizontal(1).style(separator_style));
    }
    let last = visible_rows.len().saturating_sub(1);
    for (index, comparison_row) in visible_rows.into_iter().enumerate() {
        table = table.push(corner_table_row(comparison_row, columns));
        if index != last {
            table = table.push(rule::horizontal(1).style(separator_style));
        }
    }

    table.width(Length::Fill).into()
}

fn corner_table_header(columns: &[SetupCorner]) -> Element<'static, CarSetupMessage> {
    let mut header = row![corner_table_label_cell("Setting".to_owned(), true)]
        .spacing(0)
        .align_y(Vertical::Center)
        .width(Length::Fill);
    for corner in columns {
        header =
            header
                .push(rule::vertical(1).style(separator_style))
                .push(corner_table_value_cell(
                    Some(corner.short_label().to_owned()),
                    true,
                ));
    }

    header.into()
}

fn corner_table_row(
    comparison_row: &CornerComparisonRow,
    columns: &[SetupCorner],
) -> Element<'static, CarSetupMessage> {
    let mut cells = row![corner_table_label_cell(
        comparison_row.key.path.clone(),
        false,
    )]
    .spacing(0)
    .align_y(Vertical::Center)
    .width(Length::Fill);
    for corner in columns {
        cells = cells
            .push(rule::vertical(1).style(separator_style))
            .push(corner_table_value_cell(
                comparison_row.values[corner.index()].clone(),
                false,
            ));
    }

    cells.into()
}

fn corner_table_label_cell(label: String, header: bool) -> Element<'static, CarSetupMessage> {
    let mut label = text(label).size(if header { LABEL_SIZE } else { TABLE_BODY_SIZE });
    if header {
        label = label
            .font(typography::SANS_SEMIBOLD)
            .style(secondary_text_style);
    } else {
        label = label.font(typography::SANS);
    }

    container(label)
        .padding(table_cell_padding())
        .width(Length::FillPortion(CORNER_LABEL_PORTION))
        .into()
}

fn corner_table_value_cell(
    value: Option<String>,
    header: bool,
) -> Element<'static, CarSetupMessage> {
    let missing = value.is_none();
    let mut value = text(value.unwrap_or_else(|| "--".to_owned())).size(if header {
        LABEL_SIZE
    } else {
        TABLE_BODY_SIZE
    });
    if header {
        value = value
            .font(typography::SANS_SEMIBOLD)
            .style(secondary_text_style);
    } else {
        if missing {
            value = value.font(typography::SANS).style(secondary_text_style);
        } else {
            value = value.font(typography::SANS_SEMIBOLD);
        }
    }

    container(value)
        .padding(table_cell_padding())
        .width(Length::FillPortion(CORNER_VALUE_PORTION))
        .align_x(Horizontal::Right)
        .into()
}

fn table_cell_padding() -> Padding {
    Padding {
        top: TABLE_CELL_VERTICAL_PADDING,
        right: TABLE_CELL_HORIZONTAL_PADDING,
        bottom: TABLE_CELL_VERTICAL_PADDING,
        left: TABLE_CELL_HORIZONTAL_PADDING,
    }
}

fn setup_group_row<'a>(label: &'a str, depth: usize) -> Element<'a, CarSetupMessage> {
    container(
        text(label)
            .size(TABLE_BODY_SIZE)
            .font(typography::SANS_SEMIBOLD),
    )
    .padding(Padding {
        top: 4.0,
        right: ROW_HORIZONTAL_PADDING,
        bottom: 4.0,
        left: ROW_HORIZONTAL_PADDING + depth.min(MAX_VISUAL_INDENT_DEPTH) as f32 * NESTED_INDENT,
    })
    .width(Length::Fill)
    .into()
}

fn setup_value_row<'a>(
    label: &'a str,
    value: &'a str,
    depth: usize,
) -> Element<'a, CarSetupMessage> {
    container(
        row![
            text(label)
                .size(BODY_SIZE)
                .font(typography::SANS)
                .style(secondary_text_style)
                .width(Length::FillPortion(2)),
            text(value)
                .size(BODY_SIZE)
                .font(typography::SANS_SEMIBOLD)
                .width(Length::FillPortion(3))
                .align_x(Horizontal::Right),
        ]
        .spacing(12)
        .align_y(Vertical::Center),
    )
    .padding(row_padding(depth))
    .width(Length::Fill)
    .into()
}

fn status_content(data: &SetupViewData) -> Element<'_, CarSetupMessage> {
    match &data.status {
        SetupDataStatus::Waiting => empty_state(
            "No setup data yet",
            "Connect to a live telemetry source or load an IBT recording to inspect the player car setup.",
        ),
        SetupDataStatus::Missing => empty_state(
            "Setup unavailable",
            "This session does not publish CarSetup data. It can be absent while spectating, replaying, or changing sessions.",
        ),
        SetupDataStatus::ParseError(error) => empty_state(
            "Setup could not be read",
            format!("The session information is not valid YAML: {error}"),
        ),
        SetupDataStatus::Available => empty_state(
            "No setup values",
            "iRacing published a CarSetup section, but it contains no displayable values.",
        ),
    }
}

fn empty_state<'a>(
    title: impl text::IntoFragment<'a>,
    detail: impl text::IntoFragment<'a>,
) -> Element<'a, CarSetupMessage> {
    callout(
        row![
            lucide::info().size(18),
            column![
                text(title)
                    .size(SECTION_TITLE_SIZE)
                    .font(typography::SANS_SEMIBOLD),
                text(detail)
                    .size(BODY_SIZE)
                    .font(typography::SANS)
                    .style(secondary_text_style),
            ]
            .spacing(4),
        ]
        .spacing(10)
        .align_y(Vertical::Top),
    )
    .padding(14)
    .width(Length::Fill)
    .into()
}

fn source_label(session: &Session, live_source: LiveTelemetrySourceInfo) -> String {
    if let Some(info) = session.ibt_info() {
        return info.source.description();
    }

    match session.connection() {
        ConnectionStatus::Disconnected if !live_source.is_available() => {
            format!("{} · unavailable", live_source.display_name())
        },
        ConnectionStatus::Disconnected => format!("{} · disconnected", live_source.display_name()),
        ConnectionStatus::Connecting => format!("{} · waiting", live_source.display_name()),
        ConnectionStatus::Connected => live_source.display_name().to_owned(),
    }
}

fn row_padding(depth: usize) -> Padding {
    Padding {
        top: ROW_VERTICAL_PADDING,
        right: ROW_HORIZONTAL_PADDING,
        bottom: ROW_VERTICAL_PADDING,
        left: ROW_HORIZONTAL_PADDING + depth.min(MAX_VISUAL_INDENT_DEPTH) as f32 * NESTED_INDENT,
    }
}

fn secondary_text_style(theme: &Theme) -> text::Style {
    let mut color = theme.extended_palette().background.base.text;
    color.a *= 0.68;
    text::Style { color: Some(color) }
}

fn separator_style(theme: &Theme) -> rule::Style {
    rule::Style {
        color: theme.extended_palette().background.weaker.color,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

fn setup_update_count(setup: &Value) -> Option<String> {
    let Value::Mapping(mapping) = untagged(setup) else {
        return None;
    };

    mapping.iter().find_map(|(key, value)| {
        (yaml_key(key).eq_ignore_ascii_case("UpdateCount"))
            .then(|| scalar_value(value))
            .flatten()
    })
}

fn setup_sections(setup: &Value) -> Vec<SetupSection> {
    let setup = untagged(setup);
    let Value::Mapping(mapping) = setup else {
        let mut rows = Vec::new();
        append_labeled_value(&mut rows, "Value".to_owned(), setup, 0);
        return vec![SetupSection {
            key: VALUE_SETUP_CARD_KEY.to_owned(),
            title: "Setup".to_owned(),
            rows,
        }];
    };
    let mut general_rows = Vec::new();
    let mut sections = Vec::new();

    for (key, value) in mapping {
        let raw_key = yaml_key(key);
        if raw_key.eq_ignore_ascii_case("UpdateCount") {
            continue;
        }
        let label = humanize_key(&raw_key);

        if is_collection(value) {
            let mut rows = Vec::new();
            append_collection_contents(&mut rows, value, 0);
            sections.push(SetupSection {
                key: format!("{SETUP_SECTION_CARD_PREFIX}{raw_key}"),
                title: label,
                rows,
            });
        } else {
            append_labeled_value(&mut general_rows, label, value, 0);
        }
    }

    if !general_rows.is_empty() {
        sections.insert(
            0,
            SetupSection {
                key: GENERAL_SETUP_CARD_KEY.to_owned(),
                title: "General".to_owned(),
                rows: general_rows,
            },
        );
    }

    sections
}

fn append_collection_contents(rows: &mut Vec<SetupRow>, value: &Value, depth: usize) {
    match untagged(value) {
        Value::Mapping(mapping) => {
            for (key, value) in mapping {
                append_labeled_value(rows, humanize_key(&yaml_key(key)), value, depth);
            }
        },
        Value::Sequence(sequence) => {
            for (index, value) in sequence.iter().enumerate() {
                append_labeled_value(rows, format!("Item {}", index + 1), value, depth);
            }
        },
        value => append_labeled_value(rows, "Value".to_owned(), value, depth),
    }
}

fn append_labeled_value(rows: &mut Vec<SetupRow>, label: String, value: &Value, depth: usize) {
    let value = untagged(value);
    if let Some(value) = scalar_value(value) {
        rows.push(SetupRow::Value {
            label,
            value,
            depth,
        });
        return;
    }

    if matches!(value, Value::Mapping(mapping) if mapping.is_empty()) {
        rows.push(SetupRow::Value {
            label,
            value: "--".to_owned(),
            depth,
        });
        return;
    }

    if let Value::Sequence(sequence) = value
        && sequence.iter().all(|value| scalar_value(value).is_some())
    {
        let value = if sequence.is_empty() {
            "--".to_owned()
        } else {
            sequence
                .iter()
                .filter_map(scalar_value)
                .collect::<Vec<_>>()
                .join(", ")
        };
        rows.push(SetupRow::Value {
            label,
            value,
            depth,
        });
        return;
    }

    rows.push(SetupRow::Group { label, depth });
    append_collection_contents(rows, value, depth + 1);
}

fn untagged(mut value: &Value) -> &Value {
    while let Value::Tagged(tagged) = value {
        value = &tagged.value;
    }
    value
}

fn is_collection(value: &Value) -> bool {
    matches!(untagged(value), Value::Mapping(_) | Value::Sequence(_))
}

fn scalar_value(value: &Value) -> Option<String> {
    match untagged(value) {
        Value::Null => Some("--".to_owned()),
        Value::Bool(value) => Some(if *value { "Yes" } else { "No" }.to_owned()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(non_blank(value).unwrap_or("--").to_owned()),
        Value::Sequence(_) | Value::Mapping(_) | Value::Tagged(_) => None,
    }
}

fn yaml_key(value: &Value) -> String {
    scalar_value(value).unwrap_or_else(|| "Value".to_owned())
}

fn humanize_key(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len() + 4);

    for (index, character) in characters.iter().copied().enumerate() {
        if matches!(character, '_' | '-') {
            if !output.ends_with(' ') {
                output.push(' ');
            }
            continue;
        }
        let previous = index.checked_sub(1).and_then(|index| characters.get(index));
        let next = characters.get(index + 1);
        let starts_word = character.is_uppercase()
            && index > 0
            && previous.is_some_and(|previous| {
                previous.is_lowercase()
                    || previous.is_ascii_digit()
                    || (previous.is_uppercase() && next.is_some_and(|next| next.is_lowercase()))
            });
        if starts_word && !output.ends_with(' ') {
            output.push(' ');
        }
        output.push(character);
    }

    output.trim().to_owned()
}

fn first_non_blank<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .find_map(non_blank)
        .map(str::to_owned)
}

fn non_blank(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use chiaro_irsdk::SessionInfo;

    use super::{
        ComparisonFieldKey, MAX_VISUAL_INDENT_DEPTH, NESTED_INDENT, ROW_HORIZONTAL_PADDING,
        SetupContentBlock, SetupCorner, SetupDataStatus, SetupRow, SetupViewData, humanize_key,
        row_padding, setup_content_blocks, setup_corner, setup_corner_fields, setup_sections,
    };

    fn parse(yaml: &str) -> chiaro_irsdk::SessionInfoDocument {
        SessionInfo {
            update_count: 1,
            yaml: yaml.to_owned(),
            raw: yaml.as_bytes().to_vec(),
        }
        .parse()
        .expect("valid session info fixture")
    }

    fn section_value<'a>(section: &'a super::SetupSection, label: &str) -> &'a str {
        section
            .rows
            .iter()
            .find_map(|row| match row {
                SetupRow::Value {
                    label: row_label,
                    value,
                    ..
                } if row_label == label => Some(value.as_str()),
                SetupRow::Group { .. } | SetupRow::Value { .. } => None,
            })
            .expect("section value")
    }

    #[test]
    fn extracts_player_setup_summary_and_car_specific_sections() {
        let document = parse(
            r#"
WeekendInfo:
 WeekendOptions:
  IsFixedSetup: 0
DriverInfo:
 DriverCarIdx: 2
 DriverSetupName: nordschleife_race.sto
 DriverSetupLoadTypeName: disk
 DriverSetupIsModified: 1
 DriverSetupPassedTech: 1
 Drivers:
  - CarIdx: 2
    CarScreenName: Porsche 911 GT3 R (992)
CarSetup:
 UpdateCount: 11
 TiresAero:
  LeftFront:
   ColdPressure: 155 kPa
   TreadRemaining: [100 %, 99 %, 98 %]
 Chassis:
  FrontARB: 3
"#,
        );
        let data = SetupViewData::from_document(&document);

        assert_eq!(data.status, SetupDataStatus::Available);
        assert_eq!(data.car_name.as_deref(), Some("Porsche 911 GT3 R (992)"));
        assert_eq!(data.setup_name.as_deref(), Some("nordschleife_race.sto"));
        assert_eq!(data.load_type.as_deref(), Some("disk"));
        assert_eq!(data.modified, Some(true));
        assert_eq!(data.passed_tech, Some(true));
        assert_eq!(data.fixed_setup, Some(false));
        assert_eq!(data.update_count.as_deref(), Some("11"));
        assert_eq!(
            data.sections
                .iter()
                .map(|section| section.title.as_str())
                .collect::<Vec<_>>(),
            ["Tires Aero", "Chassis"]
        );
        assert_eq!(
            data.sections
                .iter()
                .map(|section| section.key.as_str())
                .collect::<Vec<_>>(),
            ["setup:section:TiresAero", "setup:section:Chassis"]
        );
        assert!(data.sections[0].rows.iter().any(|row| matches!(
            row,
            SetupRow::Value { label, value, depth: 1 }
                if label == "Cold Pressure" && value == "155 kPa"
        )));
        assert!(data.sections[0].rows.iter().any(|row| matches!(
            row,
            SetupRow::Value { label, value, depth: 1 }
                if label == "Tread Remaining" && value == "100 %, 99 %, 98 %"
        )));
    }

    #[test]
    fn extracts_compact_vehicle_specifications_and_regulations_without_car_setup() {
        let data = SetupViewData::from_document(&parse(
            r#"
WeekendInfo:
 WeekendOptions:
  IsFixedSetup: 1
DriverInfo:
 DriverCarIdx: 4
 DriverCarIsElectric: 0
 DriverCarIdleRPM: 4000.0
 DriverCarRedLine: 13000.0
 DriverCarEngCylinderCount: 6
 DriverCarFuelKgPerLtr: 0.750
 DriverCarFuelMaxLtr: 146.667
 DriverCarMaxFuelPct: 1.0
 DriverCarGearNumForward: 8
 DriverCarGearNeutral: 1
 DriverCarGearReverse: 1
 DriverCarSLFirstRPM: 10560.0
 DriverCarSLShiftRPM: 11057.0
 DriverCarSLLastRPM: 11473.0
 DriverCarSLBlinkRPM: 11629.0
 DriverCarVersion: 2026.01.01.01
 DriverTires:
  - TireIndex: 0
    TireCompoundType: Dry
  - TireIndex: 1
    TireCompoundType: Wet
 Drivers:
  - CarIdx: 4
    CarPath: testcar
    CarClassShortName: GT3
    CarClassMaxFuelPct: 0.750 %
    CarClassWeightPenalty: 10.000 kg
    CarClassPowerAdjust: -2.000 %
    CarClassDryTireSetLimit: 3 %
"#,
        ));

        assert_eq!(data.status, SetupDataStatus::Missing);
        assert_eq!(data.car_path.as_deref(), Some("testcar"));
        assert_eq!(data.vehicle_sections.len(), 2);
        let specifications = &data.vehicle_sections[0];
        assert_eq!(specifications.key, "vehicle:specifications");
        assert_eq!(specifications.title, "Vehicle specifications");
        assert_eq!(section_value(specifications, "Version"), "2026.01.01.01");
        assert_eq!(
            section_value(specifications, "Powertrain"),
            "Combustion · 6 cyl · 4000–13000 rpm"
        );
        assert_eq!(
            section_value(specifications, "Transmission"),
            "8-speed · N · R"
        );
        assert_eq!(
            section_value(specifications, "Shift lights"),
            "F 10560 · S 11057 · L 11473 · B 11629 rpm"
        );
        assert_eq!(
            section_value(specifications, "Fuel system"),
            "146.7 L · 0.750 kg/L"
        );
        assert_eq!(
            section_value(specifications, "Tyre compounds"),
            "0 Dry · 1 Wet"
        );

        let regulations = &data.vehicle_sections[1];
        assert_eq!(regulations.key, "vehicle:regulations");
        assert_eq!(regulations.title, "Regulations");
        assert_eq!(section_value(regulations, "Class"), "GT3");
        assert_eq!(section_value(regulations, "Setup rules"), "Fixed setup");
        assert_eq!(
            section_value(regulations, "Fuel allowance"),
            "110.0 L · 75.0%"
        );
        assert_eq!(section_value(regulations, "Weight penalty"), "10.000 kg");
        assert_eq!(section_value(regulations, "Power adjustment"), "-2.000 %");
        assert_eq!(section_value(regulations, "Dry tyre set limit"), "3");
    }

    #[test]
    fn missing_car_setup_is_distinct_from_invalid_session_information() {
        let data = SetupViewData::from_document(&parse("WeekendInfo: {}"));
        assert_eq!(data.status, SetupDataStatus::Missing);
        assert!(data.sections.is_empty());

        let invalid = SetupViewData::parse_error("unexpected token");
        assert_eq!(
            invalid.status,
            SetupDataStatus::ParseError("unexpected token".to_owned())
        );
    }

    #[test]
    fn player_setup_does_not_fall_back_to_an_unmatched_driver() {
        let data = SetupViewData::from_document(&parse(
            r#"
DriverInfo:
 DriverCarIdx: 99
 Drivers:
  - CarIdx: -1
    CarScreenName: Pace car
CarSetup: {}
"#,
        ));

        assert_eq!(data.car_name, None);
    }

    #[test]
    fn top_level_scalars_are_kept_but_update_count_is_only_metadata() {
        let setup: serde_yaml_ng::Value = serde_yaml_ng::from_str(
            r#"
UpdateCount: 7
FuelLevel: 80 L
BrakeBias: 51.2 %
"#,
        )
        .expect("valid setup fixture");
        let sections = setup_sections(&setup);

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].key, "setup:general");
        assert_eq!(sections[0].title, "General");
        assert_eq!(sections[0].rows.len(), 2);
        assert!(sections[0].rows.iter().all(|row| !matches!(
            row,
            SetupRow::Value { label, .. } if label == "Update Count"
        )));
    }

    #[test]
    fn recognizes_only_explicit_corner_group_aliases() {
        assert_eq!(setup_corner("LF"), Some(SetupCorner::LeftFront));
        assert_eq!(setup_corner("front-left"), Some(SetupCorner::LeftFront));
        assert_eq!(setup_corner("Right Front"), Some(SetupCorner::RightFront));
        assert_eq!(
            setup_corner("rear_left_corner"),
            Some(SetupCorner::LeftRear)
        );
        assert_eq!(setup_corner("RearRight"), Some(SetupCorner::RightRear));
        assert_eq!(setup_corner("LeftFrontPressure"), None);
    }

    #[test]
    fn consecutive_corner_groups_become_a_canonical_comparison_in_place() {
        let setup: serde_yaml_ng::Value = serde_yaml_ng::from_str(
            r#"
TiresAero:
 AeroBalance: 1.5 %
 RightRear:
  ColdPressure: 154 kPa
  Damper:
   Bump: "--"
 LeftFront:
  ColdPressure: 155 kPa
  Damper:
   Bump: 5 clicks
 RightFront:
  ColdPressure: 156 kPa
  Damper:
   Rebound: 7 clicks
 LeftRear:
  ColdPressure: 153 kPa
  Damper:
   Bump: 6 clicks
 RearWing: 6
"#,
        )
        .expect("valid corner comparison fixture");
        let sections = setup_sections(&setup);
        let blocks = setup_content_blocks(&sections[0]);

        assert_eq!(blocks.len(), 3);
        assert!(matches!(
            &blocks[0],
            SetupContentBlock::Rows(rows)
                if matches!(rows.as_slice(), [SetupRow::Value { label, .. }]
                    if label == "Aero Balance")
        ));
        let SetupContentBlock::CornerComparison(comparison) = &blocks[1] else {
            panic!("middle block should be the corner comparison");
        };
        assert_eq!(comparison.columns, SetupCorner::ALL);
        assert_eq!(
            comparison
                .rows
                .iter()
                .map(|row| row.key.path.as_str())
                .collect::<Vec<_>>(),
            ["Cold Pressure", "Damper · Bump", "Damper · Rebound"]
        );
        assert_eq!(
            comparison.rows[0].values,
            [
                Some("155 kPa".to_owned()),
                Some("156 kPa".to_owned()),
                Some("153 kPa".to_owned()),
                Some("154 kPa".to_owned()),
            ]
        );
        assert_eq!(
            comparison.rows[1].values[SetupCorner::RightRear.index()],
            Some("--".to_owned())
        );
        assert_eq!(
            comparison.rows[2].values[SetupCorner::LeftFront.index()],
            None
        );
        assert!(matches!(
            &blocks[2],
            SetupContentBlock::Rows(rows)
                if matches!(rows.as_slice(), [SetupRow::Value { label, .. }]
                    if label == "Rear Wing")
        ));
    }

    #[test]
    fn corner_field_keys_keep_nested_paths_and_duplicate_occurrences_distinct() {
        let rows = [
            SetupRow::Group {
                label: "Damper".to_owned(),
                depth: 1,
            },
            SetupRow::Value {
                label: "Bump".to_owned(),
                value: "4".to_owned(),
                depth: 2,
            },
            SetupRow::Value {
                label: "Bump".to_owned(),
                value: "5".to_owned(),
                depth: 2,
            },
            SetupRow::Value {
                label: "Bump".to_owned(),
                value: "6".to_owned(),
                depth: 1,
            },
        ];
        let fields = setup_corner_fields(&rows).expect("valid nested fields");

        assert_eq!(
            fields.iter().map(|(key, _)| key).collect::<Vec<_>>(),
            [
                &ComparisonFieldKey {
                    path: "Damper · Bump".to_owned(),
                    occurrence: 0,
                },
                &ComparisonFieldKey {
                    path: "Damper · Bump".to_owned(),
                    occurrence: 1,
                },
                &ComparisonFieldKey {
                    path: "Bump".to_owned(),
                    occurrence: 0,
                },
            ]
        );
    }

    #[test]
    fn unsafe_corner_shapes_fall_back_to_the_original_rows() {
        for yaml in [
            r#"
Section:
 LeftFront:
  Pressure: 155 kPa
"#,
            r#"
Section:
 LeftFront:
  Pressure: 155 kPa
 LF:
  Pressure: 156 kPa
"#,
            r#"
Section:
 LeftFront:
  Pressure: 155 kPa
 Aero: 4
 RightFront:
  Pressure: 156 kPa
"#,
            r#"
Section:
 LeftFront:
  Pressure: 155 kPa
 RightFront:
  Camber: -3 deg
"#,
        ] {
            let setup: serde_yaml_ng::Value =
                serde_yaml_ng::from_str(yaml).expect("valid fallback fixture");
            let sections = setup_sections(&setup);
            let blocks = setup_content_blocks(&sections[0]);

            assert!(
                blocks
                    .iter()
                    .all(|block| matches!(block, SetupContentBlock::Rows(_)))
            );
        }
    }

    #[test]
    fn sdk_keys_are_humanized_without_breaking_acronyms() {
        assert_eq!(humanize_key("ColdPressure"), "Cold Pressure");
        assert_eq!(humanize_key("FrontARB"), "Front ARB");
        assert_eq!(humanize_key("LF_TirePressure"), "LF Tire Pressure");
    }

    #[test]
    fn empty_nested_collections_render_an_explicit_placeholder() {
        let setup: serde_yaml_ng::Value = serde_yaml_ng::from_str(
            r#"
Section:
 EmptyMap: {}
 EmptyList: []
"#,
        )
        .expect("valid setup fixture");
        let sections = setup_sections(&setup);

        assert_eq!(sections[0].rows.len(), 2);
        assert!(sections[0].rows.iter().all(|row| matches!(
            row,
            SetupRow::Value { value, .. } if value == "--"
        )));
    }

    #[test]
    fn visual_indentation_is_capped_for_deep_car_specific_data() {
        assert_eq!(
            row_padding(usize::MAX).left,
            ROW_HORIZONTAL_PADDING + MAX_VISUAL_INDENT_DEPTH as f32 * NESTED_INDENT
        );
    }
}
