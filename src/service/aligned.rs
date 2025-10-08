use std::io::Write;

use owo_colors::OwoColorize;
use tracing::info;

use crate::{
    adapter::{HtmlReporter, TimelineVisualizer},
    domain::{State, StateBuffer, TrackerError},
    port::{AlignmentKeyExtractor, Differ, StateSource}
};

pub struct AlignedTracker<L: StateSource, R: StateSource, D: Differ, E: AlignmentKeyExtractor> {
    left:             L,
    right:            R,
    differ:           D,
    extractor:        E,
    /// Optional signal key/value that marks end of a round (e.g., "type=GameCleared")
    round_end_signal: Option<String>,
    /// Enable visual timeline rendering
    visual:           bool,
    /// Optional output file for HTML report
    report_output:    Option<String>,
    /// Enable pretty diff output
    pretty_diff:      bool,
    /// Maximum number of rounds to track (None = infinite)
    max_rounds:       Option<usize>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Visual,     // Priority 1: --visual (live feed)
    PrettyDiff, // Priority 2: --pretty (pretty diffs)
    Logs        // Priority 3: default (structured logs)
}

impl<L: StateSource, R: StateSource, D: Differ, E: AlignmentKeyExtractor> AlignedTracker<L, R, D, E> {
    pub fn new(left: L, right: R, differ: D, extractor: E) -> Self {
        Self {
            left,
            right,
            differ,
            extractor,
            round_end_signal: None,
            visual: false,
            report_output: None,
            pretty_diff: false,
            max_rounds: None
        }
    }

    pub fn with_round_end_signal(mut self, signal: String) -> Self {
        self.round_end_signal = Some(signal);
        self
    }

    pub fn with_visual(mut self, enabled: bool) -> Self {
        self.visual = enabled;
        self
    }

    pub fn with_report_output(mut self, path: String) -> Self {
        self.report_output = Some(path);
        self
    }

    pub fn with_pretty_diff(mut self, enabled: bool) -> Self {
        self.pretty_diff = enabled;
        self
    }

    pub fn with_max_rounds(mut self, max: usize) -> Self {
        self.max_rounds = Some(max);
        self
    }

    fn output_mode(&self) -> OutputMode {
        // Priority: visual > pretty_diff > logs
        if self.visual {
            OutputMode::Visual
        } else if self.pretty_diff {
            OutputMode::PrettyDiff
        } else {
            OutputMode::Logs
        }
    }

