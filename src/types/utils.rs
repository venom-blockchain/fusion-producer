use serde::Serializer;
use ton_block::Message;
use ton_types::UInt256;

pub fn serialize_ton_uint<S>(id: &UInt256, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&id.to_hex_string())
}

pub fn serialize_message_as_display<S>(message: &Message, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&format!("{}", message))
}

