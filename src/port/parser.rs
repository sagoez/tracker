use serde_json::Value as JsonValue;

/// Port for extracting alignment keys from JSON states
pub trait AlignmentKeyExtractor: Send + Sync {
    /// Extract an alignment key from a JSON state (e.g., message type, phase, etc.)
    /// Returns None if no alignment key can be extracted
    fn extract_key(&self, state: &JsonValue) -> Option<String>;
}

/// Extractor that uses a JSON path to extract the alignment key
pub struct JsonPathExtractor {
    field_path: Vec<String>
}

impl JsonPathExtractor {
    /// Create extractor with a dot-separated path (e.g., "message.type" or "event_type")
    pub fn new(path: &str) -> Self {
        Self { field_path: path.split('.').map(|s| s.to_string()).collect() }
    }
}

impl AlignmentKeyExtractor for JsonPathExtractor {
    fn extract_key(&self, state: &JsonValue) -> Option<String> {
        let mut current = state;

        // Navigate through the path
        for field in &self.field_path {
            current = current.get(field)?;
        }

        // Extract the final value as a string
        match current {
            JsonValue::String(s) => Some(s.clone()),
            JsonValue::Number(n) => Some(n.to_string()),
            JsonValue::Bool(b) => Some(b.to_string()),
            _ => None
        }
    }
}

/// Extractor that tries multiple common field names
pub struct AutoExtractor {
    common_fields: Vec<String>
}

impl Default for AutoExtractor {
    fn default() -> Self {
        Self {
            common_fields: vec![
                "type".to_string(),
                "event_type".to_string(),
                "message_type".to_string(),
                "phase".to_string(),
                "state".to_string(),
                "action".to_string(),
                "args".to_string(),
            ]
        }
    }
}

impl AlignmentKeyExtractor for AutoExtractor {
    fn extract_key(&self, state: &JsonValue) -> Option<String> {
        for field in &self.common_fields {
            if let Some(value) = state.get(field) {
                match value {
                    JsonValue::String(s) => return Some(s.clone()),
                    JsonValue::Number(n) => return Some(n.to_string()),
                    JsonValue::Bool(b) => return Some(b.to_string()),
                    _ => continue
                }
            }
        }
        None
    }
}
