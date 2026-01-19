use ratatui::style::Color;
use tokio::sync::mpsc;

/// Primary facade for cross-module communication.
/// Starts with a single ws -> engine channel and can grow with more channels later.
#[derive(Debug)]
pub struct UiBus {
    ui_tx: UiTx,
    ui_rx: UiRx,
}

impl UiBus {
    pub fn builder() -> UiBusBuilder {
        UiBusBuilder::default()
    }

    /// Split the bus into channel handles for the websocket producer and the engine consumer.
    pub fn into_engine(self) -> (UiTx, UiRx) {
        (self.ui_tx, self.ui_rx)
    }

    /// Clone-able sender for the websocket side; helpful when spawning multiple tasks.
    pub fn ui_sender(&self) -> UiTx {
        self.ui_tx.clone()
    }
}

#[derive(Debug)]
pub struct UiBusBuilder {
    ui_capacity: usize,
}

impl Default for UiBusBuilder {
    fn default() -> Self {
        Self {
            ui_capacity: DEFAULT_CHANNEL_CAPACITY,
        }
    }
}

impl UiBusBuilder {
    pub fn ui_capacity(mut self, capacity: usize) -> Self {
        self.ui_capacity = capacity.max(1);
        self
    }

    pub fn build(self) -> UiBus {
        let (ui_tx, ui_rx) = mpsc::channel(self.ui_capacity);

        UiBus {
            ui_tx: UiTx(ui_tx),
            ui_rx: UiRx(ui_rx),
        }
    }
}

const DEFAULT_CHANNEL_CAPACITY: usize = 1_024;

/// Newtype wrapper so senders for different channels can't be mixed up.
#[derive(Clone, Debug)]
pub struct UiTx(mpsc::Sender<UiMessage>);

impl UiTx {
    pub async fn send(&self, message: UiMessage) -> Result<(), mpsc::error::SendError<UiMessage>> {
        self.0.send(message).await
    }
}

#[derive(Debug)]
pub struct UiRx(mpsc::Receiver<UiMessage>);

impl UiRx {
    pub async fn recv(&mut self) -> Option<UiMessage> {
        self.0.recv().await
    }

    pub fn try_recv(&mut self) -> Result<UiMessage, mpsc::error::TryRecvError> {
        self.0.try_recv()
    }

    pub fn into_inner(self) -> mpsc::Receiver<UiMessage> {
        self.0
    }
}

#[derive(Clone, Debug)]
pub enum UiMessage {
    IndicatorResults(Vec<(usize, IndicatorValue)>), // index,value
}

#[derive(Clone, Debug)]
pub enum IndicatorValue {
    Volatility(f32),
    Rsi(f32),
}

#[derive(Clone, Debug)]
pub enum IndicatorThresholds {
    Volatility { threshold: f32 },
    Rsi { oversold: f32, overbought: f32 },
}

#[derive(Clone, Copy, Debug)]
pub struct IndicatorColors {
    pub text: Color,
    pub background: Option<Color>,
}

impl IndicatorValue {
    const POSITIVE_TEXT: Color = Color::Rgb(64, 199, 122);
    const NEGATIVE_TEXT: Color = Color::Rgb(230, 82, 82);
    const POSITIVE_BG: Color = Color::Rgb(33, 178, 125);
    const NEGATIVE_BG: Color = Color::Rgb(186, 64, 117);

    pub fn display(&self) -> String {
        match self {
            IndicatorValue::Volatility(v) => format!("{:+.1}%", v),
            IndicatorValue::Rsi(v) => format!("{:.1}", v),
        }
    }

    pub fn colors(&self, thresholds: Option<&IndicatorThresholds>) -> IndicatorColors {
        match self {
            IndicatorValue::Volatility(value) => {
                let mut background = None;
                if let Some(IndicatorThresholds::Volatility { threshold }) = thresholds {
                    if *threshold > 0.0 && value.abs() >= *threshold {
                        background = Some(if *value >= 0.0 {
                            Self::POSITIVE_BG
                        } else {
                            Self::NEGATIVE_BG
                        });
                    }
                }

                let text = if background.is_some() {
                    Color::White
                } else if *value >= 0.0 {
                    Self::POSITIVE_TEXT
                } else {
                    Self::NEGATIVE_TEXT
                };

                IndicatorColors { text, background }
            }
            IndicatorValue::Rsi(value) => {
                let mut background = None;
                if let Some(IndicatorThresholds::Rsi {
                    oversold,
                    overbought,
                }) = thresholds
                {
                    if value <= oversold {
                        background = Some(Self::NEGATIVE_BG);
                    } else if value >= overbought {
                        background = Some(Self::POSITIVE_BG);
                    }
                }

                let text = if background.is_some() {
                    Color::White
                } else if *value >= 50.0 {
                    Self::POSITIVE_TEXT
                } else {
                    Self::NEGATIVE_TEXT
                };

                IndicatorColors { text, background }
            }
        }
    }
}
