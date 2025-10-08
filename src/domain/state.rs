use serde_json::Value as JsonValue;

/// Generic state with an optional alignment key
#[derive(Debug, Clone)]
pub struct State {
    /// The raw JSON data
    pub data: JsonValue,

    /// Optional alignment key extracted from the data (e.g., message type, phase, etc.)
    pub alignment_key: Option<String>,

    /// When this state was received
    pub timestamp: chrono::DateTime<chrono::Utc>
}

impl State {
    pub fn new(data: JsonValue, alignment_key: Option<String>) -> Self {
        Self { data, alignment_key, timestamp: chrono::Utc::now() }
    }

    pub fn with_data(data: JsonValue) -> Self {
        Self::new(data, None)
    }
}

#[derive(Debug, Clone)]
pub struct StateBuffer {
    states:   Vec<State>,
    max_size: usize
}

impl StateBuffer {
    pub fn new(max_size: usize) -> Self {
        Self { states: Vec::new(), max_size }
    }

    pub fn push(&mut self, state: State) {
        self.states.push(state);
        if self.states.len() > self.max_size {
            self.states.remove(0);
        }
    }

    pub fn latest(&self) -> Option<&State> {
        self.states.last()
    }

    pub fn latest_alignment_key(&self) -> Option<&str> {
        self.latest().and_then(|s| s.alignment_key.as_deref())
    }

    pub fn states(&self) -> &[State] {
        &self.states
    }

    pub fn clear(&mut self) {
        self.states.clear();
    }

    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}
