use std::fmt;

use serde::{
    Deserialize, Deserializer,
    de::{self, Visitor},
};

pub(super) fn deserialize_vec_or_default<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// An integer-backed boolean used by the iRacing session YAML.
///
/// iRacing normally publishes `0` or `1`. The raw integer remains available
/// so a future value does not make the complete document fail to parse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SdkBool(i32);

impl SdkBool {
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }

    pub const fn raw(self) -> i32 {
        self.0
    }

    pub const fn as_bool(self) -> Option<bool> {
        match self.0 {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for SdkBool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SdkBoolVisitor;

        impl Visitor<'_> for SdkBoolVisitor {
            type Value = SdkBool;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an iRacing integer-backed boolean")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                Ok(SdkBool(i32::from(value)))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                i32::try_from(value)
                    .map(SdkBool)
                    .map_err(|_| E::custom(format_args!("boolean value {value} is outside i32")))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                i32::try_from(value)
                    .map(SdkBool)
                    .map_err(|_| E::custom(format_args!("boolean value {value} is outside i32")))
            }
        }

        deserializer.deserialize_any(SdkBoolVisitor)
    }
}

/// A scalar whose representation legitimately varies between iRacing modes.
///
/// Examples include numeric lap limits versus `unlimited`, and colors which
/// can be hexadecimal integers or textual sentinel values.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum SessionScalar {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
}

impl SessionScalar {
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            Self::Float(_) | Self::Boolean(_) | Self::String(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            Self::Integer(_) | Self::Float(_) | Self::Boolean(_) => None,
        }
    }
}
