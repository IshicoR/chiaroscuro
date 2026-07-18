use std::{error::Error, fmt, marker::PhantomData};

use crate::{TelemetryFrame, TelemetryValue, VariableType};

/// Whether a typed telemetry key addresses one value or an SDK array.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableShape {
    Scalar,
    Array,
}

impl fmt::Display for VariableShape {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Scalar => "scalar",
            Self::Array => "array",
        })
    }
}

/// An error returned by strict typed access to a telemetry frame.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VariableAccessError {
    Missing {
        name: &'static str,
    },
    TypeMismatch {
        name: &'static str,
        expected: VariableType,
        actual: VariableType,
    },
    ShapeMismatch {
        name: &'static str,
        expected: VariableShape,
        actual_count: usize,
    },
}

impl fmt::Display for VariableAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { name } => {
                write!(formatter, "telemetry variable `{name}` is unavailable")
            },
            Self::TypeMismatch {
                name,
                expected,
                actual,
            } => write!(
                formatter,
                "telemetry variable `{name}` is {actual}, expected {expected}"
            ),
            Self::ShapeMismatch {
                name,
                expected,
                actual_count,
            } => write!(
                formatter,
                "telemetry variable `{name}` has {actual_count} value(s), expected a {expected}"
            ),
        }
    }
}

impl Error for VariableAccessError {}

/// A compile-time name and primitive type for one telemetry value.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ScalarKey<T> {
    name: &'static str,
    marker: PhantomData<fn() -> T>,
}

impl<T> ScalarKey<T> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }
}

impl<T> Clone for ScalarKey<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ScalarKey<T> {}

/// A compile-time name and element type for an array telemetry value.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ArrayKey<T> {
    name: &'static str,
    marker: PhantomData<fn() -> T>,
}

impl<T> ArrayKey<T> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }
}

impl<T> Clone for ArrayKey<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ArrayKey<T> {}

mod private {
    pub trait SealedPrimitive {}
    pub trait SealedKey {}
}

/// SDK primitives supported by typed telemetry keys.
///
/// This trait is sealed; the SDK's six binary primitive representations are
/// the only valid implementations.
pub trait TelemetryPrimitive: private::SealedPrimitive + Copy + 'static {
    const VARIABLE_TYPE: VariableType;

    fn scalar(value: &TelemetryValue) -> Option<Self>;
    fn array(value: &TelemetryValue) -> Option<&[Self]>;
}

macro_rules! primitive {
    ($type:ty, $variable_type:ident, $scalar:ident, $array:ident) => {
        impl private::SealedPrimitive for $type {}

        impl TelemetryPrimitive for $type {
            const VARIABLE_TYPE: VariableType = VariableType::$variable_type;

            fn scalar(value: &TelemetryValue) -> Option<Self> {
                match value {
                    TelemetryValue::$scalar(value) => Some(*value),
                    _ => None,
                }
            }

            fn array(value: &TelemetryValue) -> Option<&[Self]> {
                match value {
                    TelemetryValue::$array(values) => Some(values),
                    _ => None,
                }
            }
        }
    };
}

primitive!(u8, Char, Char, Chars);
primitive!(bool, Bool, Bool, Bools);
primitive!(i32, Int, Int, Ints);
primitive!(u32, BitField, BitField, BitFields);
primitive!(f32, Float, Float, Floats);
primitive!(f64, Double, Double, Doubles);

/// A scalar or array key accepted by [`TelemetryFrame::get`].
pub trait TelemetryKey: private::SealedKey + Copy {
    type Output<'a>
    where
        Self: 'a;

    fn name(self) -> &'static str;
    fn value_type(self) -> VariableType;
    fn shape(self) -> VariableShape;
    fn decode<'a>(self, value: &'a TelemetryValue) -> Option<Self::Output<'a>>;
}

impl<T: TelemetryPrimitive> private::SealedKey for ScalarKey<T> {}

impl<T: TelemetryPrimitive> TelemetryKey for ScalarKey<T> {
    type Output<'a>
        = T
    where
        Self: 'a;

    fn name(self) -> &'static str {
        self.name
    }

    fn value_type(self) -> VariableType {
        T::VARIABLE_TYPE
    }

    fn shape(self) -> VariableShape {
        VariableShape::Scalar
    }

    fn decode(self, value: &TelemetryValue) -> Option<Self::Output<'_>> {
        T::scalar(value)
    }
}

impl<T: TelemetryPrimitive> private::SealedKey for ArrayKey<T> {}

impl<T: TelemetryPrimitive> TelemetryKey for ArrayKey<T> {
    type Output<'a>
        = &'a [T]
    where
        Self: 'a;

    fn name(self) -> &'static str {
        self.name
    }

    fn value_type(self) -> VariableType {
        T::VARIABLE_TYPE
    }

    fn shape(self) -> VariableShape {
        VariableShape::Array
    }

    fn decode<'a>(self, value: &'a TelemetryValue) -> Option<Self::Output<'a>> {
        T::array(value)
    }
}

