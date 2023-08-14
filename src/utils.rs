use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Deserializer};

pub fn from_hex<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Vec<u8>, D::Error> {
    let hex = String::deserialize(d)?;
    hex::decode(hex).map_err(serde::de::Error::custom)
}

pub fn from_hex_vec<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<Vec<u8>>, D::Error> {
    let vec: Vec<String> = Vec::deserialize(d)?;
    vec.into_iter()
        .map(|h| hex::decode(h).map_err(serde::de::Error::custom))
        .collect()
}

pub fn from_b64<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Vec<u8>, D::Error> {
    let base64 = String::deserialize(d)?;
    general_purpose::STANDARD
        .decode(base64.as_bytes())
        .map_err(serde::de::Error::custom)
}

pub fn from_datetime<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<DateTime<FixedOffset>, D::Error> {
    let time = String::deserialize(d)?;
    DateTime::parse_from_rfc3339(&time).map_err(serde::de::Error::custom)
}

pub fn from_truthy<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<bool, D::Error> {
    let value = u64::deserialize(d)?;
    match value {
        0 => Ok(false),
        _ => Ok(true),
    }
}
