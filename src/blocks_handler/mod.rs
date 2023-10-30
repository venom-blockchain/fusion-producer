use anyhow::Result;
use futures_util::future::join_all;
use once_cell::race::OnceBox;
use rustc_hash::FxHashSet;
use ton_block::{Deserializable, HashmapAugType, Serializable};
use ton_indexer::utils::{BlockStuff, ShardStateStuff};
use ton_types::HashmapType;

use crate::{
    serializer::Serializer,
    filter::filter_transaction,
    types::SerializeMessage,
    producer::Producer
};

pub struct BlocksHandler {
    pub serializer: Serializer,
    pub producer: Producer,
}

impl BlocksHandler {
    pub fn new(serializer: Serializer, producer: Producer) -> Result<Self> {
        tracing::debug!("New blocks handle; serializer: {:?}, producer: {:?}", serializer, producer);
        Ok(Self {
            serializer,
            producer,
        })
    }

    pub async fn handle_block(
        &self,
        block_stuff: &BlockStuff,
        shard_state: Option<&ShardStateStuff>
    ) -> Result<()> {
        let block_id = block_stuff.id();
        let block = block_stuff.block();
        let block_extra = block.read_extra()?;

        tracing::trace!("Processing block: {}", block_id);

        // Process transactions
        let mut changed_accounts = FxHashSet::default();
        let mut deleted_accounts = FxHashSet::default();

        let workchain_id = block_id.shard_id.workchain_id();

        block_extra
            .read_account_blocks()?
            .iterate_objects(|account_block| {
                tracing::trace!("Processing account block for: {}", account_block.account_addr().as_hex_string());

                let state_update = account_block.read_state_update()?;

                if state_update.old_hash != state_update.new_hash {
                    if state_update.new_hash == default_account_hash() {
                        deleted_accounts.insert(account_block.account_id().clone());
                    } else {
                        changed_accounts.insert(account_block.account_id().clone());
                    }
                }

                account_block
                    .transactions()
                    .iterate_slices(|_, raw_transaction| {
                        let result = self.transaction(
                            raw_transaction,
                            &block_id.root_hash,
                            workchain_id,
                            shard_state,
                        );
                        if let Err(error) = result {
                            tracing::error!("Transaction handler: {}", error);
                        }
                        Ok(true)
                    })?;

                Ok(true)
            })?;

        Ok(())
    }

    fn transaction(
        &self,
        raw_transaction: ton_types::SliceData,
        block_id: &ton_types::UInt256,
        _workchain_id: i32,
        state: Option<&ShardStateStuff>,
    ) -> Result<()> {
        let cell = raw_transaction.reference(0)?;
        let id = cell.repr_hash();
        let transaction = ton_block::Transaction::construct_from_cell(cell)?;

        tracing::trace!("Transaction handle: {}", id.as_hex_string());

        let serializer = self.serializer.clone();
        let messages = filter_transaction(transaction, state, Default::default());
        tracing::trace!("Filtered {} messages", messages.len());

        let serialized = messages.into_iter()
            .map(|msg| {
                let msg = SerializeMessage {
                    block_id: *block_id,
                    ..msg.into()
                };
                let serialized = serializer.serialize_message(msg);
                if let Err(error) = &serialized {
                    tracing::error!("Serializing message: {}", error);
                }
                serialized.unwrap_or_default()
            })
            .collect::<Vec<_>>();
        tracing::trace!("Serialized {} messages", serialized.len());
        // Send to transport layer
        let producer = self.producer.clone();
        tokio::spawn(async move {
            let futures = serialized
                .into_iter()
                .map(|data| producer.send_data(data));
            for result in join_all(futures).await {
                tracing::trace!("Message data sent");
                if let Err(error) = result {
                    tracing::error!("Sending message data: {}", error);
                }
            }
        });

        Ok(())
    }
}

fn default_account_hash() -> &'static ton_types::UInt256 {
    static HASH: OnceBox<ton_types::UInt256> = OnceBox::new();
    HASH.get_or_init(|| {
        Box::new(
            ton_block::Account::default()
                .serialize()
                .unwrap()
                .repr_hash(),
        )
    })
}
