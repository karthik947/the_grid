use std::collections::HashMap;

use log::{info, warn};
use tokio::sync::mpsc::Receiver;

use crate::{
    error::{GlobalError, Result},
    indicators::IndicatorName,
    klinestore,
    message_bus::{EngineMessage, EngineTx, HistoryMessage, HistoryRx, KlineHist, WarmUpEvent},
    time::now_millis,
    types::{AppConfig, Bar1m, Kline, Pair, Timeframe, Timestamp, config},
};

/// Service that listens for history requests and sends warmup data to the engine.
pub struct HistoryService {
    config: Option<AppConfig>,
    rx: Receiver<HistoryMessage>,
    engine_tx: EngineTx,
}

impl HistoryService {
    pub fn new(rx: HistoryRx, engine_tx: EngineTx) -> Self {
        Self {
            config: None,
            rx: rx.into_inner(),
            engine_tx,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        while let Some(message) = self.rx.recv().await {
            match message {
                HistoryMessage::WarmUp(event) => {
                    let Some(config) = self.config.as_ref() else {
                        return Ok(());
                    };
                    let config = config.clone();
                    let engine_tx = self.engine_tx.clone();
                    tokio::spawn(async move {
                        if let Err(err) = WarmupJob::new(config, engine_tx).process(event).await {
                            warn!("warmup processing failed: {err}");
                        }
                    });
                }
                HistoryMessage::Config(config) => {
                    info!("history config updated");
                    self.config = Some(config.clone());
                }
            }
        }

        info!("history receiver closed; shutting down");
        Ok(())
    }
}

struct WarmupJob {
    config: AppConfig,
    engine_tx: EngineTx,
}

impl WarmupJob {
    fn new(config: AppConfig, engine_tx: EngineTx) -> Self {
        Self { config, engine_tx }
    }

    async fn process(&self, event: WarmUpEvent) -> Result<()> {
        let pair = event.pair;
        let start_ts = event.start_ts;

        let rsi_cfg = self.config.indicators().rsi().clone();
        let vol_cfg = self.config.indicators().volatility().clone();

        let base_tfs: Vec<Timeframe> = collect_timeframes(&rsi_cfg, &vol_cfg);
        let base_tf = highest_timeframe(&base_tfs).unwrap_or(Timeframe::M1);
        let now_ms = now_millis();
        let current_1m_start = Timeframe::M1.nearest_ms(now_ms);
        let base_start = current_1m_start
            - (base_tf.window_minutes() + 100) as i64 * Timeframe::M1.window_millis();

        let base_hist_1m = fetch_history(&pair, Timeframe::M1, base_start).await?;

        let mut bundle: Vec<KlineHist> = Vec::new();

        if vol_cfg.enabled() {
            for tf in vol_cfg.timeframes().iter().copied() {
                let now_ms = now_millis();
                let nearest =
                    now_ms - (tf.window_minutes() as i64 + 1) * Timeframe::M1.window_millis();
                let truncated = truncate_from(&base_hist_1m, nearest);
                bundle.push(KlineHist {
                    pair: pair.clone(),
                    indicator: IndicatorName::Volatility,
                    indicator_tf: tf,
                    hist_1m: truncated,
                    hist_tf: Vec::new(),
                });
            }
        }

        if rsi_cfg.enabled() {
            let mut rsi_histories = HashMap::new();

            for tf in rsi_cfg.timeframes().iter().copied() {
                if tf == Timeframe::M1 {
                    let nearest = tf.nearest_ms(start_ts);
                    rsi_histories.insert(tf, truncate_from(&base_hist_1m, nearest));
                } else {
                    let start = tf.nearest_ms(start_ts.saturating_sub(tf.window_millis() * 500));
                    let history = fetch_history(&pair, tf, start).await?;
                    rsi_histories.insert(tf, history);
                }
            }

            for tf in rsi_cfg.timeframes() {
                let nearest = tf.nearest_ms(start_ts);
                let base = truncate_from(&base_hist_1m, nearest);
                let hist_tf = rsi_histories.get(tf).cloned().unwrap();
                bundle.push(KlineHist {
                    pair: pair.clone(),
                    indicator: IndicatorName::Rsi,
                    indicator_tf: *tf,
                    hist_1m: base,
                    hist_tf,
                });
            }
        }

        if !bundle.is_empty() {
            self.send_khist_bundle(bundle).await?;
        }

        Ok(())
    }

    async fn send_khist_bundle(&self, bundle: Vec<KlineHist>) -> Result<()> {
        let message = EngineMessage::KHistBundle(bundle);
        self.engine_tx
            .send(message)
            .await
            .map_err(|e| GlobalError::Other(format!("engine send failed: {e}")))
    }
}

fn collect_timeframes(rsi: &config::RsiConfig, vol: &config::VolatilityConfig) -> Vec<Timeframe> {
    let mut frames = Vec::new();

    if rsi.enabled() {
        frames.extend_from_slice(rsi.timeframes());
    }

    if vol.enabled() {
        frames.extend_from_slice(vol.timeframes());
    }

    frames
}

fn highest_timeframe(timeframes: &[Timeframe]) -> Option<Timeframe> {
    timeframes
        .iter()
        .copied()
        .max_by_key(Timeframe::window_millis)
}

fn truncate_from(bars: &[Bar1m], start_ms: i64) -> Vec<Bar1m> {
    bars.iter()
        .cloned()
        .filter(|bar| bar.open_time >= start_ms)
        .collect()
}

async fn fetch_history(pair: &Pair, tf: Timeframe, start_ms: i64) -> Result<Vec<Kline>> {
    const LIMIT: u16 = 1_000;

    let mut start = start_ms;
    let mut history = Vec::new();
    let window = tf.window_millis();

    loop {
        let mut batch = klinestore::history(pair, tf, Timestamp(start), LIMIT).await?;
        if batch.is_empty() {
            break;
        }

        let next_start = batch
            .last()
            .map(|bar| bar.open_time.saturating_add(window))
            .unwrap_or(start);

        history.append(&mut batch);

        let now = now_millis();
        if next_start >= now || next_start == start {
            break;
        }

        start = next_start;
    }

    Ok(history)
}
