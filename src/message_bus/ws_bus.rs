use tokio::sync::mpsc;

use crate::{
    time::now_millis,
    types::{AppConfig, Bar1m, Kline, Pair, Timeframe},
};

/// Primary facade for cross-module communication.
/// Starts with a single ws -> engine channel and can grow with more channels later.
#[derive(Debug)]
pub struct WsBus {
    ws_tx: WsTx,
    ws_rx: WsRx,
}

impl WsBus {
    pub fn builder() -> WsBusBuilder {
        WsBusBuilder::default()
    }

    /// Split the bus into channel handles for the websocket producer and the engine consumer.
    pub fn into_engine(self) -> (WsTx, WsRx) {
        (self.ws_tx, self.ws_rx)
    }

    /// Clone-able sender for the websocket side; helpful when spawning multiple tasks.
    pub fn ws_sender(&self) -> WsTx {
        self.ws_tx.clone()
    }
}

#[derive(Debug)]
pub struct WsBusBuilder {
    ws_capacity: usize,
}

impl Default for WsBusBuilder {
    fn default() -> Self {
        Self {
            ws_capacity: DEFAULT_CHANNEL_CAPACITY,
        }
    }
}

impl WsBusBuilder {
    pub fn ws_capacity(mut self, capacity: usize) -> Self {
        self.ws_capacity = capacity.max(1);
        self
    }

    pub fn build(self) -> WsBus {
        let (ws_tx, ws_rx) = mpsc::channel(self.ws_capacity);

        WsBus {
            ws_tx: WsTx(ws_tx),
            ws_rx: WsRx(ws_rx),
        }
    }
}

const DEFAULT_CHANNEL_CAPACITY: usize = 1_024;

/// Newtype wrapper so senders for different channels can't be mixed up.
#[derive(Clone, Debug)]
pub struct WsTx(mpsc::Sender<WsMessage>);

impl WsTx {
    pub async fn send(&self, message: WsMessage) -> Result<(), mpsc::error::SendError<WsMessage>> {
        self.0.send(message).await
    }
}

#[derive(Debug)]
pub struct WsRx(mpsc::Receiver<WsMessage>);

impl WsRx {
    pub async fn recv(&mut self) -> Option<WsMessage> {
        self.0.recv().await
    }

    pub fn into_inner(self) -> mpsc::Receiver<WsMessage> {
        self.0
    }
}

/// Messages flowing from websocket ingestion into the engine.
#[derive(Clone, Debug)]
pub enum WsMessage {
    Config(AppConfig),
}
