//! Glimesh date serialization. Glimesh uses a strange date format,
//! use this module with `#[serde(with = ...)]` to (de)serialize dates in Glimesh format.

use chrono::{DateTime, TimeZone, Utc};
use serde::{de, Deserialize, Deserializer, Serializer};

const FORMAT: &str = "%FT%T";

/// Serialize date in Glimesh format
pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = format!("{}", date.format(FORMAT));
    serializer.serialize_str(&s)
}

/// Deserialize date in Glimesh format
pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Utc.datetime_from_str(&s, FORMAT).map_err(de::Error::custom)
}
