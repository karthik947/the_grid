use std::collections::{HashMap, HashSet};
use std::time::Duration;

use log::{debug, info};
use tokio::sync::mpsc::Receiver;
use tokio::time::Interval;

use crate::message_bus::{HistoryMessage, HistoryTx, IndicatorValue, UiMessage, UiTx};
use crate::{
    error::{GlobalError, Result},
    indicators::{IndicatorManager, IndicatorName, IndicatorResult},
    message_bus::{EngineMessage, EngineRx, KlineEvent, KlineHist, RebootEvent, WarmUpEvent},
    types::{AppConfig, KlineSource, Pair, Timeframe, config::IndexLookup},
};

/// Core engine that consumes websocket events and maintains indicator state.
pub struct Engine {
    config: Option<AppConfig>,
    rx: Receiver<EngineMessage>,
    ui_tx: UiTx,
    history_tx: HistoryTx,
    warmup_pending: Option<HashSet<Pair>>,
    warmup_done: bool,
    indicators: IndicatorManager,
    pending_results: Vec<(usize, IndicatorValue)>,
    flush_interval: Interval,
}

impl Engine {
    pub fn new(rx: EngineRx, history_tx: HistoryTx, ui_tx: UiTx) -> Self {
        Self {
            config: None,
            rx: rx.into_inner(),
            ui_tx,
            history_tx,
            warmup_pending: None,
            warmup_done: false,
            indicators: IndicatorManager::new(),
            pending_results: Vec::new(),
            flush_interval: tokio::time::interval(Duration::from_secs(2)),
        }
    }
    async fn flush_indicator_results(&mut self) -> Result<()> {
        if self.config.is_none() {
            return Ok(());
        }
        if self.pending_results.is_empty() {
            return Ok(());
        }
        // info!("Results length => {}", self.pending_results.len());
        let batch = std::mem::take(&mut self.pending_results);
        self.ui_tx
            .send(UiMessage::IndicatorResults(batch))
            .await
            .map_err(|e| GlobalError::Other(format!("ui send failed: {e}")))?;
        Ok(())
    }

    fn push_result(
        &mut self,
        index_lookup: &IndexLookup,
        pair: &Pair,
        indicator: IndicatorName,
        timeframe: Timeframe,
        value: f32,
    ) {
        let indicator_key = match indicator {
            IndicatorName::Volatility => crate::types::config::IndicatorKey::Volatility,
            IndicatorName::Rsi => crate::types::config::IndicatorKey::Rsi,
        };

        if let Some(idx) = index_lookup.index(&pair.0, indicator_key, timeframe) {
            let val = match indicator {
                IndicatorName::Volatility => IndicatorValue::Volatility(value),
                IndicatorName::Rsi => IndicatorValue::Rsi(value),
            };
            self.pending_results.push((idx, val));
        }
    }
    async fn send_warmup(&self, pair: Pair) -> Result<()> {
        let warmup = WarmUpEvent::new(pair);
        self.history_tx
            .send(HistoryMessage::WarmUp(warmup))
            .await
            .map_err(|e| GlobalError::Other(format!("history warmup send failed: {e}")))?;
        Ok(())
    }
    pub async fn run(mut self) -> Result<()> {
        loop {
            tokio::select! {
                _ = self.flush_interval.tick() => {
                    self.flush_indicator_results().await?;
                }
                maybe_msg = self.rx.recv() => {
                    let Some(message) = maybe_msg else {
                        info!("engine receiver closed; shutting down");
                        return Ok(());
                    };

                    match message {
                        EngineMessage::Reboot(event) => self.handle_reboot(event),
                        EngineMessage::Kline(event) => {
                            if self.warmup_done {
                                self.handle_kline(event);
                                continue;
                            }

                            let Some(warmup_pending) = self.warmup_pending.as_mut() else {
                                continue;
                            };

                            let should_send = if warmup_pending.remove(&event.pair) {
                                if warmup_pending.is_empty() {
                                    self.warmup_done = true;
                                }
                                true
                            } else {
                                false
                            };

                            if should_send {
                                self.send_warmup(event.pair.clone()).await?;
                            }

                            self.handle_kline(event);
                        }
                        EngineMessage::KHistBundle(event) => self.handle_khist_bundle(event),
                        EngineMessage::Config(config) => {
                            self.config = Some(config.clone());
                            let reboot_event = RebootEvent {
                                reason: "config updated".into(),
                            };
                            self.handle_reboot(reboot_event);
                        }
                    }
                }
            }
        }
    }

    fn handle_reboot(&mut self, event: RebootEvent) {
        self.indicators = IndicatorManager::new();
        self.warmup_pending = if let Some(config) = self.config.as_mut() {
            Some(config.pairs().iter().cloned().map(Pair).collect())
        } else {
            None
        };
        self.warmup_done = false;
        info!("engine reset after reboot: {}", event.reason);
    }

    fn handle_kline(&mut self, event: KlineEvent) {
        // info!("Kline event = {:?}", event);
        let (index_lookup, rsi_enabled, rsi_timeframes, vol_enabled, vol_timeframes) = {
            let Some(config) = self.config.as_ref() else {
                return;
            };
            (
                config.index_lookup().clone(),
                config.indicators().rsi().enabled(),
                config.indicators().rsi().timeframes().to_vec(),
                config.indicators().volatility().enabled(),
                config.indicators().volatility().timeframes().to_vec(),
            )
        };
        let pair = event.pair;

        if rsi_enabled {
            for tf in &rsi_timeframes {
                if let Some(val) = self
                    .indicators
                    .update(&pair, &tf, IndicatorName::Rsi, &event.bar)
                    .into_rsi_value()
                {
                    self.push_result(&index_lookup, &pair, IndicatorName::Rsi, *tf, val);
                }
            }
        }

        if vol_enabled {
            for tf in &vol_timeframes {
                if let Some(val) = self
                    .indicators
                    .update(&pair, &tf, IndicatorName::Volatility, &event.bar)
                    .into_volatility_value()
                {
                    self.push_result(&index_lookup, &pair, IndicatorName::Volatility, *tf, val);
                }
            }
        }
    }
    fn handle_khist_bundle(&mut self, event: Vec<KlineHist>) {
        // todo!("Implement kline history handlers");
        for khist in event {
            self.indicators.update_khist(khist);
        }
    }
}
