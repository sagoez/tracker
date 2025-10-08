use json_patch::diff as json_patch_diff;
use owo_colors::OwoColorize;
use serde_json::Value as JsonValue;

use crate::port::Differ;

#[derive(Debug, Clone, Copy)]
pub enum DiffEngine {
    JsonPatch,
    SerdeDiff
}

pub struct JsonPatchDiffer {
    pretty: bool,
    engine: DiffEngine
}

impl JsonPatchDiffer {
    pub fn new(pretty: bool, engine: DiffEngine) -> Self {
        Self { pretty, engine }
    }
}

impl Default for JsonPatchDiffer {
    fn default() -> Self {
        Self::new(false, DiffEngine::JsonPatch)
    }
}

impl Differ for JsonPatchDiffer {
    fn print_diff(&self, left_label: &str, right_label: &str, left: &JsonValue, right: &JsonValue) {
        if left == right {
            tracing::info!("states are identical");
            return;
        }

        if self.pretty {
            self.print_pretty_diff(left_label, right_label, left, right);
        } else {
            match self.engine {
                DiffEngine::JsonPatch => self.print_json_patch_diff(left_label, right_label, left, right),
                DiffEngine::SerdeDiff => self.print_serde_diff(left_label, right_label, left, right)
            }
        }
    }
}

impl JsonPatchDiffer {
    fn print_json_patch_diff(&self, left_label: &str, right_label: &str, left: &JsonValue, right: &JsonValue) {
        let patch = json_patch_diff(left, right);
        let patch_json = match serde_json::to_value(&patch) {
            Ok(v) => v,
            Err(_) => JsonValue::Null
        };
        let ops_count = patch_json.as_array().map(|a| a.len()).unwrap_or(1);

        println!(
            "\n{} {} -> {} ({} ops) [json-patch]",
            "diff".bold(),
            left_label.blue().bold(),
            right_label.magenta().bold(),
            ops_count
        );

        // Pretty print the JSON directly
        let json_string = serde_json::to_string_pretty(&patch_json).unwrap_or_else(|_| "[]".to_string());
        println!("{}", json_string);
    }

    fn print_serde_diff(&self, left_label: &str, right_label: &str, left: &JsonValue, right: &JsonValue) {
        println!(
            "\n{} {} {} {} {}",
            "diff".bold(),
            left_label.blue().bold(),
            "→".dimmed(),
            right_label.magenta().bold(),
            "[serde_json_diff]".dimmed()
        );

        match serde_json_diff::values(left.clone(), right.clone()) {
            Some(diff) => {
                // Serialize the structured diff directly
                let diff_json = serde_json::to_value(&diff).unwrap_or(JsonValue::Null);
                let json_string = serde_json::to_string_pretty(&diff_json).unwrap_or_else(|_| "{}".to_string());
                println!("{}", json_string);
            }
            None => {
                println!("{}", "  (no differences)".dimmed());
            }
        }
    }

    fn print_pretty_diff(&self, left_label: &str, right_label: &str, left: &JsonValue, right: &JsonValue) {
        println!(
            "\n{} {} {} {}",
            "━━━".dimmed(),
            left_label.blue().bold(),
            "vs".dimmed(),
            right_label.magenta().bold()
        );

        self.print_value_diff("", left, right, 0);
        println!();
    }

    fn print_value_diff(&self, path: &str, left: &JsonValue, right: &JsonValue, indent: usize) {
        let indent_str = "  ".repeat(indent);

        match (left, right) {
            (JsonValue::Object(l_obj), JsonValue::Object(r_obj)) => {
                let mut all_keys = std::collections::BTreeSet::new();
                all_keys.extend(l_obj.keys());
                all_keys.extend(r_obj.keys());

                for key in all_keys {
                    let current_path = if path.is_empty() { key.to_string() } else { format!("{}.{}", path, key) };

                    match (l_obj.get(key), r_obj.get(key)) {
                        (Some(l_val), Some(r_val)) => {
                            if l_val != r_val {
                                if l_val.is_object() || r_val.is_object() || l_val.is_array() || r_val.is_array() {
                                    println!("{}{}", indent_str, key.bold());
                                    self.print_value_diff(&current_path, l_val, r_val, indent + 1);
                                } else {
                                    println!(
                                        "{}{}: {} {} {}",
                                        indent_str,
                                        key.bold(),
                                        Self::format_value(l_val).red().strikethrough(),
                                        "→".yellow(),
                                        Self::format_value(r_val).green()
                                    );
                                }
                            }
                        }
                        (Some(l_val), None) => {
                            println!(
                                "{}{}: {} {}",
                                indent_str,
                                key.bold(),
                                Self::format_value(l_val).red().strikethrough(),
                                "(removed)".red().dimmed()
                            );
                        }
                        (None, Some(r_val)) => {
                            println!(
                                "{}{}: {} {}",
                                indent_str,
                                key.bold(),
                                "(added)".green().dimmed(),
                                Self::format_value(r_val).green()
                            );
                        }
                        (None, None) => {}
                    }
                }
            }
            (JsonValue::Array(l_arr), JsonValue::Array(r_arr)) => {
                if l_arr != r_arr {
                    println!(
                        "{}[array changed: {} {} {}]",
                        indent_str,
                        format!("{} items", l_arr.len()).red(),
                        "→".yellow(),
                        format!("{} items", r_arr.len()).green()
                    );
                }
            }
            _ => {
                if left != right {
                    println!(
                        "{}{} {} {}",
                        indent_str,
                        Self::format_value(left).red().strikethrough(),
                        "→".yellow(),
                        Self::format_value(right).green()
                    );
                }
            }
        }
    }

    fn format_value(val: &JsonValue) -> String {
        match val {
            JsonValue::String(s) => format!("\"{}\"", s),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Null => "null".to_string(),
            _ => val.to_string()
        }
    }
}
