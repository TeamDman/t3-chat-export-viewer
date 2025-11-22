use chrono::{DateTime, TimeZone, Utc};
use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::Deref;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct T3Timestamp(pub DateTime<Utc>);

impl Deref for T3Timestamp {
    type Target = DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for T3Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for T3Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for T3Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct T3TimestampVisitor;

        impl<'de> serde::de::Visitor<'de> for T3TimestampVisitor {
            type Value = T3Timestamp;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an integer (timestamp in ms) or a string (RFC3339 date)")
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match Utc.timestamp_millis_opt(value) {
                    chrono::LocalResult::Single(dt) => Ok(T3Timestamp(dt)),
                    _ => Err(E::custom(format!("invalid timestamp: {}", value))),
                }
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match Utc.timestamp_millis_opt(value as i64) {
                    chrono::LocalResult::Single(dt) => Ok(T3Timestamp(dt)),
                    _ => Err(E::custom(format!("invalid timestamp: {}", value))),
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match DateTime::parse_from_rfc3339(value) {
                    Ok(dt) => Ok(T3Timestamp(dt.with_timezone(&Utc))),
                    Err(e) => Err(E::custom(format!("invalid date string: {} ({})", value, e))),
                }
            }
        }

        deserializer.deserialize_any(T3TimestampVisitor)
    }
}
