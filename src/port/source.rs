use serde_json::Value;
use tokio::sync::mpsc::Receiver;

/// Abstraction for a source of JSON state updates.
/// Implementations should spawn an internal task and return a Receiver of states.
pub trait StateSource: Send + Sync {
    fn spawn(&self) -> Receiver<Value>;
}
