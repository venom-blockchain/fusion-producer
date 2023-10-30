use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::Deserialize;
use ton_block::{BlockIdExt, ShardIdent};
use ton_indexer::utils::BlockStuff;
use ton_types::UInt256;

use crate::blocks_handler::*;

/// Reads a json data about blocks and accounts for testing purposes
pub struct TestScanner {
    handler: Arc<BlocksHandler>,
    filename: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
struct BlockData {
    id: String,
    shard: String,
    workchain_id: i64,
    seq_no: u64,
    file_hash: String,
    boc: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct AccountData {
    pub id: String,
    pub code_hash: String,
    pub init_code_hash: String,
    pub boc: String,
}

#[derive(Debug, Clone, Deserialize)]
struct InnerJson {
    pub blocks: Vec<BlockData>,
    pub accounts: Vec<AccountData>,
}

#[derive(Debug, Clone, Deserialize)]
struct BlocksJson {
    pub data: InnerJson,
}

impl TestScanner {
    pub fn new(handler: Arc<BlocksHandler>, filename: PathBuf) -> Result<Self> {
        Ok(Self { handler, filename })
    }

    pub async fn run(self) -> Result<()> {
        let file = File::open(self.filename)?;
        let reader = BufReader::new(file);
        let block_json: BlocksJson = serde_json::from_reader(reader)?;
        let blocks = block_json.data.blocks;
        let _accounts = block_json.data.accounts;

        for block_data in blocks {
            let block_id = BlockIdExt {
                shard_id: ShardIdent::with_tagged_prefix(
                    block_data.workchain_id as i32,
                    u64::from_str_radix(&block_data.shard, 16)?
                )?,
                seq_no: block_data.seq_no as u32,
                root_hash: UInt256::from_str(&block_data.id)?,
                file_hash: UInt256::from_str(&block_data.file_hash)?,
            };
            let block_boc = base64::decode(&block_data.boc)?;
            let block_stuff = BlockStuff::deserialize(block_id.clone(), &block_boc)?;

            tracing::trace!("Block stuff: {:?}", block_stuff.block());
            if let Err(e) = self
                .handler
                .handle_block(
                    &block_stuff,
                    None
                )
                .await
                .context("Failed to handle block")
            {
                tracing::error!("Failed reading block {block_id}: {e:?}");
            }
        }

        Ok(())
    }
}
