use tokio::sync::mpsc;

use crate::{
    time::now_millis,
    types::{AppConfig, Bar1m, Kline, Pair, Timeframe},
};

/// Primary facade for cross-module communication.
/// Starts with a single ws -> engine channel and can grow with more channels later.
#[derive(Debug)]
pub struct HistoryBus {
    history_tx: HistoryTx,
    history_rx: HistoryRx,
}

impl HistoryBus {
    pub fn builder() -> HistoryBusBuilder {
        HistoryBusBuilder::default()
    }

    /// Split the bus into channel handles for the websocket producer and the engine consumer.
    pub fn into_engine(self) -> (HistoryTx, HistoryRx) {
        (self.history_tx, self.history_rx)
    }

    /// Clone-able sender for the websocket side; helpful when spawning multiple tasks.
    pub fn history_sender(&self) -> HistoryTx {
        self.history_tx.clone()
    }
}

#[derive(Debug)]
pub struct HistoryBusBuilder {
    history_capacity: usize,
}

impl Default for HistoryBusBuilder {
    fn default() -> Self {
        Self {
            history_capacity: DEFAULT_CHANNEL_CAPACITY,
        }
    }
}

impl HistoryBusBuilder {
    pub fn history_capacity(mut self, capacity: usize) -> Self {
        self.history_capacity = capacity.max(1);
        self
    }

    pub fn build(self) -> HistoryBus {
        let (history_tx, history_rx) = mpsc::channel(self.history_capacity);

        HistoryBus {
            history_tx: HistoryTx(history_tx),
            history_rx: HistoryRx(history_rx),
        }
    }
}

const DEFAULT_CHANNEL_CAPACITY: usize = 1_024;

/// Newtype wrapper so senders for different channels can't be mixed up.
#[derive(Clone, Debug)]
pub struct HistoryTx(mpsc::Sender<HistoryMessage>);

impl HistoryTx {
    pub async fn send(
        &self,
        message: HistoryMessage,
    ) -> Result<(), mpsc::error::SendError<HistoryMessage>> {
        self.0.send(message).await
    }
}

#[derive(Debug)]
pub struct HistoryRx(mpsc::Receiver<HistoryMessage>);

impl HistoryRx {
    pub async fn recv(&mut self) -> Option<HistoryMessage> {
        self.0.recv().await
    }

    pub fn into_inner(self) -> mpsc::Receiver<HistoryMessage> {
        self.0
    }
}

/// Messages flowing from websocket ingestion into the engine.
#[derive(Clone, Debug)]
pub enum HistoryMessage {
    WarmUp(WarmUpEvent),
    Config(AppConfig),
}

#[derive(Clone, Debug)]
pub struct WarmUpEvent {
    pub pair: Pair,
    pub start_ts: i64,
}

impl WarmUpEvent {
    pub fn new(pair: Pair) -> Self {
        Self {
            pair,
            start_ts: now_millis(),
        }
    }
}
