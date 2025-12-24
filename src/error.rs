use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("no trading pairs configured")]
    MissingPairs,
}

#[derive(Debug, Error)]
pub enum WsError {
    #[error("websocket requires at least one pair")]
    EmptyPairs,
    #[error("websocket error: {0}")]
    Transport(#[from] tokio_tungstenite::tungstenite::Error),
}

#[derive(Debug, Error)]
pub enum GlobalError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Websocket(#[from] WsError),
    #[error("other error: {0}")]
    Other(String),
}

impl From<String> for GlobalError {
    fn from(msg: String) -> Self {
        GlobalError::Other(msg)
    }
}

impl From<&'static str> for GlobalError {
    fn from(msg: &'static str) -> Self {
        GlobalError::Other(msg.to_string())
    }
}

pub type Result<T> = std::result::Result<T, GlobalError>;
