use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrackerError {
    #[error("websocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("channel closed")]
    ChannelClosed
}
