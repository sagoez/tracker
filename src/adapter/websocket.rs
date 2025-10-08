use futures::StreamExt;
use serde_json::Value;
use tokio::{
    sync::mpsc,
    time::{Duration, sleep}
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

use crate::port::StateSource;

#[derive(Clone, Debug)]
pub struct WebSocketSource {
    pub name: String,
    pub url:  String
}

impl WebSocketSource {
    pub fn new<N: Into<String>, U: Into<String>>(name: N, url: U) -> Self {
        Self { name: name.into(), url: url.into() }
    }
}

impl StateSource for WebSocketSource {
    fn spawn(&self) -> mpsc::Receiver<Value> {
        let (tx, rx) = mpsc::channel::<Value>(64);
        let name = self.name.clone();
        let url = self.url.clone();
        tokio::spawn(async move {
            let mut backoff_secs: u64 = 1;
            loop {
                match connect_async(&url).await {
                    Ok((ws_stream, _resp)) => {
                        info!("{name} connected to {url}");
                        backoff_secs = 1;
                        let (_write, mut read) = ws_stream.split();
                        while let Some(next) = read.next().await {
                            match next {
                                Ok(Message::Text(txt)) => match serde_json::from_str::<Value>(&txt) {
                                    Ok(json) => {
                                        let _ = tx.send(json).await;
                                    }
                                    Err(err) => warn!("{name} failed to parse text as JSON: {err}")
                                },
                                Ok(Message::Binary(bin)) => match String::from_utf8(bin.to_vec()) {
                                    Ok(txt) => match serde_json::from_str::<Value>(&txt) {
                                        Ok(json) => {
                                            let _ = tx.send(json).await;
                                        }
                                        Err(err) => {
                                            warn!("{name} failed to parse binary as JSON: {err}")
                                        }
                                    },
                                    Err(err) => warn!("{name} received non-utf8 binary: {err}")
                                },
                                Ok(Message::Ping(_)) => {}
                                Ok(Message::Pong(_)) => {}
                                Ok(Message::Close(frame)) => {
                                    warn!("{name} closed by peer: {:?}", frame);
                                    break; // reconnect
                                }
                                Err(err) => {
                                    warn!("{name} read error: {err}");
                                    break; // reconnect
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(err) => {
                        warn!("{name} connect error to {url}: {err}");
                    }
                }

                let delay = Duration::from_secs(backoff_secs.min(30));
                info!("{name} reconnecting in {:?}", delay);
                sleep(delay).await;
                backoff_secs = (backoff_secs * 2).max(2);
            }
        });
        rx
    }
}
