use nekoton_abi::transaction_parser::Extracted;
use serde::{Deserialize, Serialize};
use ton_block::{CommonMsgInfo, Message, Transaction, MessageId, GetRepresentationHash};
use ton_types::UInt256;

mod utils;
use utils::{serialize_ton_uint, serialize_message_as_display};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MessageType {
    InternalInbound,
    InternalOutbound,
    ExternalInbound,
    ExternalOutbound,
}

pub fn message_type_from(msg: &CommonMsgInfo, is_in_message: bool) -> MessageType {
    match msg {
        CommonMsgInfo::IntMsgInfo(_) => if is_in_message {
            MessageType::InternalInbound } else {
                MessageType::InternalOutbound
            },
        CommonMsgInfo::ExtInMsgInfo(_) => MessageType::ExternalInbound,
        CommonMsgInfo::ExtOutMsgInfo(_) => MessageType::ExternalOutbound,
    }
}

#[derive(Debug, Clone)]
pub struct FilteredMessage {
    pub name: String,
    pub message_hash: UInt256,
    pub message: Message,
    pub message_type: MessageType,
    pub tx: Transaction,
    pub index_in_transaction: u16, // The index of the message in the transaction
    pub contract_name: String,
    pub filter_name: String
}

impl<'a> From<&Extracted<'a>> for FilteredMessage {
    fn from(ext: &Extracted<'a>) -> Self {
        let message_type = message_type_from(ext.message.header(), ext.is_in_message);
        Self {
            name: ext.name.to_string(),
            message_hash: ext.message_hash,
            message: ext.message.clone(),
            message_type,
            tx: ext.tx.clone(),
            index_in_transaction: ext.index_in_transaction,
            contract_name: Default::default(),
            filter_name: Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerializeMessage {
    #[serde(serialize_with = "serialize_message_as_display")]
    pub message: Message,
    #[serde(serialize_with = "serialize_ton_uint")]
    pub message_hash: MessageId,
    pub message_type: MessageType,
    #[serde(serialize_with = "serialize_ton_uint")]
    pub block_id: UInt256,
    #[serde(serialize_with = "serialize_ton_uint")]
    pub transaction_id: UInt256,
    pub transaction_timestamp: u32,
    pub index_in_transaction: u16,
    pub contract_name: String,
    pub filter_name: String,
}

impl From<FilteredMessage> for SerializeMessage {
    fn from(msg: FilteredMessage) -> Self {
        let transaction_id = msg.tx.hash().unwrap_or_default();

        SerializeMessage {
            message: msg.message,
            message_hash: msg.message_hash,
            message_type: msg.message_type,
            block_id: Default::default(),
            transaction_id,
            transaction_timestamp: msg.tx.now,
            index_in_transaction: msg.index_in_transaction,
            contract_name: msg.contract_name,
            filter_name: msg.filter_name,
        }
    }
}
