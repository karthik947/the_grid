use tokio::sync::mpsc;

use crate::indicators::IndicatorName;
use crate::types::{AppConfig, Bar1m, Kline, Pair, Timeframe};

/// Primary facade for cross-module communication.
/// Starts with a single ws -> engine channel and can grow with more channels later.
#[derive(Debug)]
pub struct EngineBus {
    engine_tx: EngineTx,
    engine_rx: EngineRx,
}

impl EngineBus {
    pub fn builder() -> EngineBusBuilder {
        EngineBusBuilder::default()
    }

    /// Split the bus into channel handles for the websocket producer and the engine consumer.
    pub fn into_engine(self) -> (EngineTx, EngineRx) {
        (self.engine_tx, self.engine_rx)
    }

    /// Clone-able sender for the websocket side; helpful when spawning multiple tasks.
    pub fn engine_sender(&self) -> EngineTx {
        self.engine_tx.clone()
    }
}

#[derive(Debug)]
pub struct EngineBusBuilder {
    engine_capacity: usize,
}

impl Default for EngineBusBuilder {
    fn default() -> Self {
        Self {
            engine_capacity: DEFAULT_CHANNEL_CAPACITY,
        }
    }
}

impl EngineBusBuilder {
    pub fn engine_capacity(mut self, capacity: usize) -> Self {
        self.engine_capacity = capacity.max(1);
        self
    }

    pub fn build(self) -> EngineBus {
        let (engine_tx, engine_rx) = mpsc::channel(self.engine_capacity);

        EngineBus {
            engine_tx: EngineTx(engine_tx),
            engine_rx: EngineRx(engine_rx),
        }
    }
}

const DEFAULT_CHANNEL_CAPACITY: usize = 1_024;

/// Newtype wrapper so senders for different channels can't be mixed up.
#[derive(Clone, Debug)]
pub struct EngineTx(mpsc::Sender<EngineMessage>);

impl EngineTx {
    pub async fn send(
        &self,
        message: EngineMessage,
    ) -> Result<(), mpsc::error::SendError<EngineMessage>> {
        self.0.send(message).await
    }
}

#[derive(Debug)]
pub struct EngineRx(mpsc::Receiver<EngineMessage>);

impl EngineRx {
    pub async fn recv(&mut self) -> Option<EngineMessage> {
        self.0.recv().await
    }

    pub fn into_inner(self) -> mpsc::Receiver<EngineMessage> {
        self.0
    }
}

/// Messages flowing from websocket ingestion into the engine.
#[derive(Clone, Debug)]
pub enum EngineMessage {
    Kline(KlineEvent),
    Reboot(RebootEvent),
    KHistBundle(Vec<KlineHist>),
    Config(AppConfig),
}

#[derive(Clone, Debug)]
pub struct KlineHist {
    pub pair: Pair,
    pub indicator: IndicatorName,
    pub indicator_tf: Timeframe, // e.g., 5m/1h for the indicator
    pub hist_1m: Vec<Bar1m>,     // 1m history
    pub hist_tf: Vec<Kline>,     // history at indicator TF
}

#[derive(Clone, Debug)]
pub struct KlineEvent {
    pub pair: Pair,
    pub timeframe: Timeframe,
    pub bar: Bar1m,
}

#[derive(Clone, Debug)]
pub struct RebootEvent {
    pub reason: String,
}
