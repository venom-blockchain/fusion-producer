use std::str::FromStr;

use serde::Deserialize;

pub fn deserialize_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where 
    D: serde::Deserializer<'de>,
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    FromStr::from_str(&s).map_err(serde::de::Error::custom)
}
