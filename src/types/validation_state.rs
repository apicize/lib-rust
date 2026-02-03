use bitflags::bitflags;
use std::collections::HashMap;

bitflags! {
    /// Information reflecting current execution and definition state of a request or group, should not be stored
    #[derive(Copy, Clone, Default, PartialEq, Debug)]
    pub struct ValidationState: u8 {
        const WARNING = 0b00000001;
        const ERROR   = 0b00000010;
    }
}

impl serde::Serialize for ValidationState {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Serialize as a plain number for TypeScript compatibility
        serializer.serialize_u8(self.bits())
    }
}

impl<'de> serde::Deserialize<'de> for ValidationState {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct ValidationStateVisitor;

        impl<'de> Visitor<'de> for ValidationStateVisitor {
            type Value = ValidationState;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    "a ValidationState as either a number or an object with 'bits' field",
                )
            }

            // Handle plain number from TypeScript (e.g., 3)
            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                ValidationState::from_bits(value as u8)
                    .ok_or_else(|| E::custom(format!("invalid ValidationState bits: {}", value)))
            }

            // Handle i64 for negative numbers (shouldn't happen, but be safe)
            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value < 0 {
                    return Err(E::custom(format!(
                        "ValidationState cannot be negative: {}",
                        value
                    )));
                }
                self.visit_u64(value as u64)
            }

            // Handle object format from Rust (e.g., {"bits": 3})
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut bits: Option<u8> = None;
                while let Some(key) = map.next_key::<String>()? {
                    if key == "bits" {
                        bits = Some(map.next_value()?);
                    } else {
                        // Skip unknown fields
                        map.next_value::<serde::de::IgnoredAny>()?;
                    }
                }

                let bits = bits.ok_or_else(|| de::Error::missing_field("bits"))?;
                ValidationState::from_bits(bits).ok_or_else(|| {
                    de::Error::custom(format!("invalid ValidationState bits: {}", bits))
                })
            }
        }

        deserializer.deserialize_any(ValidationStateVisitor)
    }
}

impl ValidationState {
    pub fn from(warnings: &Option<Vec<String>>, errors: &Option<HashMap<String, String>>) -> Self {
        let mut new_value: ValidationState = ValidationState::empty();
        if warnings.as_ref().is_some_and(|w| !w.is_empty()) {
            new_value |= ValidationState::WARNING;
        }
        if errors.as_ref().is_some_and(|w| !w.is_empty()) {
            new_value |= ValidationState::ERROR;
        }
        new_value
    }
}

/// Trait to describe and update validation status
pub trait Validated {
    /// Return state
    fn get_validation_state(&self) -> ValidationState;

    /// Retrieve validation warnings
    fn get_validation_warnings(&self) -> &Option<Vec<String>>;

    /// Set validation warnings
    fn set_validation_warnings(&mut self, warnings: Option<Vec<String>>);

    /// Retrieve validation errors by property name
    fn get_validation_errors(&self) -> &Option<HashMap<String, String>>;

    /// Set validation errors
    fn set_validation_errors(&mut self, errors: Option<HashMap<String, String>>);
}

/// Add a named error to list of validation errors
pub fn add_validation_error(errors: &mut Option<HashMap<String, String>>, name: &str, error: &str) {
    match errors {
        Some(errs) => {
            errs.insert(name.to_string(), error.to_string());
        }
        None => {
            *errors = Some(HashMap::from([(name.to_string(), error.to_string())]));
        }
    }
}

/// Remove a named error from a list of validation errors
pub fn remove_validation_error(errors: &mut Option<HashMap<String, String>>, name: &str) {
    if let Some(errs) = errors {
        errs.remove(name);
        if errs.is_empty() {
            *errors = None;
        }
    }
}
