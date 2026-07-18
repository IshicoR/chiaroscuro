use std::{collections::HashMap, error::Error, fmt, sync::Arc};

use crate::TelemetrySample;

/// The primitive type assigned to an iRacing telemetry variable.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableType {
    Char,
    Bool,
    Int,
    BitField,
    Float,
    Double,
}

impl VariableType {
    pub(crate) fn from_raw(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Char),
            1 => Some(Self::Bool),
            2 => Some(Self::Int),
            3 => Some(Self::BitField),
            4 => Some(Self::Float),
            5 => Some(Self::Double),
            _ => None,
        }
    }

    pub(crate) const fn byte_len(self) -> usize {
        match self {
            Self::Char | Self::Bool => 1,
            Self::Int | Self::BitField | Self::Float => 4,
            Self::Double => 8,
        }
    }
}

impl fmt::Display for VariableType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Char => "char",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::BitField => "bit field",
            Self::Float => "float",
            Self::Double => "double",
        })
    }
}

/// Static information describing a telemetry variable for the current session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableMetadata {
    pub name: String,
    pub description: String,
    pub unit: String,
    pub value_type: VariableType,
    pub count: usize,
    pub count_as_time: bool,
}

/// An owned scalar or array value copied from an iRacing telemetry frame.
#[derive(Debug, Clone, PartialEq)]
pub enum TelemetryValue {
    Char(u8),
    Bool(bool),
    Int(i32),
    BitField(u32),
    Float(f32),
    Double(f64),
    Chars(Box<[u8]>),
    Bools(Box<[bool]>),
    Ints(Box<[i32]>),
    BitFields(Box<[u32]>),
    Floats(Box<[f32]>),
    Doubles(Box<[f64]>),
}

impl TelemetryValue {
    pub const fn value_type(&self) -> VariableType {
        match self {
            Self::Char(_) | Self::Chars(_) => VariableType::Char,
            Self::Bool(_) | Self::Bools(_) => VariableType::Bool,
            Self::Int(_) | Self::Ints(_) => VariableType::Int,
            Self::BitField(_) | Self::BitFields(_) => VariableType::BitField,
            Self::Float(_) | Self::Floats(_) => VariableType::Float,
            Self::Double(_) | Self::Doubles(_) => VariableType::Double,
        }
    }

    pub const fn count(&self) -> usize {
        match self {
            Self::Char(_)
            | Self::Bool(_)
            | Self::Int(_)
            | Self::BitField(_)
            | Self::Float(_)
            | Self::Double(_) => 1,
            Self::Chars(values) => values.len(),
            Self::Bools(values) => values.len(),
            Self::Ints(values) => values.len(),
            Self::BitFields(values) => values.len(),
            Self::Floats(values) => values.len(),
            Self::Doubles(values) => values.len(),
        }
    }

    pub const fn is_scalar(&self) -> bool {
        matches!(
            self,
            Self::Char(_)
                | Self::Bool(_)
                | Self::Int(_)
                | Self::BitField(_)
                | Self::Float(_)
                | Self::Double(_)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VariableCatalog {
    metadata: Arc<[VariableMetadata]>,
    by_name: HashMap<String, usize>,
}

impl VariableCatalog {
    pub(crate) fn new(metadata: Arc<[VariableMetadata]>) -> Result<Self, FrameBuildError> {
        let mut by_name = HashMap::with_capacity(metadata.len());

        for (index, variable) in metadata.iter().enumerate() {
            if variable.name.is_empty() {
                return Err(FrameBuildError::EmptyName { index });
            }
            if variable.count == 0 {
                return Err(FrameBuildError::EmptyValue {
                    name: variable.name.clone(),
                });
            }
            if by_name.insert(variable.name.clone(), index).is_some() {
                return Err(FrameBuildError::DuplicateName {
                    name: variable.name.clone(),
                });
            }
        }

        Ok(Self { metadata, by_name })
    }

    pub(crate) fn metadata(&self) -> &[VariableMetadata] {
        &self.metadata
    }

    pub(crate) fn index(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }
}

/// An invalid metadata/value combination supplied to [`TelemetryFrame::try_new`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrameBuildError {
    LengthMismatch {
        metadata: usize,
        values: usize,
    },
    EmptyName {
        index: usize,
    },
    DuplicateName {
        name: String,
    },
    EmptyValue {
        name: String,
    },
    ValueMismatch {
        name: String,
        expected_type: VariableType,
        expected_count: usize,
        actual_type: VariableType,
        actual_count: usize,
    },
}

impl fmt::Display for FrameBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { metadata, values } => write!(
                formatter,
                "telemetry frame has {metadata} variable definitions but {values} values"
            ),
            Self::EmptyName { index } => {
                write!(formatter, "telemetry variable {index} has no name")
            },
            Self::DuplicateName { name } => {
                write!(formatter, "duplicate telemetry variable `{name}`")
            },
            Self::EmptyValue { name } => {
                write!(formatter, "telemetry variable `{name}` has an empty value")
            },
            Self::ValueMismatch {
                name,
                expected_type,
                expected_count,
                actual_type,
                actual_count,
            } => write!(
                formatter,
                "telemetry variable `{name}` expects {expected_count} {expected_type} value(s), but contains {actual_count} {actual_type} value(s)"
            ),
        }
    }
}

impl Error for FrameBuildError {}

