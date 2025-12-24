use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

use reqwest::Client;
use serde::Deserialize;
use tokio::{sync::Mutex, time::sleep};

use crate::{
    error::{GlobalError, Result},
    types::{Kline, Pair, Timeframe, Timestamp},
};

const BINANCE_API_BASE: &str = "https://api.binance.com";
const BINANCE_RATE_LIMIT_PER_MIN: usize = 1_200;
const BINANCE_MAX_LIMIT: u16 = 1_000;

/// Simple REST adapter for Binance public market data.
/// Uses a small builder for configurability and keeps a local rate limiter.
#[derive(Clone)]
pub struct BinanceRest {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
}

impl BinanceRest {
    pub fn builder() -> BinanceRestBuilder {
        BinanceRestBuilder::default()
    }

    /// Fetch historical klines starting from the provided timestamp.
    pub async fn kline_history(
        &self,
        pair: &Pair,
        timeframe: Timeframe,
        start: Timestamp,
        limit: u16,
    ) -> Result<Vec<Kline>> {
        let limit = limit.clamp(1, BINANCE_MAX_LIMIT);
        self.rate_limiter.acquire().await;

        let interval: BinanceInterval = timeframe.into();
        let url = format!("{}/api/v3/klines", self.base_url);

        let response = self
            .client
            .get(url)
            .query(&[
                ("symbol", pair.0.as_str()),
                ("interval", interval.as_str()),
                ("startTime", &start.0.to_string()),
                ("limit", &limit.to_string()),
            ])
            .send()
            .await
            .map_err(|e| GlobalError::Other(format!("binance request failed: {e}")))?;

        let response = response
            .error_for_status()
            .map_err(|e| GlobalError::Other(format!("binance http error: {e}")))?;

        let body = response
            .text()
            .await
            .map_err(|e| GlobalError::Other(format!("binance response read failed: {e}")))?;

        let klines: Vec<BinanceKline> = serde_json::from_str(&body)
            .map_err(|e| GlobalError::Other(format!("binance response decode failed: {e}")))?;

        klines.into_iter().map(|raw| raw.try_into_kline()).collect()
    }
}

#[derive(Debug)]
pub struct BinanceRestBuilder {
    base_url: String,
    rate_limit_per_minute: usize,
}

impl Default for BinanceRestBuilder {
    fn default() -> Self {
        Self {
            base_url: BINANCE_API_BASE.to_string(),
            rate_limit_per_minute: BINANCE_RATE_LIMIT_PER_MIN,
        }
    }
}

impl BinanceRestBuilder {
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn rate_limit_per_minute(mut self, max: usize) -> Self {
        self.rate_limit_per_minute = max.max(1);
        self
    }

    pub fn build(self) -> BinanceRest {
        BinanceRest {
            client: Client::new(),
            base_url: self.base_url,
            rate_limiter: RateLimiter::per_minute(self.rate_limit_per_minute),
        }
    }
}

/// Newtype to convert local timeframes into Binance intervals.
#[derive(Clone, Debug)]
struct BinanceInterval(String);

impl BinanceInterval {
    fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<Timeframe> for BinanceInterval {
    fn from(tf: Timeframe) -> Self {
        Self(tf.to_string().to_owned())
    }
}

/// Minimal sliding-window rate limiter (max calls per minute).
#[derive(Clone, Debug)]
struct RateLimiter {
    window: Duration,
    max_calls: usize,
    calls: Arc<Mutex<VecDeque<Instant>>>,
}

impl RateLimiter {
    fn per_minute(max_calls: usize) -> Self {
        Self::new(Duration::from_secs(60), max_calls)
    }

    fn new(window: Duration, max_calls: usize) -> Self {
        Self {
            window,
            max_calls,
            calls: Arc::new(Mutex::new(VecDeque::with_capacity(max_calls))),
        }
    }

    async fn acquire(&self) {
        loop {
            let mut calls = self.calls.lock().await;
            let now = Instant::now();

            while let Some(&ts) = calls.front() {
                if now.duration_since(ts) >= self.window {
                    calls.pop_front();
                } else {
                    break;
                }
            }

            if calls.len() < self.max_calls {
                calls.push_back(now);
                return;
            }

            if let Some(&oldest) = calls.front() {
                let wait = self.window.saturating_sub(now.duration_since(oldest));
                drop(calls);
                sleep(wait).await;
            } else {
                // Should never hit because len >= max_calls, but avoid busy spinning.
                drop(calls);
                sleep(Duration::from_millis(10)).await;
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct BinanceKline(
    i64,    // open time
    String, // open
    String, // high
    String, // low
    String, // close
    String, // volume
    i64,    // close time
    String, // quote asset volume
    i64,    // number of trades
    String, // taker buy base asset volume
    String, // taker buy quote asset volume
    String, // ignore
);

impl BinanceKline {
    fn try_into_kline(self) -> Result<Kline> {
        Ok(Kline {
            open: parse_f64(&self.1, "open")?,
            high: parse_f64(&self.2, "high")?,
            low: parse_f64(&self.3, "low")?,
            close: parse_f64(&self.4, "close")?,
            volume: parse_f64(&self.5, "volume")?,
            open_time: self.0,
            closed: true,
        })
    }
}

fn parse_f64(value: &str, field: &str) -> Result<f64> {
    value
        .parse::<f64>()
        .map_err(|e| GlobalError::Other(format!("failed to parse {field}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rate_limiter_respects_window() {
        let limiter = RateLimiter::new(Duration::from_millis(50), 2);
        limiter.acquire().await;
        limiter.acquire().await;

        let start = Instant::now();
        limiter.acquire().await;
        assert!(start.elapsed() >= Duration::from_millis(50));
    }

    #[test]
    fn timeframe_translation_matches_binance() {
        let tf = BinanceInterval::from(Timeframe::M15);
        assert_eq!(tf.as_str(), "15m");
    }

    #[test]
    fn parses_kline_payload() {
        let raw = BinanceKline(
            1_700_000_000_000,
            "1.0".to_string(),
            "2.0".to_string(),
            "0.5".to_string(),
            "1.5".to_string(),
            "10.0".to_string(),
            1_700_000_060_000,
            "0".to_string(),
            0,
            "0".to_string(),
            "0".to_string(),
            "0".to_string(),
        );

        let kline = raw.try_into_kline().expect("should parse");
        assert_eq!(kline.open_time, 1_700_000_000_000);
        assert_eq!(kline.close, 1.5);
        assert!(kline.closed);
    }
}