    pub async fn start(&self) -> Result<(), TrackerError> {
        let mut left_rx = self.left.spawn();
        let mut right_rx = self.right.spawn();

        let mut left_buffer = StateBuffer::new(100);
        let mut right_buffer = StateBuffer::new(100);

        let mut left_round_complete = false;
        let mut right_round_complete = false;
        let mut rounds_completed: usize = 0;

        let mode = self.output_mode();

        let mut visualizer = if mode == OutputMode::Visual { Some(TimelineVisualizer::new(15, 100)) } else { None };

        let mut reporter = if self.report_output.is_some() { Some(HtmlReporter::new()) } else { None };

        // Show initial status for non-visual modes
        if mode != OutputMode::Visual {
            match mode {
                OutputMode::PrettyDiff => println!("🎨 Pretty Diff Mode - Showing aligned state comparisons\n"),
                OutputMode::Logs => {
                    info!("📊 State tracker started");
                    if let Some(ref signal) = self.round_end_signal {
                        info!("🎯 Waiting for round completion signal: {}", signal);
                    }
                }
                _ => {}
            }
        }

        loop {
            tokio::select! {
                msg = left_rx.recv() => {
                    match msg {
                        Some(data) => {
                            let alignment_key = self.extractor.extract_key(&data);
                            let state = State::new(data, alignment_key.clone());

                            // Always add to visualizer (even if no key extracted)
                            if let Some(ref mut viz) = visualizer {
                                let display_key = alignment_key.as_deref().unwrap_or("<no-key>");
                                viz.add_left(display_key);
                            }

                            // Add to reporter
                            if let Some(ref mut rep) = reporter {
                                rep.add_left(state.clone());
                            }

                            if let Some(key) = &alignment_key {
                                // Only log in Logs mode
                                if mode == OutputMode::Logs {
                                    info!("left: {}", key);
                                }

                                // Check if this is the round end signal
                                if let Some(ref signal) = self.round_end_signal {
                                    if key == signal {
                                        if mode == OutputMode::Logs {
                                            info!("✓ left round complete");
                                        }
                                        left_round_complete = true;
                                    }
                                }
                            }

                            left_buffer.push(state);

                            // Render visual if enabled
                            if let Some(ref viz) = visualizer {
                                viz.render();
                            }

                            // Check alignment or round completion
                            if self.round_end_signal.is_some() {
                                let should_exit = self.check_round_completion(
                                    &mut left_buffer,
                                    &mut right_buffer,
                                    left_round_complete,
                                    right_round_complete,
                                    &mut left_round_complete,
                                    &mut right_round_complete,
                                    visualizer.as_mut(),
                                    &mut rounds_completed,
                                );

                                if should_exit {
                                    if mode != OutputMode::Visual {
                                        info!("🏁 Completed {} round(s), exiting", rounds_completed);
                                    }
                                    return Ok(());
                                }
                            } else {
                                self.check_alignment(&left_buffer, &right_buffer);
                            }
                        }
                        None => {
                            if mode != OutputMode::Visual {
                                info!("left stream closed");
                            }
                            break;
                        }
                    }
                }
                msg = right_rx.recv() => {
                    match msg {
                        Some(data) => {
                            let alignment_key = self.extractor.extract_key(&data);
                            let state = State::new(data, alignment_key.clone());

                            // Always add to visualizer (even if no key extracted)
                            if let Some(ref mut viz) = visualizer {
                                let display_key = alignment_key.as_deref().unwrap_or("<no-key>");
                                viz.add_right(display_key);
                            }

                            // Add to reporter
                            if let Some(ref mut rep) = reporter {
                                rep.add_right(state.clone());
                            }

                            if let Some(key) = &alignment_key {
                                // Only log in Logs mode
                                if mode == OutputMode::Logs {
                                    info!("right: {}", key);
                                }

                                // Check if this is the round end signal
                                if let Some(ref signal) = self.round_end_signal {
                                    if key == signal {
                                        if mode == OutputMode::Logs {
                                            info!("✓ right round complete");
                                        }
                                        right_round_complete = true;
                                    }
                                }
                            }

                            right_buffer.push(state);

                            // Render visual if enabled
                            if let Some(ref viz) = visualizer {
                                viz.render();
                            }

                            // Check alignment or round completion
                            if self.round_end_signal.is_some() {
                                let should_exit = self.check_round_completion(
                                    &mut left_buffer,
                                    &mut right_buffer,
                                    left_round_complete,
                                    right_round_complete,
                                    &mut left_round_complete,
                                    &mut right_round_complete,
                                    visualizer.as_mut(),
                                    &mut rounds_completed,
                                );

                                if should_exit {
                                    if mode != OutputMode::Visual {
                                        info!("🏁 Completed {} round(s), exiting", rounds_completed);
                                    }
                                    return Ok(());
                                }
                            } else {
                                self.check_alignment(&left_buffer, &right_buffer);
                            }
                        }
                        None => {
                            if mode != OutputMode::Visual {
                                info!("right stream closed");
                            }
                            break;
                        }
                    }
                }
            }
        }

        // Generate HTML report if requested
        if let (Some(output_path), Some(rep)) = (self.report_output.as_ref(), reporter) {
            if let Err(e) = rep.generate(output_path) {
                eprintln!("⚠️  Failed to generate report: {}", e);
            }
        }

        Ok(())
    }