/// Every telemetry variable copied from one stable iRacing frame.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryFrame {
    packet_id: i32,
    catalog: Arc<VariableCatalog>,
    values: Box<[TelemetryValue]>,
}

impl TelemetryFrame {
    /// Builds a frame while validating its metadata/value invariants.
    pub fn try_new(
        packet_id: i32,
        variables: impl Into<Arc<[VariableMetadata]>>,
        values: impl Into<Box<[TelemetryValue]>>,
    ) -> Result<Self, FrameBuildError> {
        let catalog = Arc::new(VariableCatalog::new(variables.into())?);
        let values = values.into();
        Self::validate(&catalog, &values)?;

        Ok(Self {
            packet_id,
            catalog,
            values,
        })
    }

    pub(crate) fn from_catalog(
        packet_id: i32,
        catalog: Arc<VariableCatalog>,
        values: Box<[TelemetryValue]>,
    ) -> Result<Self, FrameBuildError> {
        Self::validate(&catalog, &values)?;
        Ok(Self {
            packet_id,
            catalog,
            values,
        })
    }

    fn validate(
        catalog: &VariableCatalog,
        values: &[TelemetryValue],
    ) -> Result<(), FrameBuildError> {
        if catalog.metadata.len() != values.len() {
            return Err(FrameBuildError::LengthMismatch {
                metadata: catalog.metadata.len(),
                values: values.len(),
            });
        }

        for (variable, value) in catalog.metadata.iter().zip(values) {
            let actual_type = value.value_type();
            let actual_count = value.count();
            let shape_matches = (variable.count == 1) == value.is_scalar();
            if variable.value_type != actual_type
                || variable.count != actual_count
                || !shape_matches
            {
                return Err(FrameBuildError::ValueMismatch {
                    name: variable.name.clone(),
                    expected_type: variable.value_type,
                    expected_count: variable.count,
                    actual_type,
                    actual_count,
                });
            }
        }

        Ok(())
    }

    pub const fn packet_id(&self) -> i32 {
        self.packet_id
    }

    pub fn variables(&self) -> &[VariableMetadata] {
        self.catalog.metadata()
    }

    pub fn values(&self) -> &[TelemetryValue] {
        &self.values
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn value(&self, name: &str) -> Option<&TelemetryValue> {
        self.entry(name).map(|(_, value)| value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&VariableMetadata, &TelemetryValue)> {
        self.catalog.metadata.iter().zip(&self.values)
    }

    pub(crate) fn entry(&self, name: &str) -> Option<(&VariableMetadata, &TelemetryValue)> {
        let index = self.catalog.index(name)?;
        Some((self.catalog.metadata.get(index)?, self.values.get(index)?))
    }
}

/// A desktop sample and every raw SDK value produced from the same stable frame.
#[derive(Debug, Clone, PartialEq)]
pub struct TelemetrySnapshot {
    pub sample: TelemetrySample,
    pub frame: TelemetryFrame,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{FrameBuildError, TelemetryFrame, TelemetryValue, VariableMetadata, VariableType};

    fn metadata(name: &str, value_type: VariableType, count: usize) -> VariableMetadata {
        VariableMetadata {
            name: name.to_owned(),
            description: String::new(),
            unit: String::new(),
            value_type,
            count,
            count_as_time: false,
        }
    }

    #[test]
    fn reports_the_type_and_shape_of_values() {
        assert_eq!(TelemetryValue::Float(1.0).value_type(), VariableType::Float);
        assert_eq!(TelemetryValue::Float(1.0).count(), 1);
        assert!(TelemetryValue::Float(1.0).is_scalar());

        let values = TelemetryValue::BitFields(Box::new([1, 2]));
        assert_eq!(values.value_type(), VariableType::BitField);
        assert_eq!(values.count(), 2);
        assert!(!values.is_scalar());
    }

    #[test]
    fn validates_frame_lengths_types_and_shapes() {
        let length_error = TelemetryFrame::try_new(
            1,
            Arc::from([metadata("RPM", VariableType::Float, 1)]),
            Vec::new(),
        );
        assert!(matches!(
            length_error,
            Err(FrameBuildError::LengthMismatch {
                metadata: 1,
                values: 0
            })
        ));

        let type_error = TelemetryFrame::try_new(
            1,
            Arc::from([metadata("RPM", VariableType::Float, 1)]),
            vec![TelemetryValue::Int(6_000)],
        );
        assert!(matches!(
            type_error,
            Err(FrameBuildError::ValueMismatch { .. })
        ));

        let shape_error = TelemetryFrame::try_new(
            1,
            Arc::from([metadata("CarIdxLap", VariableType::Int, 1)]),
            vec![TelemetryValue::Ints(Box::new([1]))],
        );
        assert!(matches!(
            shape_error,
            Err(FrameBuildError::ValueMismatch { .. })
        ));
    }

    #[test]
    fn indexes_values_by_name() {
        let frame = TelemetryFrame::try_new(
            12,
            Arc::from([
                metadata("RPM", VariableType::Float, 1),
                metadata("CarIdxLap", VariableType::Int, 2),
            ]),
            vec![
                TelemetryValue::Float(6_000.0),
                TelemetryValue::Ints(Box::new([4, 5])),
            ],
        )
        .expect("valid frame");

        assert_eq!(frame.packet_id(), 12);
        assert_eq!(frame.len(), 2);
        assert_eq!(frame.value("RPM"), Some(&TelemetryValue::Float(6_000.0)));
        assert!(frame.value("Missing").is_none());
    }
}
