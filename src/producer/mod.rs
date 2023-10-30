use std::{net::SocketAddr, io, io::Write};

use anyhow::Result;
use serde::Deserialize;
use tokio::sync::broadcast::{channel, Sender};

use self::http2::start_producer_service;

mod http2;

#[derive(Debug, Clone)]
pub struct Producer {
    pub transport: Transport,
    inner: TransportInner,
}

type TransportData = Vec<u8>;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Transport {
    Http2 {
        capacity: usize,
        listen_address: Option<SocketAddr>,
    },
    Stdio,
}

#[derive(Debug, Clone)]
enum TransportInner {
    Http2 {
        messages: Sender<TransportData>,
    },
    Stdio,
}

impl Producer {
    pub fn new(transport: Transport) -> Result<Self> {
        match transport {
            Transport::Http2 { capacity, listen_address } => {
                let listen_address = listen_address.unwrap_or(SocketAddr::from(([127, 0, 0, 1], 3000)));
                let (messages_tx, messages_rx) = channel(capacity);
                start_producer_service(messages_rx, listen_address);
                Ok(Producer {
                    transport,
                    inner: TransportInner::Http2 { messages: messages_tx }
                })
            },
            Transport::Stdio => Ok(Producer {
                transport,
                inner: TransportInner::Stdio,
            }),
        }
    }

    pub async fn send_data(&self, data: TransportData) -> Result<()> {
        match &self.inner {
            TransportInner::Http2 { messages: tx } => tx.send(data)
                .map(|_count| ())
                .map_err(Into::into),
            TransportInner::Stdio => self.send_data_sync(data),
        }
    }

    pub fn send_data_sync(&self, data: TransportData) -> Result<()> {
        match self.inner {
            TransportInner::Http2 { messages: _ } => unimplemented!("Http producer does not support blocking send"),
            TransportInner::Stdio => {
                static PREFIX: &[u8] = ("-----\n").as_bytes();
                static POSTFIX: &[u8] = ("\n-----\n").as_bytes();

                let mut output = PREFIX.to_vec();
                output.extend(data);
                output.extend_from_slice(POSTFIX);

                io::stdout().write_all(&output)?;
                Ok(())
            },
        }
    }
}
