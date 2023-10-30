use std::mem::size_of;

use anyhow::Result;
use serde::Deserialize;

use crate::types::SerializeMessage;

mod protobuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Serializer {
    #[cfg(feature="serialize-protobuf")]
    Protobuf,
    #[cfg(feature="serialize-json")]
    Json,
}

/// Prepend the array with a length
#[cfg(feature="serialize-json")]
pub fn write_json_with_prefix(message: SerializeMessage) -> Result<Vec<u8>> {
    let mut json_vec = serde_json::to_vec(&message)?;
    let len = json_vec.len();
    let mut res = Vec::with_capacity(size_of::<u128>() + len);
    res.extend((len as u32).to_be_bytes());
    res.append(&mut json_vec);
    Ok(res)
}

impl Serializer {
    pub fn serialize_message(&self, message: SerializeMessage) -> Result<Vec<u8>> {
        match self {
            #[cfg(feature="serialize-protobuf")]
            Self::Protobuf => protobuf::serialize_message(message),
            #[cfg(feature="serialize-json")]
            Self::Json => write_json_with_prefix(message),
        }
    }
}
