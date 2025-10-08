use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::{EnvFilter, fmt};
use tracker::prelude::*;

#[derive(Parser, Debug)]
#[command(name = "tracker", version, about = "Track diffs between two WebSocket JSON streams")]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum Engine {
    JsonPatch,
    SerdeDiff
}

impl From<Engine> for DiffEngine {
    fn from(e: Engine) -> Self {
        match e {
            Engine::JsonPatch => DiffEngine::JsonPatch,
            Engine::SerdeDiff => DiffEngine::SerdeDiff
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Diff two WebSocket JSON streams in real-time (immediate mode)
    Diff {
        /// Left WebSocket URL
        left_url:  String,
        /// Right WebSocket URL
        right_url: String,
        /// Use pretty, human-readable diff format
        #[arg(long)]
        pretty:    bool,
        /// Diff engine to use
        #[arg(long, value_enum, default_value = "json-patch")]
        engine:    Engine
    },
    /// Track and align states by a specific field (phase-aligned mode)
    Track {
        /// Left WebSocket URL
        left_url:   String,
        /// Right WebSocket URL
        right_url:  String,
        /// JSON field path to use for alignment (e.g., "type", "message.phase", "event_type")
        #[arg(long)]
        align_by:   String,
        /// Optional signal value that marks end of a round (e.g., "GameCleared")
        /// When set, waits for both sides to receive this signal before comparing full rounds
        #[arg(long)]
        round_end:  Option<String>,
        /// Enable visual timeline display
        #[arg(long)]
        visual:     bool,
        /// Generate HTML report to file (e.g., "report.html")
        #[arg(long)]
        report:     Option<String>,
        /// Stop after tracking one round
        #[arg(long)]
        once:       bool,
        /// Maximum number of rounds to track (default: infinite)
        #[arg(long)]
        max_rounds: Option<usize>,
        /// Use pretty, human-readable diff format
        #[arg(long)]
        pretty:     bool,
        /// Diff engine to use
        #[arg(long, value_enum, default_value = "json-patch")]
        engine:     Engine
    },
    /// Show example diff with random JSON streams
    Example {
        /// Interval in milliseconds for left stream
        #[arg(long, default_value = "1000")]
        left_interval:  u64,
        /// Interval in milliseconds for right stream
        #[arg(long, default_value = "1500")]
        right_interval: u64,
        /// Use pretty, human-readable diff format
        #[arg(long)]
        pretty:         bool,
        /// Diff engine to use
        #[arg(long, value_enum, default_value = "json-patch")]
        engine:         Engine,
        /// JSON field path to use for alignment (optional)
        #[arg(long)]
        align_by:       Option<String>,
        /// Optional signal value that marks end of a round (e.g., "order.completed")
        #[arg(long)]
        round_end:      Option<String>,
        /// Enable visual timeline display
        #[arg(long)]
        visual:         bool,
        /// Generate HTML report to file (e.g., "report.html")
        #[arg(long)]
        report:         Option<String>,
        /// Stop after tracking one round
        #[arg(long)]
        once:           bool,
        /// Maximum number of rounds to track (default: infinite)
        #[arg(long)]
        max_rounds:     Option<usize>
    }
}

async fn run_tracker<L: StateSource, R: StateSource, D: Differ>(tracker: Tracker<L, R, D>) -> Result<(), TrackerError> {
    tokio::select! {
        result = tracker.start() => result,
        _ = tokio::signal::ctrl_c() => {
            eprintln!("received Ctrl-C, shutting down...");
            Ok(())
        }
    }
}

async fn run_aligned_tracker<L: StateSource, R: StateSource, D: Differ, E: AlignmentKeyExtractor>(
    tracker: AlignedTracker<L, R, D, E>
) -> Result<(), TrackerError> {
    tokio::select! {
        result = tracker.start() => result,
        _ = tokio::signal::ctrl_c() => {
            eprintln!("received Ctrl-C, shutting down...");
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() {
    // logging
    let _ = fmt().with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap())).try_init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Diff { left_url, right_url, pretty, engine } => {
            let left = WebSocketSource::new("left", left_url);
            let right = WebSocketSource::new("right", right_url);
            let differ = JsonPatchDiffer::new(pretty, engine.into());
            let tracker = Tracker::new(left, right, differ);
            run_tracker(tracker).await
        }
        Commands::Track {
            left_url,
            right_url,
            align_by,
            round_end,
            visual,
            report,
            pretty,
            engine,
            once,
            max_rounds
        } => {
            // Validate: --report requires --round-end
            if report.is_some() && round_end.is_none() {
                eprintln!("error: --report requires --round-end to be set");
                eprintln!(
                    "The report is generated at the end of each round, so a round completion signal is required."
                );
                eprintln!("\nExample:");
                eprintln!("  cargo run -- track <urls> --align-by phase --round-end GameCleared --report output.html");
                std::process::exit(1);
            }

            // Resolve max_rounds: --once takes precedence
            let final_max_rounds = if once { Some(1) } else { max_rounds };

            let left = WebSocketSource::new("left", left_url);
            let right = WebSocketSource::new("right", right_url);
            let differ = JsonPatchDiffer::new(pretty, engine.into());
            let extractor = JsonPathExtractor::new(&align_by);
            let mut tracker =
                AlignedTracker::new(left, right, differ, extractor).with_visual(visual).with_pretty_diff(pretty);

            if let Some(signal) = round_end {
                tracker = tracker.with_round_end_signal(signal);
            }

            if let Some(output) = report {
                tracker = tracker.with_report_output(output);
            }

            if let Some(max) = final_max_rounds {
                tracker = tracker.with_max_rounds(max);
            }

            run_aligned_tracker(tracker).await
        }
        Commands::Example {
            left_interval,
            right_interval,
            pretty,
            engine,
            align_by,
            round_end,
            visual,
            report,
            once,
            max_rounds
        } => {
            let left = RandomStream::new("left", left_interval);
            let right = RandomStream::new("right", right_interval);
            let differ = JsonPatchDiffer::new(pretty, engine.into());

            match align_by {
                Some(field) => {
                    // Validate: --report requires --round-end
                    if report.is_some() && round_end.is_none() {
                        eprintln!("error: --report requires --round-end to be set");
                        eprintln!(
                            "The report is generated at the end of each round, so a round completion signal is \
                             required."
                        );
                        eprintln!("\nExample:");
                        eprintln!(
                            "  cargo run -- example --align-by event_type --round-end order.completed --report \
                             output.html"
                        );
                        std::process::exit(1);
                    }

                    let extractor = JsonPathExtractor::new(&field);
                    let mut tracker = AlignedTracker::new(left, right, differ, extractor)
                        .with_visual(visual)
                        .with_pretty_diff(pretty);

                    if let Some(signal) = round_end {
                        tracker = tracker.with_round_end_signal(signal);
                    }

                    if let Some(output) = report {
                        tracker = tracker.with_report_output(output);
                    }

                    // Resolve max_rounds: --once takes precedence
                    let final_max_rounds = if once { Some(1) } else { max_rounds };
                    if let Some(max) = final_max_rounds {
                        tracker = tracker.with_max_rounds(max);
                    }

                    run_aligned_tracker(tracker).await
                }
                None => {
                    let tracker = Tracker::new(left, right, differ);
                    run_tracker(tracker).await
                }
            }
        }
    };

    if let Err(err) = result {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
