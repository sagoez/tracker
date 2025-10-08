use owo_colors::OwoColorize;
use std::collections::VecDeque;

use crate::domain::State;

/// Visual timeline renderer for state tracking
pub struct TimelineVisualizer {
    left_history:  VecDeque<String>,
    right_history: VecDeque<String>,
    max_history:   usize,
    width:         usize,
}

impl TimelineVisualizer {
    pub fn new(max_history: usize, width: usize) -> Self {
        Self {
            left_history:  VecDeque::new(),
            right_history: VecDeque::new(),
            max_history,
            width,
        }
    }

    pub fn add_left(&mut self, key: &str) {
        self.left_history.push_back(key.to_string());
        if self.left_history.len() > self.max_history {
            self.left_history.pop_front();
        }
    }

    pub fn add_right(&mut self, key: &str) {
        self.right_history.push_back(key.to_string());
        if self.right_history.len() > self.max_history {
            self.right_history.pop_front();
        }
    }

    pub fn render(&self) {
        self.clear_screen();
        self.print_header();
        self.print_timeline();
        self.print_footer();
    }

    pub fn render_round_comparison(&self, left_states: &[State], right_states: &[State]) {
        self.clear_screen();
        println!("\n{}", "═".repeat(self.width).bright_cyan());
        println!(
            "{}",
            format!("🎯 ROUND COMPARISON").bright_yellow().bold()
        );
        println!("{}\n", "═".repeat(self.width).bright_cyan());

        let max_len = left_states.len().max(right_states.len());

        // Header
        println!(
            "{:^4} │ {:<30} │ {:<30} │ {}",
            "#".bright_white().bold(),
            "LEFT".blue().bold(),
            "RIGHT".magenta().bold(),
            "STATUS".bright_white().bold()
        );
        println!("{}", "─".repeat(self.width).dimmed());

        // Compare states
        for i in 0..max_len {
            let left_key = left_states.get(i).and_then(|s| s.alignment_key.as_deref());
            let right_key = right_states.get(i).and_then(|s| s.alignment_key.as_deref());

            let status = match (left_key, right_key) {
                (Some(l), Some(r)) if l == r => "✓".green().to_string(),
                (Some(_), Some(_)) => "✗ MISMATCH".red().bold().to_string(),
                (Some(_), None) => "← MISSING".yellow().to_string(),
                (None, Some(_)) => "MISSING →".yellow().to_string(),
                (None, None) => "".dimmed().to_string(),
            };

            let left_display = left_key
                .map(|k| format!("{}", k.blue()))
                .unwrap_or_else(|| "—".dimmed().to_string());
            let right_display = right_key
                .map(|k| format!("{}", k.magenta()))
                .unwrap_or_else(|| "—".dimmed().to_string());

            println!(
                "{:>4} │ {:<30} │ {:<30} │ {}",
                format!("{}", i + 1).bright_white(),
                left_display,
                right_display,
                status
            );
        }

        println!("\n{}", "═".repeat(self.width).bright_cyan());
        println!(
            "{}",
            format!(
                "📊 Total: {} left, {} right",
                left_states.len(),
                right_states.len()
            )
            .dimmed()
        );
    }

    fn clear_screen(&self) {
        print!("\x1B[2J\x1B[1;1H");
    }

    fn print_header(&self) {
        println!("\n{}", "═".repeat(self.width).bright_cyan());
        println!(
            "{}  {}",
            "🔄 STATE TRACKER".bright_yellow().bold(),
            "(Live View)".dimmed()
        );
        println!("{}\n", "═".repeat(self.width).bright_cyan());
    }

    fn print_timeline(&self) {
        let left_len = self.left_history.len();
        let right_len = self.right_history.len();
        let max_len = left_len.max(right_len);

        // Column headers
        println!(
            "{:^4} │ {:<40} │ {:<40}",
            "#".bright_white().bold(),
            "LEFT STREAM".blue().bold(),
            "RIGHT STREAM".magenta().bold()
        );
        println!("{}", "─".repeat(self.width).dimmed());

        // Print timeline rows
        for i in 0..max_len {
            let left = self
                .left_history
                .get(i)
                .map(|s| self.format_state_box(s, true))
                .unwrap_or_else(|| " ".repeat(40));

            let right = self
                .right_history
                .get(i)
                .map(|s| self.format_state_box(s, false))
                .unwrap_or_else(|| " ".repeat(40));

            // Check if they're aligned
            let marker = if let (Some(l), Some(r)) = (self.left_history.get(i), self.right_history.get(i)) {
                if l == r {
                    "✓".green().bold().to_string()
                } else {
                    "✗".red().to_string()
                }
            } else {
                " ".to_string()
            };

            println!("{:>4} │ {} │ {}", marker, left, right);
        }

        // Show current alignment status
        if let (Some(l), Some(r)) = (self.left_history.back(), self.right_history.back()) {
            println!("\n{}", "─".repeat(self.width).dimmed());
            if l == r {
                println!(
                    "{} {}",
                    "✓ ALIGNED:".green().bold(),
                    l.bright_white().bold()
                );
            } else {
                println!(
                    "{} left={} ≠ right={}",
                    "⏳ WAITING:".yellow().bold(),
                    l.blue().bold(),
                    r.magenta().bold()
                );
            }
        }
    }

    fn format_state_box(&self, state: &str, is_left: bool) -> String {
        let truncated = if state.len() > 35 {
            format!("{}...", &state[..32])
        } else {
            state.to_string()
        };

        if is_left {
            format!("{:<40}", truncated.blue())
        } else {
            format!("{:<40}", truncated.magenta())
        }
    }

    fn print_footer(&self) {
        println!("\n{}", "─".repeat(self.width).dimmed());
        println!(
            "{}  Press Ctrl-C to exit",
            "ℹ".bright_cyan().bold()
        );
    }

    pub fn clear_history(&mut self) {
        self.left_history.clear();
        self.right_history.clear();
    }
}

