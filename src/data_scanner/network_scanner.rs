use std::sync::Arc;

use anyhow::{Context, Result};
use everscale_rpc_server::RpcState;
use ton_indexer::utils::*;
use ton_indexer::ProcessBlockContext;

use crate::blocks_handler::*;
use crate::config::*;

pub struct NetworkScanner {
    indexer: Arc<ton_indexer::Engine>,
}

impl NetworkScanner {
    pub async fn new(
        node_settings: NodeConfig,
        global_config: ton_indexer::GlobalConfig,
        handler: Arc<BlocksHandler>,
        rpc_state: Option<Arc<RpcState>>,
    ) -> Result<Arc<Self>> {
        let subscriber: Arc<dyn ton_indexer::Subscriber> = BlocksSubscriber::new(handler, rpc_state)?;
        println!("Indexer staring...");

        let indexer = ton_indexer::Engine::new(
            node_settings
                .build_indexer_config()
                .await
                .context("Failed to build node config")?,
            global_config,
            subscriber,
        )
            .await
            .context("Failed to start node")?;

        // let message_consumer = if let Some(config) = unimplemented!() {
        //     Some(
        //         MessageConsumer::new(indexer.clone(), config)
        //             .context("Failed to create message consumer")?,
        //     )
        // } else {
        //     None
        // };

        Ok(Arc::new(Self {
            indexer,
            /* message_consumer */
        }))
    }

    pub async fn start(self: &Arc<Self>) -> Result<()> {
        self.indexer.start().await?;
        /* if let Some(consumer) = &self.message_consumer {
            consumer.start();
        } */
        Ok(())
    }

    pub fn indexer(&self) -> &Arc<ton_indexer::Engine> {
        &self.indexer
    }
}

struct BlocksSubscriber {
    handler: Arc<BlocksHandler>,
    rpc_state: Option<Arc<RpcState>>
}

impl BlocksSubscriber {
    fn new(handler: Arc<BlocksHandler>, rpc_state: Option<Arc<RpcState>>) -> Result<Arc<Self>> {

        Ok(Arc::new(Self {
            handler,
            rpc_state
        }))
    }
}

impl BlocksSubscriber {
    async fn handle_block(
        &self,
        block_stuff: &BlockStuff,
        shard_state: Option<&ShardStateStuff>,
    ) -> Result<()> {
        if let Some(rpc_state) = &self.rpc_state {
            rpc_state
                .process_block(block_stuff, shard_state)
                .context("Failed to update RPC state")?;
        }

        self.handler
            .handle_block(block_stuff, shard_state)
            .await
            .context("Failed to handle block")
    }
}

#[async_trait::async_trait]
impl ton_indexer::Subscriber for BlocksSubscriber {
    async fn process_block(&self, ctx: ProcessBlockContext<'_>) -> Result<()> {
        self.handle_block(
            ctx.block_stuff(),
            ctx.shard_state_stuff(),
        )
        .await
    }

    async fn process_full_state(&self, state: Arc<ShardStateStuff>) -> Result<()> {
        if let Some(rpc_state) = &self.rpc_state {
            rpc_state
                .process_full_state(state)
                .await
                .context("Failed to update RPC state")?;
        }

        Ok(())
    }

    async fn process_blocks_edge(
        &self,
        _: ton_indexer::ProcessBlocksEdgeContext<'_>,
    ) -> Result<()> {
        if let Some(rpc_state) = &self.rpc_state {
            rpc_state.process_blocks_edge();
        }
        Ok(())
    }
}
