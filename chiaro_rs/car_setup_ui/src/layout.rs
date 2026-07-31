//! Persisted layout values for the Car Setup screen.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarSetupLayoutFlag {
    pub key: String,
    pub value: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CarSetupLayout {
    pub card_order: Vec<String>,
    pub card_collapsed: Vec<CarSetupLayoutFlag>,
}

pub(super) fn normalize_order(keys: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for key in keys {
        if !key.trim().is_empty() && !normalized.contains(key) {
            normalized.push(key.clone());
        }
    }
    normalized
}

pub(super) fn normalize_flags(flags: &[CarSetupLayoutFlag]) -> BTreeMap<String, bool> {
    let mut normalized = BTreeMap::new();
    for flag in flags {
        if !flag.key.trim().is_empty() {
            normalized.entry(flag.key.clone()).or_insert(flag.value);
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::{CarSetupLayoutFlag, normalize_flags, normalize_order};

    #[test]
    fn dynamic_layout_helpers_ignore_empty_and_duplicate_keys() {
        assert_eq!(
            normalize_order(&["summary".into(), String::new(), "summary".into()]),
            ["summary"]
        );
        assert_eq!(
            normalize_flags(&[
                CarSetupLayoutFlag {
                    key: "status".into(),
                    value: true,
                },
                CarSetupLayoutFlag {
                    key: "status".into(),
                    value: false,
                },
            ])
            .get("status"),
            Some(&true)
        );
    }
}
