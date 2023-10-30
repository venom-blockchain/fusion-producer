use std::sync::OnceLock;

use anyhow::{anyhow, Context, Result};
use ton_block::Deserializable;

use crate::types::{FilteredMessage, message_type_from};

use super::config::{FilterConfig, FilterEntry, FilterRecord, FilterType};

static PARSERS: OnceLock<Vec<Parser>> = OnceLock::new();

pub fn get_parsers<'a>() -> &'a Vec<Parser> {
    PARSERS.get().unwrap()
}

#[derive(Debug)]
pub struct Parser {
    pub name: String,
    // Action parameters to filter the events with
    pub filters: Vec<FilterEntry>,
    /// ABI data to parse actions with nekoton transaction parser
    pub inner_parser: InnerParser,
}

impl Parser {
    pub fn new(name: String, filters: Vec<FilterEntry>, inner_parser: InnerParser) -> Self {
        Parser {
            name,
            filters,
            inner_parser,
        }
    }
}

/// Intialize parsers object
pub fn init_parsers(config: FilterConfig) -> Result<()> {
    let v = init_all_parsers(config)?;

    PARSERS
        .set(v)
        .map_err(|_| anyhow!("Unable to initialize parsers and handlers"))
}

/// Construct nekoton parser from abi file
fn get_abi_parser(abi_path: &str) -> Result<InnerParser> {
    let abi_json = std::fs::read_to_string(abi_path)?;
    let abi = ton_abi::Contract::load(&abi_json)?;

    let events = abi.events.into_values();
    let funs = abi.functions.into_values();
    Ok(
        InnerParser::Nekoton(
            nekoton_abi::TransactionParser::builder()
                .function_in_list(funs, false)
                .events_list(events)
                .build_with_external_in()?
        )
    )
}

/// Initialize parsers from config
fn init_all_parsers(config: FilterConfig) -> Result<Vec<Parser>> {
    let mut parsers = vec![];
    for record in config.message_filters.into_iter() {
        let FilterRecord { filter_type, entries } = record;
        let parser = match filter_type {
            FilterType::Contract { name, abi_path } => {
                let inner_parser = get_abi_parser(&abi_path)?;
                Parser::new(
                    name,
                    entries,
                    inner_parser,
                )
            },
            FilterType::NativeTransfer => Parser {
                name: "EmptyMessage".to_string(),
                filters: entries,
                inner_parser: InnerParser::EmptyMessage
            },
            FilterType::AnyMessage => Parser {
                name: "RawMessage".to_string(),
                filters: entries,
                inner_parser: InnerParser::RawBodyMessageParser,
            },
        };
        parsers.push(parser);
    }
    Ok(parsers)
}

#[derive(Debug, Clone)]
pub enum InnerParser {
    Nekoton(nekoton_abi::TransactionParser),
    EmptyMessage,
    RawBodyMessageParser,
}

impl InnerParser {
    pub fn parse<'tx>(&'tx self, tx: &'tx ton_block::Transaction) -> Result<Vec<FilteredMessage>> {
        match self {
            Self::Nekoton(parser) => parser
                .parse(tx)
                .map(|v| v.iter().map(FilteredMessage::from).collect()),
            Self::EmptyMessage => EmptyMessageParser::parse_empty_messages(tx),
            Self::RawBodyMessageParser => RawMessageParser::parse_raw_messages(tx),
        }
    }
}

pub struct EmptyMessageParser {}

impl EmptyMessageParser{
    // Since nekoton skip messages with empty bodies, we need a separate parser
    pub fn parse_empty_messages(tx: &ton_block::Transaction) -> Result<Vec<FilteredMessage>> {
        let mut output = Vec::new();

        let name = "%%EmptyOutMessage%%".to_string(); // An impossible name in ABI
        let mut index_in_transaction = 0;
        tx.out_msgs.iterate_slices(|slice| {
            let message = slice.reference(0)?;
            let message_hash = message.repr_hash();
            let message = ton_block::Message::construct_from_cell(message)?;
            let message_type = message_type_from(message.header(), false);

            if !message.has_body() {
                output.push(
                    FilteredMessage {
                        name: name.clone(),
                        message_hash,
                        message,
                        message_type,
                        tx: tx.clone(),
                        index_in_transaction,
                        contract_name: Default::default(),
                        filter_name: Default::default()
                    }
                );
            }

            index_in_transaction += 1;
            Ok(true)
        })?;

        Ok(output)
    }
}

// Passes any message
pub struct RawMessageParser {}

impl RawMessageParser{
    pub fn parse_raw_messages(tx: &ton_block::Transaction) -> Result<Vec<FilteredMessage>> {
        let mut output = Vec::new();

        let name = "%%RawBodyMessage%%".to_string();  // An impossible name in ABI
        if let Some(message) = &tx.in_msg {
            let message_hash = message.hash();
            let message = message.read_struct().context("Failed reading in msg")?;
            let message_type = message_type_from(message.header(), true);

            output.push(
                FilteredMessage {
                    name: name.clone(),
                    message_hash,
                    message,
                    message_type,
                    tx: tx.clone(),
                    index_in_transaction: 0,
                    contract_name: Default::default(),
                    filter_name: Default::default()
                }
            );
        }

        let mut index_in_transaction = 0;
        tx.out_msgs.iterate_slices(|slice| {
            let message = slice.reference(0)?;
            let message_hash = message.repr_hash();
            let message = ton_block::Message::construct_from_cell(message)?;
            let message_type = message_type_from(message.header(), false);

            output.push(
                FilteredMessage {
                    name: name.clone(),
                    message_hash,
                    message,
                    message_type,
                    tx: tx.clone(),
                    index_in_transaction,
                    contract_name: Default::default(),
                    filter_name: Default::default()
                }
            );

            index_in_transaction += 1;
            Ok(true)
        })?;

        Ok(output)
    }
}