impl TelemetryFrame {
    /// Reads a required variable using its exact SDK primitive type and shape.
    pub fn get<K: TelemetryKey>(&self, key: K) -> Result<K::Output<'_>, VariableAccessError> {
        self.get_optional(key)?
            .ok_or(VariableAccessError::Missing { name: key.name() })
    }

    /// Reads a typed variable when it is published for the current car/session.
    ///
    /// A missing variable returns `Ok(None)`. A published variable with the
    /// wrong primitive type or scalar/array shape is still an error.
    pub fn get_optional<K: TelemetryKey>(
        &self,
        key: K,
    ) -> Result<Option<K::Output<'_>>, VariableAccessError> {
        let Some((metadata, value)) = self.entry(key.name()) else {
            return Ok(None);
        };

        let expected_type = key.value_type();
        if metadata.value_type != expected_type {
            return Err(VariableAccessError::TypeMismatch {
                name: key.name(),
                expected: expected_type,
                actual: metadata.value_type,
            });
        }

        let expected_shape = key.shape();
        let actual_is_scalar = metadata.count == 1;
        let shape_matches = matches!(expected_shape, VariableShape::Scalar) == actual_is_scalar;
        if !shape_matches {
            return Err(VariableAccessError::ShapeMismatch {
                name: key.name(),
                expected: expected_shape,
                actual_count: metadata.count,
            });
        }

        // TelemetryFrame validates metadata and values together when built.
        Ok(Some(key.decode(value).expect("validated telemetry value")))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{ArrayKey, ScalarKey, VariableAccessError, VariableShape};
    use crate::{TelemetryFrame, TelemetryValue, VariableMetadata, VariableType};

    fn frame() -> TelemetryFrame {
        let definitions: Arc<[VariableMetadata]> = Arc::from([
            definition("Char", VariableType::Char, 1),
            definition("Bool", VariableType::Bool, 1),
            definition("Int", VariableType::Int, 1),
            definition("Bits", VariableType::BitField, 1),
            definition("Float", VariableType::Float, 1),
            definition("Double", VariableType::Double, 1),
            definition("Chars", VariableType::Char, 2),
            definition("Bools", VariableType::Bool, 2),
            definition("Ints", VariableType::Int, 2),
            definition("BitFields", VariableType::BitField, 2),
            definition("Floats", VariableType::Float, 2),
            definition("Doubles", VariableType::Double, 2),
        ]);
        TelemetryFrame::try_new(
            7,
            definitions,
            vec![
                TelemetryValue::Char(65),
                TelemetryValue::Bool(true),
                TelemetryValue::Int(-2),
                TelemetryValue::BitField(0x8000_0001),
                TelemetryValue::Float(1.5),
                TelemetryValue::Double(2.5),
                TelemetryValue::Chars(Box::new([65, 66])),
                TelemetryValue::Bools(Box::new([true, false])),
                TelemetryValue::Ints(Box::new([-2, 3])),
                TelemetryValue::BitFields(Box::new([1, 2])),
                TelemetryValue::Floats(Box::new([1.5, 2.5])),
                TelemetryValue::Doubles(Box::new([2.5, 3.5])),
            ],
        )
        .expect("valid test frame")
    }

    fn definition(name: &str, value_type: VariableType, count: usize) -> VariableMetadata {
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
    fn reads_all_six_scalar_and_array_types() {
        let frame = frame();

        assert_eq!(frame.get(ScalarKey::<u8>::new("Char")), Ok(65));
        assert_eq!(frame.get(ScalarKey::<bool>::new("Bool")), Ok(true));
        assert_eq!(frame.get(ScalarKey::<i32>::new("Int")), Ok(-2));
        assert_eq!(frame.get(ScalarKey::<u32>::new("Bits")), Ok(0x8000_0001));
        assert_eq!(frame.get(ScalarKey::<f32>::new("Float")), Ok(1.5));
        assert_eq!(frame.get(ScalarKey::<f64>::new("Double")), Ok(2.5));
        assert_eq!(frame.get(ArrayKey::<u8>::new("Chars")), Ok(&[65, 66][..]));
        assert_eq!(
            frame.get(ArrayKey::<bool>::new("Bools")),
            Ok(&[true, false][..])
        );
        assert_eq!(frame.get(ArrayKey::<i32>::new("Ints")), Ok(&[-2, 3][..]));
        assert_eq!(
            frame.get(ArrayKey::<u32>::new("BitFields")),
            Ok(&[1, 2][..])
        );
        assert_eq!(
            frame.get(ArrayKey::<f32>::new("Floats")),
            Ok(&[1.5, 2.5][..])
        );
        assert_eq!(
            frame.get(ArrayKey::<f64>::new("Doubles")),
            Ok(&[2.5, 3.5][..])
        );
    }

    #[test]
    fn distinguishes_missing_type_and_shape_errors() {
        let frame = frame();

        assert_eq!(
            frame.get(ScalarKey::<f32>::new("Missing")),
            Err(VariableAccessError::Missing { name: "Missing" })
        );
        assert_eq!(
            frame.get(ScalarKey::<i32>::new("Float")),
            Err(VariableAccessError::TypeMismatch {
                name: "Float",
                expected: VariableType::Int,
                actual: VariableType::Float,
            })
        );
        assert_eq!(
            frame.get(ArrayKey::<f32>::new("Float")),
            Err(VariableAccessError::ShapeMismatch {
                name: "Float",
                expected: VariableShape::Array,
                actual_count: 1,
            })
        );
        assert_eq!(
            frame.get_optional(ScalarKey::<f32>::new("Missing")),
            Ok(None)
        );
    }
}
