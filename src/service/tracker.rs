use serde_json::Value as JsonValue;
use tracing::info;

use crate::{
    domain::TrackerError,
    port::{Differ, StateSource}
};

pub struct Tracker<L: StateSource, R: StateSource, D: Differ> {
    left:   L,
    right:  R,
    differ: D
}

impl<L: StateSource, R: StateSource, D: Differ> Tracker<L, R, D> {
    pub fn new(left: L, right: R, differ: D) -> Self {
        Self { left, right, differ }
    }

    pub async fn start(&self) -> Result<(), TrackerError> {
        let mut left_rx = self.left.spawn();
        let mut right_rx = self.right.spawn();

        let mut left_state: Option<JsonValue> = None;
        let mut right_state: Option<JsonValue> = None;

        loop {
            tokio::select! {
                msg = left_rx.recv() => {
                    match msg {
                        Some(state) => {
                            left_state = Some(state);
                            if let (Some(l), Some(r)) = (left_state.as_ref(), right_state.as_ref()) {
                                self.differ.print_diff("left", "right", l, r);
                            } else {
                                info!("left updated; waiting for right before diffing");
                            }
                        }
                        None => break,
                    }
                }
                msg = right_rx.recv() => {
                    match msg {
                        Some(state) => {
                            right_state = Some(state);
                            if let (Some(l), Some(r)) = (left_state.as_ref(), right_state.as_ref()) {
                                self.differ.print_diff("left", "right", l, r);
                            } else {
                                info!("right updated; waiting for left before diffing");
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        Ok(())
    }
}
