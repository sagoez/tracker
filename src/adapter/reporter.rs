use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs::File;
use std::io::Write;

use crate::domain::State;

#[derive(Serialize, Clone)]
struct TimelineEvent {
    side: String,        // "left" or "right"
    key: String,
    timestamp: String,
    timestamp_ms: i64,
    data: String,
    index: usize,
}

#[derive(Serialize)]
struct ReportState {
    key: String,
    timestamp: String,
    data: String,
}

pub struct HtmlReporter {
    session_id: String,
    started_at: DateTime<Utc>,
    left_states: Vec<State>,
    right_states: Vec<State>,
}

impl HtmlReporter {
    pub fn new() -> Self {
        Self {
            session_id: uuid::Uuid::new_v4().to_string(),
            started_at: Utc::now(),
            left_states: Vec::new(),
            right_states: Vec::new(),
        }
    }

    pub fn add_left(&mut self, state: State) {
        self.left_states.push(state);
    }

    pub fn add_right(&mut self, state: State) {
        self.right_states.push(state);
    }

    pub fn generate(&self, output_path: &str) -> std::io::Result<()> {
        let mut file = File::create(output_path)?;
        
        let html = self.build_html();
        file.write_all(html.as_bytes())?;
        
        Ok(())
    }

    fn build_timeline_events(&self) -> Vec<TimelineEvent> {
        let mut events = Vec::new();

        for (i, state) in self.left_states.iter().enumerate() {
            events.push(TimelineEvent {
                side: "left".to_string(),
                key: state.alignment_key.clone().unwrap_or_else(|| "<no-key>".to_string()),
                timestamp: state.timestamp.format("%H:%M:%S%.3f").to_string(),
                timestamp_ms: state.timestamp.timestamp_millis(),
                data: serde_json::to_string_pretty(&state.data).unwrap_or_default(),
                index: i,
            });
        }

        for (i, state) in self.right_states.iter().enumerate() {
            events.push(TimelineEvent {
                side: "right".to_string(),
                key: state.alignment_key.clone().unwrap_or_else(|| "<no-key>".to_string()),
                timestamp: state.timestamp.format("%H:%M:%S%.3f").to_string(),
                timestamp_ms: state.timestamp.timestamp_millis(),
                data: serde_json::to_string_pretty(&state.data).unwrap_or_default(),
                index: i,
            });
        }

        // Sort by timestamp
        events.sort_by_key(|e| e.timestamp_ms);
        events
    }

