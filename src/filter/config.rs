use serde::Deserialize;
use ton_block::MsgAddressInt;
use ton_types::UInt256;

use crate::types::MessageType;
use super::utils::deserialize_from_str;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FilterType {
    /// Filter by contract ABI
    Contract {
        /// Contract name, must be unique
        name: String,
        /// Path to contract ABI
        abi_path: String,
    },
    /// Filter messages with empty body
    NativeTransfer,
    /// Pass all messages
    AnyMessage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    /// Contract name, must be unique
    pub name: String,
    /// Path to contract abi
    pub abi_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterRecord {
    #[serde(rename = "type")]
    pub filter_type: FilterType,
    pub entries: Vec<FilterEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterConfig {
    pub message_filters: Vec<FilterRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AddressOrCodeHash {
    #[serde(deserialize_with = "deserialize_from_str")]
    Address(MsgAddressInt),
    #[serde(deserialize_with = "deserialize_from_str")]
    CodeHash(UInt256),
}

impl From<MsgAddressInt> for AddressOrCodeHash {
    fn from(address: MsgAddressInt) -> Self {
        AddressOrCodeHash::Address(address)
    }
}

impl From<UInt256> for AddressOrCodeHash {
    fn from(code_hash: UInt256) -> Self {
        AddressOrCodeHash::CodeHash(code_hash)
    }
}

impl AddressOrCodeHash {
    pub fn match_address(&self, other: &MsgAddressInt) -> bool {
        match self {
            Self::Address(address) => address == other,
            Self::CodeHash(_) => false,
        }
    }

    pub fn match_code_hash(&self, other: &UInt256) -> bool {
        match self {
            Self::Address(_) => false,
            Self::CodeHash(hash) => hash == other,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterEntry {
    /// Custom name for a filter
    pub name: String,
    /// Message source by address or code hash
    pub sender: Option<AddressOrCodeHash>,
    /// Message destination by address or code hash
    pub receiver: Option<AddressOrCodeHash>,
    /// Array of messages to match
    pub message: Option<MessageFilter>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageFilter {
    #[serde(rename = "name")]
    pub message_name: String,
    #[serde(rename = "type")]
    pub message_type: MessageType,
}

impl PartialEq for Contract {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Contract {}

impl std::hash::Hash for Contract {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
