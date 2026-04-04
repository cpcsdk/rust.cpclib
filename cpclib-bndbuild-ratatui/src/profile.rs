//! Self-contained HTML build-profile report.
//!
//! Produces a single standalone HTML file with:
//!  - A Gantt-style SVG timeline showing when each rule ran (relative to build
//!    start), coloured by outcome.  Hover any bar for a tooltip with the full
//!    rule name, duration, and per-task breakdown.
//!  - A table sorted by duration (slowest first) for forensic analysis, with
//!    a proportional bar, share of total build time, and slowest-task column.
//!
//! HTML generation uses the `minijinja` template engine (already in the
//! workspace). `build_gantt` and `build_table` produce SVG/HTML strings that
//! are injected into the Jinja2 template as `{{ gantt }}` and `{{ table }}`.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::model::{RuleEntry, RuleStatus, TaskStatus};

// ─── Public entry point ───────────────────────────────────────────────────────

/// Generate and write the HTML profile to `path`.
pub(crate) fn save_profile(
    rules: &[RuleEntry],
    build_started: Instant,
    total_duration: Duration,
    path: &Path
) -> std::io::Result<()> {
    let html = build_html_minijinja(rules, build_started, total_duration);
    std::fs::write(path, html.as_bytes())
}

// ─── Internal data model ──────────────────────────────────────────────────────

struct TaskRow {
    name: String,
    start: Duration, // offset from build_started
    dur: Duration,
    color: &'static str
}

struct RuleRow {
    name: String,
    start: Duration, // offset from build_started
    dur: Duration,
    color: &'static str,
    status: &'static str,
    tasks: Vec<TaskRow>
}

fn collect_rows(rules: &[RuleEntry], build_started: Instant) -> Vec<RuleRow> {
    rules
        .iter()
        .map(|r| {
            let start = r
                .started
                .checked_duration_since(build_started)
                .unwrap_or(Duration::ZERO);

            let (dur, color, status) = match &r.status {
                RuleStatus::Success(d) => (*d, "#a6e3a1", "ok"),
                RuleStatus::Failed(d) => (*d, "#f38ba8", "FAILED"),
                RuleStatus::UpToDate => (Duration::ZERO, "#89dceb", "up-to-date"),
                RuleStatus::Running => (r.started.elapsed(), "#f9e2af", "running")
            };

            let tasks = r
                .tasks
                .iter()
                .map(|t| {
                    let task_start = t
                        .started
                        .checked_duration_since(build_started)
                        .unwrap_or(Duration::ZERO);
                    let (d, c) = match &t.status {
                        TaskStatus::Success(d) => (*d, "#a6e3a1"),
                        TaskStatus::Failed(d) => (*d, "#f38ba8"),
                        TaskStatus::Running => (t.started.elapsed(), "#f9e2af")
                    };
                    TaskRow {
                        name: t.task.clone(),
                        start: task_start,
                        dur: d,
                        color: c
                    }
                })
                .collect();

            RuleRow {
                name: r.name.clone(),
                start,
                dur,
                color,
                status,
                tasks
            }
        })
        .collect()
}

// ─── Formatting helpers ───────────────────────────────────────────────────────

fn fmt_dur(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    }
    else {
        let s = d.as_secs_f64();
        if s < 100.0 {
            format!("{:.2}s", s)
        }
        else {
            format!("{:.0}s", s)
        }
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Truncate long names from the left so the tail (filename / leaf) is visible.
fn truncate_left(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        s.to_owned()
    }
    else {
        let tail: String = s
            .chars()
            .rev()
            .take(max - 1)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("\u{2026}{}", tail) // …tail
    }
}

// ─── HTML template ───────────────────────────────────────────────────────────
// CSS lives in the template; Gantt SVG and table HTML are injected as variables.
// Using HTML entities for special chars keeps the template free of Rust escapes.

const TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>bndbuild &#183; Build Profile</title>
<style>
:root {
  --bg:#1e1e2e; --bg2:#181825; --surface:#313244; --overlay:#45475a;
  --text:#cdd6f4; --sub:#a6adc8;
  --blue:#89b4fa; --green:#a6e3a1; --red:#f38ba8;
  --yellow:#f9e2af; --sky:#89dceb;
}
* { box-sizing:border-box; margin:0; padding:0; }
body {
  background:var(--bg); color:var(--text);
  font-family:'Cascadia Code','JetBrains Mono',ui-monospace,monospace;
  font-size:13px; padding:28px 32px; line-height:1.5;
}
h1 { color:var(--blue); font-size:18px; margin-bottom:6px; letter-spacing:.3px; }
.summary { color:var(--sub); margin-bottom:28px; }
.summary b { color:var(--text); }
h2 {
  color:var(--blue); font-size:11px; text-transform:uppercase;
  letter-spacing:1.5px; margin:28px 0 10px;
  border-bottom:1px solid var(--surface); padding-bottom:5px;
}
.chart-wrap { overflow-x:auto; border-radius:8px; margin-bottom:6px; }
table { border-collapse:collapse; width:100%; margin-top:4px; }
thead th {
  background:var(--surface); padding:7px 14px; text-align:left;
  color:var(--blue); font-size:11px; text-transform:uppercase;
  letter-spacing:.8px; white-space:nowrap;
  border-bottom:1px solid var(--overlay);
}
tbody td { padding:6px 14px; border-bottom:1px solid var(--surface); vertical-align:middle; }
tbody tr:hover td { background:var(--surface); }
td.rank { color:var(--overlay); font-size:11px; text-align:right; padding-right:8px; width:32px; }
td.name { max-width:420px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
td.dur  { color:var(--yellow); font-weight:bold; white-space:nowrap; padding-right:6px; }
td.pct  { color:var(--sub); font-size:11px; min-width:50px; white-space:nowrap; }
td.tc   { color:var(--sub); text-align:center; white-space:nowrap; }
td.slow { color:var(--sub); max-width:380px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.bar-wrap { background:var(--bg2); border-radius:3px; height:8px; min-width:60px; max-width:260px; margin-top:5px; }
.bar     { height:100%; border-radius:3px; min-width:2px; }
.s-ok       { color:var(--green); }
.s-failed   { color:var(--red); font-weight:bold; }
.s-uptodate { color:var(--sky); }
.s-running  { color:var(--yellow); }
</style>
</head>
<body>
<h1>&#9935; bndbuild &#183; Build Profile</h1>
<div class="summary">
  <b>Total:</b> {{ total_str }} &nbsp;&#183;&nbsp;
  <b>Rules:</b> {{ n_rules }} &nbsp;&#183;&nbsp;
  {%- if n_failed > 0 %} <span class='s-failed'>&#9888; {{ n_failed }} FAILED</span>
  {%- else %} <span class='s-ok'>&#10004; all OK</span>
  {%- endif %}
</div>
<h2>&#8987; Timeline</h2>
<div class="chart-wrap">{{ gantt }}</div>
<h2>&#128202; Duration &#8212; slowest first</h2>
{{ table }}
</body>
</html>"#;

/// Render the full HTML report using the minijinja template above.
/// `build_gantt` and `build_table` supply the pre-rendered SVG / HTML pieces.
fn build_html_minijinja(rules: &[RuleEntry], build_started: Instant, total: Duration) -> String {
    let rows = collect_rows(rules, build_started);
    let n_failed = rows.iter().filter(|r| r.status == "FAILED").count();
    let gantt = build_gantt(&rows, total);
    let table = build_table(&rows, total);
    let ctx = minijinja::context! {
        total_str => fmt_dur(total),
        n_rules   => rows.len(),
        n_failed  => n_failed,
        gantt     => gantt,
        table     => table,
    };
    let env = minijinja::Environment::new();
    env.render_str(TEMPLATE, ctx)
        .unwrap_or_else(|e| format!("<pre>Template error: {e}</pre>"))
}

// ─── SVG Gantt chart ─────────────────────────────────────────────────────────

fn build_gantt(rows: &[RuleRow], total: Duration) -> String {
    const LABEL_W: i64 = 280;
    const CHART_W: i64 = 880;
    const ROW_H: i64 = 26;
    const PAD_X: i64 = 12;
    const PAD_Y: i64 = 8;
    const TICK_ROW: i64 = 22;

    // Guard against zero-duration builds (avoids div/0).
    let total_ns = total.as_nanos().max(1) as f64;
    let total_secs = total.as_secs_f64().max(1e-9);
    let n = rows.len() as i64;

    let svg_w = PAD_X + LABEL_W + CHART_W + PAD_X;
    let svg_h = PAD_Y + TICK_ROW + n * ROW_H + PAD_Y;

    // Choose a tick interval such that at most 14 ticks are shown.
    let tick_secs = [
        0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0
    ]
    .iter()
    .copied()
    .find(|&t| total_secs / t <= 14.0)
    .unwrap_or(600.0);

    let chart_x0 = PAD_X + LABEL_W;
    let chart_y0 = PAD_Y + TICK_ROW;

    let mut s = String::new();
    macro_rules! w { ($($a:tt)*) => { s.push_str(&format!($($a)*)); }; }

    w!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" \
         style=\"font-family:inherit\">",
        svg_w,
        svg_h
    );
    w!("<rect width=\"100%\" height=\"100%\" fill=\"#181825\" rx=\"6\"/>");

    // Vertical separator between label and chart areas.
    w!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" \
         stroke=\"#45475a\" stroke-width=\"1\"/>",
        chart_x0,
        PAD_Y,
        chart_x0,
        svg_h - PAD_Y
    );

    // X-axis grid lines and labels.
    let mut t = 0.0f64;
    while t <= total_secs * 1.001 {
        let x = chart_x0 as f64 + (t / total_secs) * CHART_W as f64;
        w!(
            "<line x1=\"{x:.1}\" y1=\"{}\" x2=\"{x:.1}\" y2=\"{}\" \
             stroke=\"#313244\" stroke-width=\"1\"/>",
            chart_y0,
            svg_h - PAD_Y
        );
        let label = if tick_secs >= 1.0 {
            format!("{}s", t as u64)
        }
        else {
            format!("{:.1}s", t)
        };
        w!(
            "<text x=\"{x:.1}\" y=\"{}\" fill=\"#585b70\" text-anchor=\"middle\" \
             font-size=\"11\">{}</text>",
            PAD_Y + TICK_ROW - 5,
            label
        );
        t += tick_secs;
    }

    // Rule rows.
    for (i, row) in rows.iter().enumerate() {
        let iy = i as i64;
        let row_top = chart_y0 + iy * ROW_H;
        let bar_y = row_top + 4;
        let bar_h = ROW_H - 8;
        let mid_y = bar_y + bar_h / 2;

        // Alternating stripe on the label area.
        if i % 2 == 1 {
            w!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" \
                 fill=\"#1e1e2e\" opacity=\"0.45\"/>",
                PAD_X,
                row_top,
                LABEL_W,
                ROW_H
            );
        }

        // Right-aligned label in the label area.
        let display = esc(&truncate_left(&row.name, 36));
        w!(
            "<text x=\"{}\" y=\"{}\" fill=\"#cdd6f4\" text-anchor=\"end\" \
             dominant-baseline=\"middle\" font-size=\"12\">\
             <title>{}</title>{}</text>",
            chart_x0 - 6,
            mid_y,
            esc(&row.name),
            display
        );

        // Bar position and width.
        let bar_x = chart_x0 as f64 + (row.start.as_nanos() as f64 / total_ns) * CHART_W as f64;
        let bar_w = (row.dur.as_nanos() as f64 / total_ns * CHART_W as f64).max(2.0);

        // Tooltip: name + status + duration + task list.
        let mut tip = format!("{}\n{} · {}", esc(&row.name), row.status, fmt_dur(row.dur));
        for t in &row.tasks {
            tip.push_str(&format!("\n  {} \u{2014} {}", fmt_dur(t.dur), esc(&t.name)));
        }
        w!(
            "<rect x=\"{bar_x:.1}\" y=\"{bar_y}\" width=\"{bar_w:.1}\" height=\"{bar_h}\" \
             fill=\"{}\" rx=\"3\" opacity=\"0.88\"><title>{}</title></rect>",
            row.color,
            tip
        );

        // Duration label inside bar when bar is wide enough.
        if bar_w > 46.0 {
            w!(
                "<text x=\"{:.1}\" y=\"{}\" fill=\"#1e1e2e\" text-anchor=\"middle\" \
                 dominant-baseline=\"middle\" font-size=\"10\" font-weight=\"bold\">{}</text>",
                bar_x + bar_w / 2.0,
                mid_y,
                fmt_dur(row.dur)
            );
        }

        // Task stripes: a thin 3 px strip at the bottom of the rule bar showing
        // each task's actual duration and start offset relative to build start.
        if row.tasks.len() > 1 {
            for task in &row.tasks {
                let tx =
                    chart_x0 as f64 + (task.start.as_nanos() as f64 / total_ns) * CHART_W as f64;
                let tw = (task.dur.as_nanos() as f64 / total_ns * CHART_W as f64).max(1.0);
                let ty = bar_y + bar_h - 3;
                w!(
                    "<rect x=\"{tx:.1}\" y=\"{ty}\" width=\"{tw:.1}\" height=\"3\" \
                     fill=\"{}\" opacity=\"0.75\"><title>{} ({})</title></rect>",
                    task.color,
                    esc(&task.name),
                    fmt_dur(task.dur)
                );
            }
        }
    }

    w!("</svg>");
    s
}

