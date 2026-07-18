use serde::Deserialize;

use super::{SdkBool, SessionInfoExtra, value::deserialize_vec_or_default};

/// In-sim voice radios and the currently selected radio.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct RadioInfo {
    pub selected_radio_num: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub radios: Vec<Radio>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// One in-sim voice radio.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Radio {
    pub radio_num: Option<i32>,
    pub hop_count: Option<i32>,
    pub num_frequencies: Option<i32>,
    pub tuned_to_frequency_num: Option<i32>,
    pub scanning_is_on: Option<SdkBool>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub frequencies: Vec<RadioFrequency>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// One voice-chat frequency and its access state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct RadioFrequency {
    pub frequency_num: Option<i32>,
    pub frequency_name: Option<String>,
    pub priority: Option<i32>,
    pub car_idx: Option<i32>,
    pub entry_idx: Option<i32>,
    #[serde(rename = "ClubID")]
    pub club_id: Option<i32>,
    pub can_scan: Option<SdkBool>,
    pub can_squawk: Option<SdkBool>,
    pub muted: Option<SdkBool>,
    pub is_mutable: Option<SdkBool>,
    pub is_deletable: Option<SdkBool>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}
