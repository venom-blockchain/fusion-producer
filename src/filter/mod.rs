use crate::types::FilteredMessage;

use self::{
    config::{AddressOrCodeHash, FilterEntry},
    parser::get_parsers,
};
use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime};
use ton_block::{MsgAddressInt, Transaction};
use ton_indexer::utils::ShardStateStuff;
use ton_types::UInt256;

pub mod config;
mod parser;
mod utils;

pub use parser::init_parsers;

/// Read state and check account's code hash
fn match_code_hash(
    state: &ShardStateStuff,
    filter_hash: &UInt256,
    account: &MsgAddressInt,
) -> Result<bool> {
    let shard_accounts = state.state().read_accounts()?;
    let Some(account) = shard_accounts.account(&account.address())? else {
        tracing::trace!(
            "match_code_hash: account not found in the shard: {}",
            state.shard()
        );
        return Ok(false);
    };
    let account = account.read_account()?;
    Ok(account
        .get_code_hash()
        .map(|account_hash| account_hash == filter_hash)
        .unwrap_or(false))
}

/// Match the filter with an account
fn match_account_filter(
    state: Option<&ShardStateStuff>,
    filter: Option<&AddressOrCodeHash>,
    value: Option<&MsgAddressInt>,
) -> bool {
    match (filter, value) {
        // Check address
        (Some(AddressOrCodeHash::Address(address)), Some(account)) => address == account,
        // Check code hash
        (Some(AddressOrCodeHash::CodeHash(filter_hash)), Some(account)) => match state {
            Some(state) => match_code_hash(state, filter_hash, account).unwrap_or_else(|err| {
                tracing::error!("Error during match_code_hash: {}", err);
                false
            }),
            None => {
                tracing::error!("Filter has no state to match the code hash");
                false
            }
        },
        // No account -> no match
        (Some(_), None) => false,
        // No filter -> passthrough
        (None, _) => true,
    }
}

/// Check sender, recipient and event data with filter
fn match_filter(
    state: Option<&ShardStateStuff>,
    filter: &FilterEntry,
    src: Option<&MsgAddressInt>,
    dst: Option<&MsgAddressInt>,
    ext: &FilteredMessage,
) -> bool {
    // Match sender and recipient
    let src_match = match_account_filter(state, filter.sender.as_ref(), src);
    let dst_match = match_account_filter(state, filter.receiver.as_ref(), dst);
    // Match abi messages
    let messages_filter = &filter.message;
    let event_match = match messages_filter {
        Some(filter) => filter.message_name == ext.name && filter.message_type == ext.message_type,
        None => true
    };
    src_match && dst_match && event_match
}

