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
