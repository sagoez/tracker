use rand::Rng;
use serde_json::{Value as JsonValue, json};
use tokio::{
    sync::mpsc,
    time::{Duration, sleep}
};

use crate::port::StateSource;

pub struct RandomStream {
    name:        String,
    interval_ms: u64,
    event_types: Vec<String>
}

impl RandomStream {
    pub fn new<N: Into<String>>(name: N, interval_ms: u64) -> Self {
        Self {
            name: name.into(),
            interval_ms,
            event_types: vec![
                "user.login".to_string(),
                "user.logout".to_string(),
                "order.created".to_string(),
                "order.updated".to_string(),
                "order.completed".to_string(), // Round end signal
                "payment.processed".to_string(),
                "inventory.changed".to_string(),
            ]
        }
    }

    pub fn with_event_types<N: Into<String>>(name: N, interval_ms: u64, event_types: Vec<String>) -> Self {
        Self { name: name.into(), interval_ms, event_types }
    }

    fn generate_event(&self) -> JsonValue {
        let mut rng = rand::rng();

        let event_type = &self.event_types[rng.random_range(0..self.event_types.len())];
        let user_id = rng.random_range(1000..9999);
        let amount = rng.random_range(10.0..1000.0);
        let status = ["pending", "completed", "failed"][rng.random_range(0..3)];

        json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "event_type": event_type,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "user_id": user_id,
            "data": {
                "amount": format!("{:.2}", amount),
                "status": status,
                "metadata": {
                    "source": self.name,
                    "version": "1.0"
                }
            }
        })
    }
}

impl StateSource for RandomStream {
    fn spawn(&self) -> mpsc::Receiver<JsonValue> {
        let (tx, rx) = mpsc::channel::<JsonValue>(64);
        let name = self.name.clone();
        let interval = Duration::from_millis(self.interval_ms);
        let event_types = self.event_types.clone();

        tokio::spawn(async move {
            tracing::info!("{name} starting random event stream (interval: {:?})", interval);
            let stream = RandomStream::with_event_types(name.clone(), interval.as_millis() as u64, event_types);

            loop {
                let event = stream.generate_event();
                if tx.send(event).await.is_err() {
                    tracing::warn!("{name} output channel closed");
                    break;
                }
                sleep(interval).await;
            }
        });

        rx
    }
}