// ─── Duration-sorted HTML table ───────────────────────────────────────────────

fn build_table(rows: &[RuleRow], total: Duration) -> String {
    let total_ns = total.as_nanos().max(1) as f64;

    let mut sorted: Vec<&RuleRow> = rows.iter().collect();
    sorted.sort_by(|a, b| b.dur.cmp(&a.dur));

    let mut s = String::new();
    macro_rules! w { ($($a:tt)*) => { s.push_str(&format!($($a)*)); }; }

    w!("<table><thead><tr>\
         <th>#</th><th>Rule</th><th>Duration</th><th>Share</th>\
         <th>Status</th><th>Tasks</th><th>Slowest task</th>\
         </tr></thead><tbody>");

    for (rank, row) in sorted.iter().enumerate() {
        let pct = row.dur.as_nanos() as f64 / total_ns * 100.0;
        let slowest = row.tasks.iter().max_by_key(|t| t.dur.as_nanos());
        let slowest_str = slowest
            .map(|t| format!("{} ({})", esc(&t.name), fmt_dur(t.dur)))
            .unwrap_or_default();
        let status_class = match row.status {
            "ok" => "s-ok",
            "FAILED" => "s-failed",
            "up-to-date" => "s-uptodate",
            _ => "s-running"
        };
        w!(
            "<tr>\
             <td class='rank'>{rank}</td>\
             <td class='name' title='{title}'>{name}</td>\
             <td class='dur'>{dur}\
               <div class='bar-wrap'>\
                 <div class='bar' style='width:{pct:.1}%;background:{color}'></div>\
               </div>\
             </td>\
             <td class='pct'>{pct:.1}%</td>\
             <td class='{scls}'><b>{status}</b></td>\
             <td class='tc'>{tc}</td>\
             <td class='slow'>{slow}</td>\
             </tr>",
            rank = rank + 1,
            title = esc(&row.name),
            name = esc(&truncate_left(&row.name, 60)),
            dur = fmt_dur(row.dur),
            pct = pct.min(100.0),
            color = row.color,
            scls = status_class,
            status = row.status,
            tc = row.tasks.len(),
            slow = slowest_str,
        );
    }

    w!("</tbody></table>");
    s
}
