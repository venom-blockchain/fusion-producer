use anyhow::Result;
use prost::Message;

use crate::types::{MessageType, SerializeMessage};

use ton_types::serialize_toc;
use ton_block::{CommonMsgInfo, Serializable, MsgAddressIntOrNone};

mod bindings {
    // Generated protobuf bindings
    include!(concat!(env!("OUT_DIR"), "/data_producer.rs"));
}

impl From<MessageType> for bindings::MessageType {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::InternalInbound => Self::InternalInbound,
            MessageType::InternalOutbound => Self::InternalOutbound,
            MessageType::ExternalInbound => Self::ExternalInbound,
            MessageType::ExternalOutbound => Self::ExternalOutbound,
        }
    }
}

impl TryFrom<SerializeMessage> for bindings::Message {
    type Error = anyhow::Error;

    fn try_from(msg: SerializeMessage) -> Result<Self, Self::Error> {
        let cell = msg.message.body().unwrap_or_default().into_cell();

        let message_header = match msg.message.header() {
            CommonMsgInfo::IntMsgInfo(header) =>
                bindings::message::MessageHeader::Internal (
                    bindings::InternalHeader {
                        bounce: header.bounce,
                        bounced: header.bounced,
                        ihr_disabled: header.ihr_disabled,
                        src: match header.src {
                            MsgAddressIntOrNone::Some(ref msg) => msg.write_to_bytes()?,
                            MsgAddressIntOrNone::None => Default::default()
                        },
                        dst: header.dst.write_to_bytes()?,
                        value: header.value.grams.as_u128().write_to_bytes()?,
                        ihr_fee: header.ihr_fee.as_u128().write_to_bytes()?,
                        fwd_fee: header.fwd_fee.as_u128().write_to_bytes()?,
                        created_at: header.created_at.as_u32(),
                        created_lt: header.created_lt,
                    }
                ),
            CommonMsgInfo::ExtInMsgInfo(header) =>
                bindings::message::MessageHeader::ExtInbound(
                    bindings::ExternalInboundHeader {
                        dst: header.dst.write_to_bytes()?
                    }
                ),
            CommonMsgInfo::ExtOutMsgInfo(header) =>
                bindings::message::MessageHeader::ExtOutbound(
                    bindings::ExternalOutboudHeader {
                        src: match header.src {
                            MsgAddressIntOrNone::Some(ref msg) => msg.write_to_bytes()?,
                            MsgAddressIntOrNone::None => Default::default()
                        },
                        created_at: header.created_at.as_u32(),
                        created_lt: header.created_lt,
                    }
                ),
        };

        Ok(Self {
            id: msg.message_hash.into_vec(),
            body_boc: serialize_toc(&cell)?,
            message_type: bindings::MessageType::from(msg.message_type).into(),
            block_id: msg.block_id.into_vec(),
            transaction_id: msg.transaction_id.into_vec(),
            transaction_timestamp: msg.transaction_timestamp,
            index_in_transaction: msg.index_in_transaction.into(),
            contract_name: msg.contract_name,
            filter_name: msg.filter_name,
            message_header: Some(message_header)
        })
    }
}

pub fn serialize_message(message: SerializeMessage) -> Result<Vec<u8>> {
    let message: bindings::Message = message.try_into()?;
    Ok(message.encode_length_delimited_to_vec())
}
