use serde::Deserialize;

use super::{SessionInfoExtra, value::deserialize_vec_or_default};

/// Camera groups available for replay and spectator control.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CameraInfo {
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub groups: Vec<CameraGroup>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// A named camera group such as Cockpit, Scenic, or TV1.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct CameraGroup {
    pub group_num: Option<i32>,
    pub group_name: Option<String>,
    pub is_scenic: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_vec_or_default")]
    pub cameras: Vec<Camera>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}

/// One selectable camera in a group.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Camera {
    pub camera_num: Option<i32>,
    pub camera_name: Option<String>,
    #[serde(flatten)]
    pub extra: SessionInfoExtra,
}
