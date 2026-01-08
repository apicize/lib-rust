use bitflags::bitflags;
use std::fmt;

bitflags! {
    /// Information reflecting current execution and definition state of a request or group, should not be stored
    #[derive(Clone, Copy, Default, PartialEq, Debug)]
    pub struct ExecutionState: u32 {
        const RUNNING = 0b00000001;
        const SUCCESS = 0b00000010;
        const FAILURE = 0b00000100;
        const ERROR   = 0b00001000;
    }
}

impl serde::Serialize for ExecutionState {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Serialize as a plain number for TypeScript compatibility
        serializer.serialize_u32(self.bits())
    }
}

impl fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "None");
        }

        let mut flags = Vec::new();
        if self.contains(ExecutionState::RUNNING) {
            flags.push("Running");
        }
        if self.contains(ExecutionState::SUCCESS) {
            flags.push("Success");
        }
        if self.contains(ExecutionState::FAILURE) {
            flags.push("Failure");
        }
        if self.contains(ExecutionState::ERROR) {
            flags.push("Error");
        }

        write!(f, "{}", flags.join(" | "))
    }
}

impl<'de> serde::Deserialize<'de> for ExecutionState {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{self, Visitor, MapAccess};

        struct ExecutionStateVisitor;

        impl<'de> Visitor<'de> for ExecutionStateVisitor {
            type Value = ExecutionState;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an ExecutionState as either a number or an object with 'bits' field")
            }

            // Handle plain number from TypeScript (e.g., 10)
            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                ExecutionState::from_bits(value as u32)
                    .ok_or_else(|| E::custom(format!("invalid ExecutionState bits: {}", value)))
            }

            // Handle i64 for negative numbers (shouldn't happen, but be safe)
            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value < 0 {
                    return Err(E::custom(format!("ExecutionState cannot be negative: {}", value)));
                }
                self.visit_u64(value as u64)
            }

            // Handle object format from Rust (e.g., {"bits": 10})
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut bits: Option<u32> = None;
                while let Some(key) = map.next_key::<String>()? {
                    if key == "bits" {
                        bits = Some(map.next_value()?);
                    } else {
                        // Skip unknown fields
                        map.next_value::<serde::de::IgnoredAny>()?;
                    }
                }

                let bits = bits.ok_or_else(|| de::Error::missing_field("bits"))?;
                ExecutionState::from_bits(bits)
                    .ok_or_else(|| de::Error::custom(format!("invalid ExecutionState bits: {}", bits)))
            }
        }

        deserializer.deserialize_any(ExecutionStateVisitor)
    }
}

pub trait Executable {
    fn get_execution_state(&self) -> &ExecutionState;
}