    fn check_alignment(&self, left_buffer: &StateBuffer, right_buffer: &StateBuffer) {
        let left_key = left_buffer.latest_alignment_key();
        let right_key = right_buffer.latest_alignment_key();
        let mode = self.output_mode();

        match (left_key, right_key) {
            (Some(l_key), Some(r_key)) if l_key == r_key => {
                // Keys are aligned! Compare the states
                if let (Some(left_state), Some(right_state)) = (left_buffer.latest(), right_buffer.latest()) {
                    match mode {
                        OutputMode::Logs => {
                            info!("✓ aligned: {}", l_key);
                        }
                        OutputMode::PrettyDiff => {
                            println!("\n✓ Aligned at: {}", l_key.bright_green().bold());
                            self.differ.print_diff("left", "right", &left_state.data, &right_state.data);
                        }
                        OutputMode::Visual => {} // Handled by visualizer
                    }
                }
            }
            (Some(l_key), Some(r_key)) => {
                if mode == OutputMode::PrettyDiff {
                    print!("\r⏳ Waiting: left={} ≠ right={}     ", l_key, r_key);
                    std::io::stdout().flush().ok();
                } else if mode == OutputMode::Logs {
                    info!("⏳ out of sync - left: {}, right: {}", l_key, r_key);
                }
            }
            (Some(l_key), None) => {
                if mode == OutputMode::PrettyDiff {
                    print!("\r⏳ left={}, waiting for right...     ", l_key);
                    std::io::stdout().flush().ok();
                } else if mode == OutputMode::Logs {
                    info!("⏳ left ahead: {} (right not received)", l_key);
                }
            }
            (None, Some(r_key)) => {
                if mode == OutputMode::PrettyDiff {
                    print!("\r⏳ right={}, waiting for left...     ", r_key);
                    std::io::stdout().flush().ok();
                } else if mode == OutputMode::Logs {
                    info!("⏳ right ahead: {} (left not received)", r_key);
                }
            }
            (None, None) => {}
        }
    }

    fn check_round_completion(
        &self,
        left_buffer: &mut StateBuffer,
        right_buffer: &mut StateBuffer,
        left_complete: bool,
        right_complete: bool,
        left_flag: &mut bool,
        right_flag: &mut bool,
        mut visualizer: Option<&mut TimelineVisualizer>,
        rounds_completed: &mut usize
    ) -> bool {
        let mode = self.output_mode();
        if left_complete && right_complete {
            *rounds_completed += 1;
            // Compare all states in the buffers
            let left_states = left_buffer.states();
            let right_states = right_buffer.states();

            if self.visual {
                // Use visual rendering
                if let Some(ref mut viz) = visualizer {
                    viz.render_round_comparison(left_states, right_states);
                    // Wait a bit so user can see it
                    std::thread::sleep(std::time::Duration::from_millis(2000));
                }
            } else {
                info!("🎯 Both rounds complete! Comparing full rounds...");
                info!("📊 Round stats: left={} states, right={} states", left_states.len(), right_states.len());
            }

            if !self.visual {
                // Compare state by state based on alignment keys
                for (i, left_state) in left_states.iter().enumerate() {
                    if let Some(left_key) = &left_state.alignment_key {
                        // Find matching state in right buffer
                        if let Some(right_state) =
                            right_states.iter().find(|r| r.alignment_key.as_ref() == Some(left_key))
                        {
                            info!("  Comparing state {}: {}", i + 1, left_key);
                            self.differ.print_diff("left", "right", &left_state.data, &right_state.data);
                        } else {
                            info!("  ⚠️  State {} ({}) missing in right", i + 1, left_key);
                        }
                    }
                }

                // Check for states in right that aren't in left
                for right_state in right_states.iter() {
                    if let Some(right_key) = &right_state.alignment_key {
                        if !left_states.iter().any(|l| l.alignment_key.as_ref() == Some(right_key)) {
                            info!("  ⚠️  State ({}) only in right", right_key);
                        }
                    }
                }

                info!("✅ Round comparison complete\n");
            }

            if let Some(output_path) = &self.report_output {
                let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                let report_path = output_path.replace(".html", &format!("_{}.html", timestamp));

                let mut final_reporter = HtmlReporter::new();
                for state in left_buffer.states() {
                    final_reporter.add_left(state.clone());
                }
                for state in right_buffer.states() {
                    final_reporter.add_right(state.clone());
                }

                if let Err(e) = final_reporter.generate(&report_path) {
                    if mode != OutputMode::Visual {
                        eprintln!("⚠️  Failed to generate round report: {}", e);
                    }
                } else if mode != OutputMode::Visual {
                    println!("📄 Round report: {}", report_path);
                }
            }

            // Reset for next round
            *left_flag = false;
            *right_flag = false;
            left_buffer.clear();
            right_buffer.clear();
            if let Some(ref mut viz) = visualizer {
                viz.clear_history();
            }

            // Check if we should stop
            if let Some(max) = self.max_rounds {
                if *rounds_completed >= max {
                    return true; // Signal to exit
                }
            }
        } else if left_complete && mode == OutputMode::Logs {
            info!("⏳ left round complete, waiting for right...");
        } else if right_complete && mode == OutputMode::Logs {
            info!("⏳ right round complete, waiting for left...");
        }

        false // Continue tracking
    }
}
