use std::sync::{Arc, OnceLock};

use crate::{
    adapters::binance::BinanceRest,
    error::{GlobalError, Result},
    types::{Kline, Pair, Timeframe, Timestamp},
};

static STORE: OnceLock<Arc<KlineStore>> = OnceLock::new();

/// Simple façade over the Binance REST adapter to fetch klines.
/// Initialized once at startup and accessed via free functions.
#[derive(Clone)]
pub struct KlineStore {
    binance: BinanceRest,
}

impl KlineStore {
    /// Initialize the store once (idempotent). Calling again returns the first instance.
    pub fn init(binance: BinanceRest) -> Arc<Self> {
        STORE.get_or_init(|| Arc::new(Self { binance })).clone()
    }

    fn global() -> Result<Arc<Self>> {
        STORE
            .get()
            .cloned()
            .ok_or_else(|| GlobalError::Other("KlineStore not initialized".into()))
    }

    async fn history_inner(
        &self,
        pair: &Pair,
        tf: Timeframe,
        start: Timestamp,
        limit: u16,
    ) -> Result<Vec<Kline>> {
        self.binance.kline_history(pair, tf, start, limit).await
    }
}

/// Fetch klines using the globally initialized store.
pub async fn history(
    pair: &Pair,
    tf: Timeframe,
    start: Timestamp,
    limit: u16,
) -> Result<Vec<Kline>> {
    let store = KlineStore::global()?;
    store.history_inner(pair, tf, start, limit).await
}
