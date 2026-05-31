use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::crp::CrpPacket;

pub type ConnTx = mpsc::UnboundedSender<Bytes>;

#[derive(Clone, Default)]
pub struct ConnectionRegistry {
    inner: Arc<DashMap<String, ConnTx>>,
}

impl ConnectionRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register(&self, player_id: String, tx: ConnTx) {
        self.inner.insert(player_id, tx);
    }

    pub fn deregister(&self, player_id: &str) {
        self.inner.remove(player_id);
    }

    pub fn send_to(&self, player_id: &str, data: Bytes) -> bool {
        if let Some(tx) = self.inner.get(player_id) {
            tx.send(data).is_ok()
        } else {
            false
        }
    }

    pub fn broadcast(&self, data: Bytes) {
        self.inner.retain(|_, tx| tx.send(data.clone()).is_ok());
    }
}

#[async_trait]
pub trait CubeAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn start(&self, inbound: mpsc::Sender<CrpPacket>) -> Result<ConnectionRegistry>;
    async fn stop(&self) -> Result<()>;
}
