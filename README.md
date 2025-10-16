# State Tracker 🔄

A powerful tool for tracking and comparing JSON state streams from dual WebSocket sources in real-time. Perfect for validating API compatibility during migrations, and multi-source synchronization.

## Features

- **Dual WebSocket Streaming**: Connect to two WebSocket URLs simultaneously
- **Phase-Aligned Tracking**: Align and compare states based on custom JSON fields
- **Round-Based Comparison**: Define round boundaries and compare complete game sessions
- **Multiple Output Modes**: Choose between visual timeline, pretty diffs, or structured logs
- **HTML Reports**: Generate beautiful, standalone HTML reports with interactive visualizations
- **Multiple Diff Engines**: Choose between `json-patch` and `serde_json_diff`
- **Smart Output**: Automatically selects best display mode based on flags

## Installation

```bash
cargo build --release
```

## Usage

### 1. **Immediate Diff Mode**
Compare states as they arrive, without alignment:

```bash
# Basic usage
cargo run -- diff ws://left-server ws://right-server

# With pretty output
cargo run -- diff ws://left-server ws://right-server --pretty

# Using serde_json_diff engine
cargo run -- diff ws://left-server ws://right-server --engine serde-diff
```

### 2. **Phase-Aligned Tracking** (Recommended for Game Migrations)
Align states by a specific JSON field before comparing:

```bash
# Align by a top-level field
cargo run -- track ws://old-engine ws://new-engine --align-by event_type

# Align by nested field (dot notation)
cargo run -- track ws://old-engine ws://new-engine --align-by message.type

# With visual timeline
cargo run -- track ws://old-engine ws://new-engine --align-by phase --visual

# Generate HTML report
cargo run -- track ws://old-engine ws://new-engine --align-by phase --report report.html
```

### 3. **Round-Based Comparison**
Wait for complete rounds (e.g., game sessions) before comparing:

```bash
# Define round completion signal
cargo run -- track \
  ws://old-engine ws://new-engine \
  --align-by phase \
  --round-end GameCleared

# With visual output
cargo run -- track \
  ws://old-engine ws://new-engine \
  --align-by phase \
  --round-end GameCleared \
  --visual

# Generate HTML report for entire session
cargo run -- track \
  ws://old-engine ws://new-engine \
  --align-by phase \
  --round-end GameCleared \
  --report session-report.html
```

**How it works:**
- Buffers all states from both sources
- Waits for both sides to receive the `--round-end` signal (e.g., `"GameCleared"`)
- Compares all states in the round, matching by alignment key
- Reports mismatches, missing states, and extra states
- Clears buffers and repeats for next round

### 4. **Example Mode** (Testing)
Generate random JSON streams for testing:

```bash
# Basic example (immediate diff mode)
cargo run -- example

# With alignment and visual timeline
cargo run -- example --align-by event_type --visual

# Custom stream intervals (milliseconds)
cargo run -- example \
  --left-interval 500 \
  --right-interval 1000 \
  --align-by event_type \
  --visual \

# With round completion (when event_type matches specific value)
cargo run -- example \
  --align-by event_type \
  --round-end order.completed \
  --visual
```

**Note**: Visual mode requires `--align-by` flag to enable phase-aligned tracking.

## Casino Game Migration Example

Perfect use case: Migrating a casino game from one engine to another while ensuring 100% state compatibility.

```bash
cargo run -- track \
  ws://old-game-engine.example.com/game/slots \
  ws://new-game-engine.example.com/game/slots \
  --align-by message.phase \
  --round-end GameCleared \
  --visual \
  --pretty
```

**Expected flow:**
```
GameStarted → Balance → Balance → Balance → GameCleared
```

The tracker will:
1. ✅ Collect all states from both engines
2. ⏳ Wait for `GameCleared` from both sides
3. 📊 Compare state-by-state alignment
4. 🎯 Show visual timeline and differences
5. 🔄 Reset and track the next round

## Visual Output

When using `--visual`, you get a real-time ASCII dashboard:

