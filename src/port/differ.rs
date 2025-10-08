use serde_json::Value as JsonValue;

/// Port for diffing two JSON values and producing output
pub trait Differ: Send + Sync {
    fn print_diff(&self, left_label: &str, right_label: &str, left: &JsonValue, right: &JsonValue);
}
