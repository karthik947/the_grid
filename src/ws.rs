use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use log::warn;
use serde::Deserialize;
use serde::de;
use tokio::pin;
use tokio::sync::mpsc::Receiver;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::{
    error::{GlobalError, Result, WsError},
    message_bus::{EngineMessage, EngineTx, KlineEvent, RebootEvent, WsMessage, WsRx},
    types::{AppConfig, Bar1m, Pair, Timeframe},
};

const BINANCE_WS_BASE: &str = "wss://stream.binance.com:9443/stream?streams=";
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct WsClient {
    rx: Receiver<WsMessage>,
    config: Option<AppConfig>,
    engine_tx: EngineTx,
    reconnect_delay: Duration,
}

impl WsClient {
    pub fn new(rx: WsRx, engine_tx: EngineTx) -> Self {
        Self {
            rx: rx.into_inner(),
            config: None,
            engine_tx,
            reconnect_delay: RECONNECT_DELAY,
        }
    }

    pub fn update_config(&mut self, config: AppConfig) {
        self.config = Some(config);
    }

    pub fn with_reconnect_delay(mut self, delay: Duration) -> Self {
        self.reconnect_delay = delay;
        self
    }

    pub async fn run(mut self) -> Result<()> {
        let mut sent_start_reboot = false;

        loop {
            let config = match self.config.clone() {
                Some(cfg) => cfg,
                None => match self.rx.recv().await {
                    Some(WsMessage::Config(cfg)) => {
                        self.config = Some(cfg.clone());
                        cfg
                    }
                    None => return Ok(()),
                },
            };

            if !sent_start_reboot {
                self.send_reboot("ws starting").await?;
                sent_start_reboot = true;
            }

            let engine_tx = self.engine_tx.clone();
            let config_for_stream = config.clone();
            let stream_fut = WsClient::stream_once(engine_tx, &config_for_stream);
            pin!(stream_fut);
            let restart = tokio::select! {
                res = &mut stream_fut => Restart::Stream(res),
                msg = self.rx.recv() => match msg {
                    Some(WsMessage::Config(cfg)) => {
                        self.config = Some(cfg);
                        Restart::ConfigUpdate
                    }
                    None => Restart::ChannelClosed,
                },
            };

            match restart {
                Restart::Stream(Ok(())) => {
                    warn!("websocket closed; scheduling reboot");
                    self.send_reboot("ws closed").await?;
                }
                Restart::Stream(Err(err)) => {
                    warn!("websocket error: {err}; scheduling reboot");
                    self.send_reboot(format!("ws error: {err}")).await?;
                }
                Restart::ConfigUpdate => {
                    // Config change: restart without notifying engine.
                }
                Restart::ChannelClosed => return Ok(()),
            }

            sleep(self.reconnect_delay).await;
        }
    }

    async fn stream_once(engine_tx: EngineTx, config: &AppConfig) -> Result<()> {
        let url = build_stream_url(config)?;
        let (mut socket, _) = connect_async(url).await.map_err(WsError::from)?;

        while let Some(msg) = socket.next().await {
            match msg.map_err(WsError::from)? {
                Message::Text(text) => WsClient::handle_payload(&engine_tx, &text).await?,
                Message::Binary(bin) => match String::from_utf8(bin.to_vec()) {
                    Ok(text) => WsClient::handle_payload(&engine_tx, &text).await?,
                    Err(err) => warn!("non-utf8 binary message: {err}"),
                },
                Message::Ping(payload) => socket
                    .send(Message::Pong(payload))
                    .await
                    .map_err(WsError::from)?,
                Message::Pong(_) => (),
                Message::Close(reason) => {
                    warn!("websocket closed: {:?}", reason);
                    break;
                }
                Message::Frame(_) => (),
            }
        }

        Ok(())
    }

    async fn handle_payload(engine_tx: &EngineTx, text: &str) -> Result<()> {
        match parse_kline(text) {
            Ok(Some(event)) => engine_tx
                .send(EngineMessage::Kline(event))
                .await
                .map_err(|e| GlobalError::Other(format!("failed to send kline: {e}"))),
            Ok(None) => Ok(()),
            Err(err) => {
                warn!("failed to parse kline: {err}");
                Ok(())
            }
        }
    }

    async fn send_reboot(&self, reason: impl Into<String>) -> Result<()> {
        self.engine_tx
            .send(EngineMessage::Reboot(RebootEvent {
                reason: reason.into(),
            }))
            .await
            .map_err(|e| GlobalError::Other(format!("failed to send reboot: {e}")))
    }
}

fn build_stream_url(config: &AppConfig) -> Result<String> {
    let streams = config
        .pairs()
        .iter()
        .map(|pair| format!("{}@kline_1m", pair.to_lowercase()))
        .collect::<Vec<_>>();

    if streams.is_empty() {
        return Err(WsError::EmptyPairs.into());
    }

    Ok(format!("{BINANCE_WS_BASE}{}", streams.join("/")))
}

fn parse_kline(raw: &str) -> Result<Option<KlineEvent>> {
    let envelope: CombinedStream = serde_json::from_str(raw)
        .map_err(|e| GlobalError::Other(format!("deserialize error: {e}")))?;

    let tf = match parse_timeframe(&envelope.data.kline.interval) {
        Some(tf) => tf,
        None => return Ok(None),
    };

    let bar = Bar1m {
        open: envelope.data.kline.open,
        high: envelope.data.kline.high,
        low: envelope.data.kline.low,
        close: envelope.data.kline.close,
        volume: envelope.data.kline.volume,
        open_time: envelope.data.kline.open_time,
        closed: envelope.data.kline.closed,
    };

    Ok(Some(KlineEvent {
        pair: Pair(envelope.data.kline.symbol),
        timeframe: tf,
        bar,
    }))
}

fn parse_timeframe(interval: &str) -> Option<Timeframe> {
    match interval {
        "1m" => Some(Timeframe::M1),
        "5m" => Some(Timeframe::M5),
        "15m" => Some(Timeframe::M15),
        "30m" => Some(Timeframe::M30),
        "1h" => Some(Timeframe::H1),
        "4h" => Some(Timeframe::H4),
        "1d" => Some(Timeframe::D1),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct CombinedStream {
    #[allow(dead_code)]
    stream: Option<String>,
    data: StreamData,
}

#[derive(Debug, Deserialize)]
struct StreamData {
    #[serde(rename = "k")]
    kline: RawKline,
}

#[derive(Debug, Deserialize)]
struct RawKline {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "i")]
    interval: String,
    #[serde(rename = "t")]
    open_time: i64,
    #[serde(rename = "o", deserialize_with = "de_str_f64")]
    open: f64,
    #[serde(rename = "h", deserialize_with = "de_str_f64")]
    high: f64,
    #[serde(rename = "l", deserialize_with = "de_str_f64")]
    low: f64,
    #[serde(rename = "c", deserialize_with = "de_str_f64")]
    close: f64,
    #[serde(rename = "v", deserialize_with = "de_str_f64")]
    volume: f64,
    #[serde(rename = "x")]
    closed: bool,
}

fn de_str_f64<'de, D>(deserializer: D) -> std::result::Result<f64, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<f64>().map_err(de::Error::custom)
}

enum Restart {
    Stream(Result<()>),
    ConfigUpdate,
    ChannelClosed,
}