/// Filters transaction by source, destination and/or abi action name
pub fn filter_transaction(
    tx: Transaction,
    state: Option<&ShardStateStuff>,
    start_date: NaiveDate,
) -> Vec<FilteredMessage> {
    let mut filtered = vec![];
    let tx_now = NaiveDateTime::from_timestamp_opt(tx.now.into(), 0);
    if tx_now.is_none() || tx_now.unwrap().date() < start_date {
        return vec![];
    }
    for parser in get_parsers().iter() {
        if let Ok(extracted) = parser.inner_parser.parse(&tx) {
            let mut extracted = extracted.into_iter().filter_map(|ext| {
                let (src, dst) = (ext.message.src_ref(), ext.message.dst_ref());
                // find a first filter match
                let match_filter = parser.filters.iter()
                    .find(|filter| match_filter(state, filter, src, dst, &ext));
                // fill parser and filter names in the 
                match_filter.map(|filter| {
                    FilteredMessage {
                        contract_name: parser.name.clone(),
                        filter_name: filter.name.clone(),
                        ..ext
                    }
                })
            });
            filtered.extend(&mut extracted);
        }
    }
    filtered
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, sync::Once};

    use chrono::NaiveDate;
    use ton_block::{Deserializable, MsgAddressInt, Transaction};
    use ton_types::UInt256;

    use crate::types::MessageType;

    use super::{
        config::{FilterType, FilterEntry, FilterConfig, MessageFilter, FilterRecord},
        parser::init_parsers, filter_transaction,
    };

    static TEST_INIT: Once = Once::new();

    fn test_filter_config(src: Option<MsgAddressInt>, dst: Option<MsgAddressInt>) -> FilterConfig {
        let contract = FilterType::Contract {
            name: "TokenWallet".to_string(),
            abi_path: "./test/abi/TokenWallet.abi.json".to_string(),
        };
        let contract_filter = FilterEntry {
            name: "tip3 transfer".to_string(),
            sender: src.map(Into::into),
            receiver: dst.clone().map(Into::into),
            message: Some(MessageFilter {
                message_name: "transfer".to_string(),
                message_type: MessageType::InternalInbound,
            }),
        };
        let native_transfer_filter = FilterEntry {
            name: "native trasnfer".to_string(),
            sender: dst.map(Into::into),
            receiver: None,
            message: None,
        };
        FilterConfig {
            message_filters: Vec::from([
                FilterRecord {
                    filter_type: contract,
                    entries: vec![contract_filter]
                },
                FilterRecord {
                    filter_type: FilterType::NativeTransfer,
                    entries: vec![native_transfer_filter],
                }
            ]),
        }
    }

    fn transfer_token_tx() -> Transaction {
        Transaction::construct_from_base64(
            "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"
        ).unwrap()
    }

    fn init() {
        TEST_INIT.call_once(|| {
            let sender = MsgAddressInt::from_str("0:1ef42a3c649061ba446f2d5ae5219380573c78de3541fe67c742ead0cae68d0d").unwrap();
            let receiver = MsgAddressInt::from_str("0:e6f7da94405c55c9fb14b5be6b8f91bba1be76e678900ecb418499bfe37ada05").unwrap();
            let filter_config = test_filter_config(Some(sender), Some(receiver));
            init_parsers(filter_config).unwrap();
        });
    }

    #[test]
    fn test_token_transfer_filter() {
        init();
        // Tip3 token transfer
        let tx = transfer_token_tx();
        let message_hash = UInt256::from_str("3b1c0c89be14e92f4d9465911b2ac28ce5588f1616994b7a2e94da50d6e22fa4").unwrap();
        let start_date = NaiveDate::from_ymd_opt(2023, 09, 1).unwrap();

        let filtered = filter_transaction(tx, None, start_date);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].message_hash, message_hash);
    }

    #[test]
    fn test_random_tx_skip() {
        init();
        // Tip3 token transfer
        let tx = Transaction::construct_from_base64("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").unwrap();
        let start_date = NaiveDate::from_ymd_opt(2023, 09, 1).unwrap();

        let filtered = filter_transaction(tx, None, start_date);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_late_date() {
        init();
        // Tip3 token transfer
        let tx = transfer_token_tx();
        let start_date = NaiveDate::from_ymd_opt(2023, 09, 20).unwrap();

        let filtered = filter_transaction(tx, None, start_date);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_native_transfer_filter() {
        init();
        // native coin transfer
        let tx = Transaction::construct_from_base64("te6ccgECDAEAAlwAA7V+b32pRAXFXJ+xS1vmuPkbuhvnbmeJAOy0GEmb/jetoFAAAimeamUMEUZcH4ZeMycxqFO+Qtx1wKHL1ZnFvEX2BNOxTljTIEwgAAIpnmaUfBZQw21wADRkmWkIBQQBAhcEQQkAQdSKGGSJIhEDAgBtyYDDUEoI0AAAAAAABAACAAAAAi/Kw/8gXD0ilOnrDoFOdOyIzavNfU+KreaCt9HmIQZaQFAVzACeRzeMCfvIAAAAAAAAAADgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACCctuYHxy64icPHxb/ZoNfCbM4wf3G5cnqnU+oqPab2wX9w0CH75dIy0g1PN/p++fOBmCKgxwYAmH0PbeVmK2UXQ4CAeAIBgEB3wcAr0gBze+1KIC4q5P2KWt81x8jd0N87czxIB2Wgwkzf8b1tAsAM5srDrbAJr3vUfScydyZm/JmFwE+AlDnlOoZFCIsyxdPhgngBgII2AAARTPNTKGEyhhtrkABsWgBEYWcwWlbdPqMPu1crIumVsKzoJK22anTJ/x2cIL+s0UAOb32pRAXFXJ+xS1vmuPkbuhvnbmeJAOy0GEmb/jetoFQBB1IoAYEDxQAAEUzzQ+YlMoYbazACQFrZ6C5XwAAAAAAAAAAAAAAAABO1QSAEtouobjOT15DoVZ0K56Wx16JPRKQZRf+Bw5iyamCaFTwCgFDgBnNlYdbYBNe96j6TmTuTM35MwuAnwEoc8p1DIoRFmWLqAsAAA==").unwrap();
        let message_hash = UInt256::from_str("4a81042d202c35cc123015bd6d1656ff1eab66674b2f6368bd9ded8670829bca").unwrap();
        let start_date = NaiveDate::from_ymd_opt(2023, 09, 1).unwrap();

        let filtered = filter_transaction(tx, None, start_date);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].message_hash, message_hash);
    }
}
