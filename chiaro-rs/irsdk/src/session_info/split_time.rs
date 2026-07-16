use serde::Deserialize;

use super::{SessionInfoExtra, value::deserialize_vec_or_default};

/// Sector boundaries for the current track layout.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct SplitTimeInfo {
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub sectors: Vec<Sector>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// One sector's start as a fraction of lap distance.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Sector {
    pub sector_num: Option<i32>,
    pub sector_start_pct: Option<f64>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}