    fn build_html(&self) -> String {
        let timeline_json = serde_json::to_string(&self.build_timeline_events()).unwrap_or_else(|_| "[]".to_string());
        let left_states_json = self.states_to_json(&self.left_states);
        let right_states_json = self.states_to_json(&self.right_states);

        format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>State Tracker Report</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 2rem;
        }}
        
        .container {{
            max-width: 1600px;
            margin: 0 auto;
            background: white;
            border-radius: 20px;
            box-shadow: 0 20px 60px rgba(0,0,0,0.3);
            overflow: hidden;
        }}
        
        .header {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 3rem 2rem;
            text-align: center;
        }}
        
        .header h1 {{
            font-size: 3rem;
            margin-bottom: 1rem;
            font-weight: 800;
        }}
        
        .header .meta {{
            opacity: 0.95;
            font-size: 1rem;
            margin-top: 0.5rem;
        }}
        
        .stats {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
            gap: 1.5rem;
            padding: 2rem;
            background: #f8f9fa;
        }}
        
        .stat-card {{
            background: white;
            padding: 2rem;
            border-radius: 12px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.08);
            text-align: center;
            transition: transform 0.2s;
        }}
        
        .stat-card:hover {{
            transform: translateY(-2px);
            box-shadow: 0 6px 16px rgba(0,0,0,0.12);
        }}
        
        .stat-value {{
            font-size: 3rem;
            font-weight: 800;
            color: #667eea;
            margin-bottom: 0.5rem;
        }}
        
        .stat-label {{
            color: #6c757d;
            font-size: 1rem;
            font-weight: 500;
        }}
        
        .tabs {{
            display: flex;
            background: #f8f9fa;
            border-bottom: 3px solid #dee2e6;
        }}
        
        .tab {{
            flex: 1;
            padding: 1.2rem 2rem;
            background: none;
            border: none;
            cursor: pointer;
            font-size: 1.1rem;
            font-weight: 700;
            color: #6c757d;
            transition: all 0.3s;
            position: relative;
        }}
        
        .tab:hover {{
            background: rgba(102, 126, 234, 0.1);
            color: #667eea;
        }}
        
        .tab.active {{
            color: #667eea;
            background: white;
        }}
        
        .tab.active::after {{
            content: '';
            position: absolute;
            bottom: -3px;
            left: 0;
            right: 0;
            height: 3px;
            background: #667eea;
        }}
        
        .tab-content {{
            display: none;
            padding: 2rem;
            min-height: 500px;
        }}
        
        .tab-content.active {{
            display: block;
        }}
        
        /* Chronological Timeline */
        .chrono-timeline {{
            max-width: 1400px;
            margin: 0 auto;
            padding: 2rem;
            position: relative;
        }}
        
        /* Left track line */
        .chrono-timeline::before {{
            content: '';
            position: absolute;
            left: calc(50% - 15px);
            top: 0;
            bottom: 0;
            width: 2px;
            background: linear-gradient(180deg, transparent 0%, #667eea 5%, #667eea 95%, transparent 100%);
            z-index: 0;
        }}
        
        /* Right track line */
        .chrono-timeline::after {{
            content: '';
            position: absolute;
            left: calc(50% + 15px);
            top: 0;
            bottom: 0;
            width: 2px;
            background: linear-gradient(180deg, transparent 0%, #f093fb 5%, #f093fb 95%, transparent 100%);
            z-index: 0;
        }}
        
        .chrono-item {{
            display: grid;
            grid-template-columns: 1fr 140px 1fr;
            gap: 2rem;
            margin-bottom: 3rem;
            align-items: start;
            position: relative;
            z-index: 1;
        }}
        
        .chrono-spacer {{
            /* Empty cell for alignment */
        }}
        
        .chrono-marker {{
            display: flex;
            flex-direction: column;
            align-items: center;
            position: relative;
            z-index: 2;
        }}
        
        .chrono-dot {{
            width: 16px;
            height: 16px;
            border-radius: 50%;
            box-shadow: 0 0 0 4px white, 0 2px 8px rgba(0,0,0,0.15);
            margin-bottom: 0.75rem;
            position: relative;
            z-index: 3;
        }}
        
        .chrono-dot.left {{
            background: #667eea;
        }}
        
        .chrono-dot.right {{
            background: #f093fb;
        }}
        
        .chrono-time {{
            font-size: 0.875rem;
            color: #4a5568;
            font-weight: 600;
            white-space: nowrap;
            background: white;
            padding: 0.5rem 1rem;
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
            border: 1px solid #e2e8f0;
        }}
        
        .event-card {{
            padding: 1.5rem;
            border-radius: 12px;
            box-shadow: 0 4px 16px rgba(0,0,0,0.12);
            cursor: pointer;
            transition: all 0.3s;
            position: relative;
        }}
        
        .event-card:hover {{
            transform: translateY(-4px);
            box-shadow: 0 8px 24px rgba(0,0,0,0.15);
        }}
        
        .event-card.left {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
        }}
        
        .event-card.right {{
            background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
            color: white;
        }}
        
        .event-key {{
            font-size: 1.3rem;
            font-weight: 700;
            margin-bottom: 0.5rem;
        }}
        
        .event-badge {{
            display: inline-block;
            padding: 0.25rem 0.75rem;
            border-radius: 20px;
            font-size: 0.75rem;
            font-weight: 600;
            background: rgba(255,255,255,0.2);
            margin-right: 0.5rem;
        }}
        
        .event-data {{
            margin-top: 1rem;
            padding: 1rem;
            background: rgba(0,0,0,0.2);
            border-radius: 8px;
            font-family: 'Monaco', 'Courier New', monospace;
            font-size: 0.85rem;
            line-height: 1.5;
            max-height: 0;
            overflow: hidden;
            transition: max-height 0.3s ease;
        }}
        
        .event-card.expanded .event-data {{
            max-height: 800px;
            overflow: auto;
        }}
        
        .expand-hint {{
            text-align: center;
            font-size: 0.8rem;
            opacity: 0.7;
            margin-top: 0.5rem;
        }}
        
        /* Matching View */
        .matching-grid {{
            display: flex;
            flex-direction: column;
            gap: 2rem;
            max-width: 1400px;
            margin: 0 auto;
        }}
        
        .match-row {{
            display: grid;
            grid-template-columns: 1fr 100px 1fr;
            gap: 2rem;
            align-items: center;
        }}
        
        .match-card {{
            padding: 1.5rem;
            border-radius: 12px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.1);
        }}
        
        .match-card.left {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            text-align: right;
        }}
        
        .match-card.right {{
            background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
            color: white;
        }}
        
        .match-card.empty {{
            background: #f8f9fa;
            color: #adb5bd;
            text-align: center;
            border: 2px dashed #dee2e6;
        }}
        
        .match-key {{
            font-size: 1.2rem;
            font-weight: 700;
            margin-bottom: 0.5rem;
        }}
        
        .match-time {{
            font-size: 0.9rem;
            opacity: 0.85;
        }}
        
        .match-indicator {{
            width: 70px;
            height: 70px;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 2rem;
            font-weight: bold;
            box-shadow: 0 4px 16px rgba(0,0,0,0.2);
            margin: 0 auto;
        }}
        
        .match-indicator.match {{
            background: linear-gradient(135deg, #28a745 0%, #20c997 100%);
            color: white;
        }}
        
        .match-indicator.mismatch {{
            background: linear-gradient(135deg, #dc3545 0%, #fd7e14 100%);
            color: white;
        }}
        
        .match-indicator.missing {{
            background: linear-gradient(135deg, #ffc107 0%, #fd7e14 100%);
            color: white;
        }}
        
        .footer {{
            text-align: center;
            padding: 2rem;
            color: #6c757d;
            background: #f8f9fa;
            font-size: 0.9rem;
        }}
        
        .footer a {{
            color: #667eea;
            text-decoration: none;
            font-weight: 600;
        }}
        
        .footer a:hover {{
            text-decoration: underline;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🔄 State Tracker</h1>
            <div class="meta">
                <div>Session: {session_id}</div>
                <div>Generated: {timestamp}</div>
            </div>
        </div>
        
        <div class="stats">
            <div class="stat-card">
                <div class="stat-value">{left_count}</div>
                <div class="stat-label">Left States</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{right_count}</div>
                <div class="stat-label">Right States</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{matched}</div>
                <div class="stat-label">Matched</div>
            </div>
            <div class="stat-card">
                <div class="stat-value">{mismatched}</div>
                <div class="stat-label">Mismatched</div>
            </div>
        </div>
        
        <div class="tabs">
            <button class="tab active" onclick="showTab('timeline')">📊 Timeline</button>
            <button class="tab" onclick="showTab('matching')">🔗 Matching View</button>
        </div>
        
        <div id="timeline-tab" class="tab-content active">
            <div class="chrono-timeline" id="timeline"></div>
        </div>
        
        <div id="matching-tab" class="tab-content">
            <div class="matching-grid" id="matching"></div>
        </div>
        
        <div class="footer">
            Generated by State Tracker • <a href="https://github.com/sagoez/tracker">GitHub</a>
        </div>
    </div>
    
    <script>
        const timelineEvents = {timeline_json};
        const leftStates = {left_states_json};
        const rightStates = {right_states_json};
        
        function showTab(tabName) {{
            document.querySelectorAll('.tab').forEach(tab => tab.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(content => content.classList.remove('active'));
            
            event.target.classList.add('active');
            document.getElementById(tabName + '-tab').classList.add('active');
        }}
        
        function renderTimeline() {{
            const timeline = document.getElementById('timeline');
            
            timelineEvents.forEach((event, i) => {{
                const item = document.createElement('div');
                item.className = 'chrono-item';
                
                // Timeline marker (always in center)
                const marker = document.createElement('div');
                marker.className = 'chrono-marker';
                marker.innerHTML = `
                    <div class="chrono-dot ${{event.side}}"></div>
                    <div class="chrono-time">${{event.timestamp}}</div>
                `;
                
                // Event card
                const card = document.createElement('div');
                card.className = `event-card ${{event.side}}`;
                card.innerHTML = `
                    <div class="event-key">${{event.key}}</div>
                    <div>
                        <span class="event-badge">${{event.side.toUpperCase()}}</span>
                        <span class="event-badge">#${{event.index + 1}}</span>
                    </div>
                    <div class="event-data">${{escapeHtml(event.data)}}</div>
                    <div class="expand-hint">Click to expand JSON</div>
                `;
                
                card.addEventListener('click', () => {{
                    card.classList.toggle('expanded');
                }});
                
                // Empty spacer div
                const spacer = document.createElement('div');
                spacer.className = 'chrono-spacer';
                
                // Arrange in grid: left card | marker | spacer OR spacer | marker | right card
                if (event.side === 'left') {{
                    item.appendChild(card);
                    item.appendChild(marker);
                    item.appendChild(spacer);
                }} else {{
                    item.appendChild(spacer);
                    item.appendChild(marker);
                    item.appendChild(card);
                }}
                
                timeline.appendChild(item);
            }});
        }}
        
        function renderMatching() {{
            const matching = document.getElementById('matching');
            const maxLength = Math.max(leftStates.length, rightStates.length);
            
            for (let i = 0; i < maxLength; i++) {{
                const left = leftStates[i];
                const right = rightStates[i];
                
                const row = document.createElement('div');
                row.className = 'match-row';
                
                // Left card
                const leftCard = document.createElement('div');
                if (left) {{
                    leftCard.className = 'match-card left';
                    leftCard.innerHTML = `
                        <div class="match-key">${{left.key}}</div>
                        <div class="match-time">${{left.timestamp}}</div>
                    `;
                }} else {{
                    leftCard.className = 'match-card empty';
                    leftCard.innerHTML = '<div>—</div>';
                }}
                
                // Indicator
                const indicator = document.createElement('div');
                const status = getStatus(left?.key, right?.key);
                indicator.className = `match-indicator ${{status}}`;
                indicator.textContent = status === 'match' ? '✓' : status === 'mismatch' ? '✗' : '⚠';
                
                // Right card
                const rightCard = document.createElement('div');
                if (right) {{
                    rightCard.className = 'match-card right';
                    rightCard.innerHTML = `
                        <div class="match-key">${{right.key}}</div>
                        <div class="match-time">${{right.timestamp}}</div>
                    `;
                }} else {{
                    rightCard.className = 'match-card empty';
                    rightCard.innerHTML = '<div>—</div>';
                }}
                
                row.appendChild(leftCard);
                row.appendChild(indicator);
                row.appendChild(rightCard);
                matching.appendChild(row);
            }}
        }}
        
        function getStatus(leftKey, rightKey) {{
            if (!leftKey || !rightKey) return 'missing';
            return leftKey === rightKey ? 'match' : 'mismatch';
        }}
        
        function escapeHtml(text) {{
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }}
        
        renderTimeline();
        renderMatching();
    </script>
</body>
</html>"#,
            session_id = self.session_id,
            timestamp = self.started_at.format("%Y-%m-%d %H:%M:%S UTC"),
            left_count = self.left_states.len(),
            right_count = self.right_states.len(),
            matched = self.count_matched(),
            mismatched = self.count_mismatched(),
            timeline_json = timeline_json,
            left_states_json = left_states_json,
            right_states_json = right_states_json,
        )
    }

    fn states_to_json(&self, states: &[State]) -> String {
        let report_states: Vec<ReportState> = states
            .iter()
            .map(|s| ReportState {
                key: s.alignment_key.clone().unwrap_or_else(|| "<no-key>".to_string()),
                timestamp: s.timestamp.format("%H:%M:%S%.3f").to_string(),
                data: serde_json::to_string(&s.data).unwrap_or_default(),
            })
            .collect();

        serde_json::to_string(&report_states).unwrap_or_else(|_| "[]".to_string())
    }

    fn count_matched(&self) -> usize {
        let max_len = self.left_states.len().min(self.right_states.len());
        (0..max_len)
            .filter(|&i| {
                self.left_states[i].alignment_key == self.right_states[i].alignment_key
                    && self.left_states[i].alignment_key.is_some()
            })
            .count()
    }

    fn count_mismatched(&self) -> usize {
        let max_len = self.left_states.len().min(self.right_states.len());
        (0..max_len)
            .filter(|&i| {
                let left = &self.left_states[i].alignment_key;
                let right = &self.right_states[i].alignment_key;
                left.is_some() && right.is_some() && left != right
            })
            .count()
    }
}

impl Default for HtmlReporter {
    fn default() -> Self {
        Self::new()
    }
}