```
═══════════════════════════════════════════════════════════════════
🔄 STATE TRACKER  (Live View)
═══════════════════════════════════════════════════════════════════

   # │ LEFT STREAM                              │ RIGHT STREAM
─────┼──────────────────────────────────────────┼──────────────────────────────────────
   ✓ │ GameStarted                              │ GameStarted
   ✓ │ Balance                                  │ Balance
   ✗ │ Balance                                  │ GameCleared
   
─────────────────────────────────────────────────────────────────────
⏳ WAITING: left=Balance ≠ right=GameCleared
─────────────────────────────────────────────────────────────────────
ℹ  Press Ctrl-C to exit
```

## Output Modes

The tracker automatically selects the best output mode based on your flags:

1. **Visual Mode** (`--visual`) - Priority 1
   - Live ASCII timeline with color-coded streams
   - Real-time alignment status
   - Best for: Monitoring active sessions

2. **Pretty Diff Mode** (`--pretty`) - Priority 2
   - Clean, formatted diffs with progress indicators
   - Shows `⏳ Waiting...` with a spinner effect
   - Best for: Detailed state comparison

3. **Structured Logs Mode** (default) - Priority 3
   - Timestamped logs with alignment context
   - Shows sync status: `⏳ out of sync`, `✓ aligned`
   - Best for: Piping to log aggregators or debugging

**Note**: Only ONE mode is active at a time. Priority: `--visual` > `--pretty` > default logs.

## CLI Options

| Flag | Description | Example |
|------|-------------|---------|
| `--align-by` | JSON field path for alignment | `type`, `message.phase` |
| `--round-end` | Signal value marking round completion | `GameCleared`, `session.end` |
| `--once` | Stop after tracking one complete round | (flag) |
| `--max-rounds` | Maximum number of rounds to track | `--max-rounds 5` |
| `--visual` | Enable visual timeline display (Priority 1) | (flag) |
| `--pretty` | Enable pretty diff output (Priority 2) | (flag) |
| `--report` | Generate HTML report to file (requires `--round-end`) | `--report output.html` |
| `--engine` | Diff engine: `json-patch` or `serde-diff` | `--engine serde-diff` |

## How Round Synchronization Works

When you use `--round-end`, the tracker implements a **dual-flag synchronization** mechanism:

### Synchronization Logic

1. **Both streams accumulate events** into separate buffers
2. **When left stream receives the signal** (e.g., `order.completed`):
   - Sets `left_round_complete = true`
   - Continues listening
3. **When right stream receives the signal**:
   - Sets `right_round_complete = true`
   - Continues listening
4. **Only when BOTH flags are true**:
   - ✅ Compares all accumulated states from both buffers
   - 📊 Generates timestamped HTML report (if `--report` is set)
   - 🔄 Clears buffers and resets flags for the next round
   - 🏁 Exits if `--once` or `--max-rounds` reached

### Example Timeline

```
Time    Left Stream              Right Stream           Action
────────────────────────────────────────────────────────────────────
0.1s    user.login              -                      (buffering)
0.2s    -                       user.login             (buffering)
0.3s    order.created           -                      (buffering)
0.5s    -                       order.created          (buffering)
0.7s    order.completed ✓       -                      left_flag=true
        (waiting for right...)
1.0s    -                       order.completed ✓      right_flag=true
        🎯 BOTH COMPLETE → Compare & Generate Report
        📄 Round 1 report saved: output_20251008_210006.html
        🔄 Buffers cleared, ready for Round 2
```

### Key Benefits

- **No race conditions**: Waits for both streams before comparing
- **Complete round capture**: All events between signals are preserved
- **Multiple rounds**: Automatically continues to next round
- **Perfect for async systems**: Handles different timing/latency gracefully

## HTML Report Output

Generate beautiful, shareable HTML reports with `--report` (requires `--round-end`):

```bash
cargo run -- example \
  --align-by event_type \
  --round-end order.completed \
  --report demo.html

# Opens in browser to see:
# - Interactive timeline view
# - Comparison table with status badges
# - Raw JSON data viewer
# - Match/mismatch statistics
```

**Note**: Reports are generated when rounds complete, so `--round-end` must be specified.

The HTML report includes:
- 📊 **Statistics Dashboard**: Match/mismatch counts, state totals
- 🎨 **Timeline Visualization**: Color-coded side-by-side state flow
- 📋 **Comparison Table**: Sortable table with status indicators
- 💾 **Raw Data**: JSON viewer for deep inspection
- 🎯 **Self-Contained**: Single HTML file, no dependencies

## License

MIT

