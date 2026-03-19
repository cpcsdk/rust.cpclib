use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::model::{RuleEntry, RuleStatus, TaskEntry, TaskStatus};

// ─── Duration formatting ──────────────────────────────────────────────────────

/// Format a `Duration` in a compact, human-readable way:
/// -    < 60 s  → "12.3s"
/// -  ≥ 60 s    → "1m23s"
/// - ≥ 3600 s   → "1h23m"
pub(crate) fn fmt_duration(d: std::time::Duration) -> String {
    let total_secs = d.as_secs();
    if total_secs < 60 {
        format!("{:.1}s", d.as_secs_f64())
    } else if total_secs < 3600 {
        let m = total_secs / 60;
        let s = total_secs % 60;
        format!("{m}m{s:02}s")
    } else {
        let h = total_secs / 3600;
        let m = (total_secs % 3600) / 60;
        format!("{h}h{m:02}m")
    }
}


/// Strip ANSI/VT escape sequences from a string so raw terminal output does
/// not corrupt ratatui's cell buffer.
///
/// Handled families:
/// - CSI  `ESC [ … final-byte(0x40–0x7e)` — colour codes, cursor motion, …
/// - OSC  `ESC ] … BEL` or `ESC ] … ESC \`  — window-title strings, etc.
/// - SS3  `ESC O <one-char>` — function-key encoding
/// - Other two-char `ESC X` sequences — the ESC and the following char are dropped.
pub(crate) fn strip_ansi_codes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                // CSI: consume until final byte in 0x40–0x7e
                for inner in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&inner) {
                        break;
                    }
                }
            },
            Some(']') => {
                // OSC: consume until BEL or ST (ESC \)
                loop {
                    match chars.next() {
                        None | Some('\x07') => break,          // BEL terminates
                        Some('\x1b') => { chars.next(); break }, // ESC \ terminates
                        _ => {},
                    }
                }
            },
            Some('O') => {
                // SS3: single character follows
                chars.next();
            },
            Some(_) | None => {
                // Any other two-char ESC sequence: both chars dropped
            },
        }
    }
    out
}

// ─── Marquee helper ───────────────────────────────────────────────────────────

/// Return an `avail`-wide window of `names` scrolling left over time.
/// If `names` fits it is returned as-is, right-padded with spaces.
pub(crate) fn marquee_window(names: &str, elapsed_ms: u64, avail: usize) -> String {
    if avail == 0 {
        return String::new();
    }
    let chars: Vec<char> = names.chars().collect();
    if chars.len() <= avail {
        let mut s = names.to_owned();
        s.extend(std::iter::repeat(' ').take(avail - chars.len()));
        return s;
    }
    // Pad the cycle with a visible separator so wrap-around is clear.
    let sep: Vec<char> = "  ·  ".chars().collect();
    let padded: Vec<char> = chars.iter().chain(sep.iter()).copied().collect();
    let cycle = padded.len();
    let offset = (elapsed_ms as usize / 80) % cycle;
    (0..avail).map(|i| padded[(offset + i) % cycle]).collect()
}

// ─── Rules list widget ────────────────────────────────────────────────────────

pub(crate) struct RulesView<'a> {
    pub(crate) rules:             &'a [RuleEntry],
    pub(crate) orphans:           &'a [TaskEntry],
    pub(crate) skip:              usize,
    pub(crate) selected_rule:     Option<usize>,
    /// When true, UpToDate rules are hidden and replaced by a single summary
    /// line at the bottom of the list.
    pub(crate) collapse_uptodate: bool,
}

impl<'a> Widget for RulesView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut y = area.y;
        let bottom = area.y + area.height;
        let mut remaining_skip = self.skip;
        let mut collapsed_uptodate: usize = 0;

        for (idx, rule) in self.rules.iter().enumerate() {
            // Collapsed UpToDate rules are removed from the visible layout —
            // they don't consume skip budget or screen rows.
            if self.collapse_uptodate && matches!(rule.status, RuleStatus::UpToDate) {
                collapsed_uptodate += 1;
                continue;
            }
            if remaining_skip > 0 {
                remaining_skip -= 1;
                continue;
            }
            if y >= bottom {
                break;
            }
            let is_selected = self.selected_rule == Some(idx);
            // A selected rule expands to fill all remaining space so the
            // user can read its full output. Nothing renders below it.
            let h = if is_selected {
                bottom - y
            } else {
                rule.height().min(bottom - y)
            };
            RuleWidget { rule, selected: is_selected }.render(
                Rect { x: area.x, y, width: area.width, height: h },
                buf,
            );
            y += h;
        }
        for task in self.orphans {
            if remaining_skip > 0 {
                remaining_skip -= 1;
                continue;
            }
            if y >= bottom {
                break;
            }
            let h = task.inline_height().min(bottom - y);
            InlineTaskWidget::new(task).render(
                Rect { x: area.x, y, width: area.width, height: h },
                buf,
            );
            y += h;
        }
        // Collapsed UpToDate summary line rendered at the bottom of the list.
        if collapsed_uptodate > 0 && y < bottom {
            let s = if collapsed_uptodate == 1 {
                "\u{2261}  [1 rule up-to-date  \u{b7}  u to expand]".to_owned()
            } else {
                format!("\u{2261}  [{collapsed_uptodate} rules up-to-date  \u{b7}  u to expand]")
            };
            Paragraph::new(Line::from(Span::styled(s, Style::default().fg(Color::DarkGray))))
                .render(Rect { x: area.x, y, width: area.width, height: 1 }, buf);
        }
    }
}

// ─── Inline task widget ───────────────────────────────────────────────────────

pub(crate) struct InlineTaskWidget<'a> {
    pub(crate) task:     &'a TaskEntry,
    /// Horizontal character offset applied to all output lines.
    pub(crate) h_scroll: usize,
}

impl<'a> InlineTaskWidget<'a> {
    pub(crate) fn new(task: &'a TaskEntry) -> Self {
        Self { task, h_scroll: 0 }
    }

    pub(crate) fn with_h_scroll(task: &'a TaskEntry, h_scroll: usize) -> Self {
        Self { task, h_scroll }
    }
}

impl<'a> Widget for InlineTaskWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }
        let entry = self.task;
        let h_scroll = self.h_scroll;
        let (prefix, style) = match &entry.status {
            TaskStatus::Running => ("● ", Style::default().fg(Color::Yellow)),
            TaskStatus::Success(_) => ("✓ ", Style::default().fg(Color::Green)),
            TaskStatus::Failed(_) => (
                "✗ ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        };
        let elapsed_ms = entry.started.elapsed().as_millis() as u64;
        let header = match &entry.status {
            TaskStatus::Running => {
                const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                let frame = FRAMES[(elapsed_ms as usize / 100) % FRAMES.len()];
                let est_hint = entry.estimated_duration.and_then(|est| {
                    let spent = entry.started.elapsed();
                    if spent >= est {
                        None
                    } else {
                        Some(format!("  ~{}", fmt_duration(est - spent)))
                    }
                }).unwrap_or_default();
                // Marquee-scroll the task name when it doesn't fit and the user
                // hasn't manually scrolled (h_scroll == 0 means "follow mode").
                let prefix_cols = 2usize; // spinner + space
                let hint_cols = est_hint.chars().count();
                let name_avail = (area.width as usize).saturating_sub(prefix_cols + hint_cols);
                let name_str = if h_scroll == 0 {
                    marquee_window(&entry.task, elapsed_ms, name_avail)
                } else {
                    entry.task.clone()
                };
                format!("{frame} {name_str}{est_hint}")
            },
            TaskStatus::Success(d) | TaskStatus::Failed(d) => {
                format!("{prefix} {}  {}", fmt_duration(*d), entry.task)
            },
        };
        // Apply h_scroll to header line too.
        let header_chars: Vec<char> = header.chars().collect();
        let header_display: String =
            header_chars[h_scroll.min(header_chars.len())..].iter().collect();
        Paragraph::new(header_display)
            .style(style)
            .render(Rect { height: 1, ..area }, buf);

        let show_output = area.height > 1
            && (matches!(entry.status, TaskStatus::Running)
                || (matches!(entry.status, TaskStatus::Failed(_)) && !entry.stderr.is_empty())
                || (matches!(entry.status, TaskStatus::Success(_)) && !entry.stdout.is_empty()));
        if show_output {
            let out_area = Rect { y: area.y + 1, height: area.height - 1, ..area };
            let all_lines: Vec<(&str, Style)> = match &entry.status {
                TaskStatus::Running => {
                    let stderr_iter = entry
                        .stderr
                        .iter()
                        .map(|s| (s.as_str(), Style::default().fg(Color::Red)));
                    stderr_iter
                        .chain(entry.stdout.iter().map(|s| (s.as_str(), Style::default())))
                        .collect()
                },
                TaskStatus::Failed(_) => {
                    // Failed: stderr first (red), then stdout — PTY-spawned processes
                    // emit all output (including errors) through stdout.
                    let stderr_iter = entry
                        .stderr
                        .iter()
                        .map(|s| (s.as_str(), Style::default().fg(Color::Red)));
                    stderr_iter
                        .chain(entry.stdout.iter().map(|s| (s.as_str(), Style::default())))
                        .collect()
                },
                TaskStatus::Success(_) => {
                    // Success with stdout: show the output (e.g. emulator logs).
                    entry
                        .stdout
                        .iter()
                        .map(|s| (s.as_str(), Style::default().fg(Color::DarkGray)))
                        .collect()
                },
            };
            let start = all_lines.len().saturating_sub(out_area.height as usize);
            let w = out_area.width as usize;
            for (i, (text, sty)) in all_lines[start..].iter().enumerate() {
                if i as u16 >= out_area.height {
                    break;
                }
                let line = format!("  {text}");
                let chars: Vec<char> = line.chars().collect();
                let from = h_scroll.min(chars.len());
                let visible: String = chars[from..].iter().take(w).collect();
                Paragraph::new(visible)
                    .style(*sty)
                    .render(Rect { y: out_area.y + i as u16, height: 1, ..out_area }, buf);
            }
        }
    }
}

// ─── Rule widget ──────────────────────────────────────────────────────────────

pub(crate) struct RuleWidget<'a> {
    pub(crate) rule:     &'a RuleEntry,
    pub(crate) selected: bool,
}

impl<'a> Widget for RuleWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }
        let rule = self.rule;
        let selected = self.selected;
        // Build a full display name that includes any co-target aliases.
        let full_name: std::borrow::Cow<str> = if rule.aliases.is_empty() {
            std::borrow::Cow::Borrowed(rule.name.as_str())
        } else {
            let parts = std::iter::once(rule.name.as_str())
                .chain(rule.aliases.iter().map(|s| s.as_str()))
                .collect::<Vec<_>>()
                .join(" ");
            std::borrow::Cow::Owned(parts)
        };
        let elapsed_ms = rule.started.elapsed().as_millis() as u64;

        // Trim trailing '.' from display names (targets often end with '.' as a convention).
        let full_name_display = full_name.trim_end_matches('.');

        match &rule.status {
            RuleStatus::Running => render_running_rule(rule, selected, full_name_display, area, buf),
            RuleStatus::UpToDate => {
                render_uptodate_rule(rule, full_name_display, elapsed_ms, area, buf)
            },
            RuleStatus::Success(dur) | RuleStatus::Failed(dur) => {
                let is_success = matches!(&rule.status, RuleStatus::Success(_));
                render_finished_rule(rule, selected, is_success, dur, full_name_display, elapsed_ms, area, buf);
            },
        }
    }
}

fn render_running_rule(
    rule: &RuleEntry,
    selected: bool,
    full_name: &str,
    area: Rect,
    buf: &mut Buffer,
) {
    let elapsed_ms = rule.started.elapsed().as_millis() as u64;
    let elapsed_str = fmt_duration(rule.started.elapsed());
    let counter = if rule.out_of > 0 {
        format!("  [{}/{}]  {elapsed_str}", rule.nb, rule.out_of)
    } else {
        format!("  {elapsed_str}")
    };
    // ETA hint: show remaining time only; hide when over-time to avoid confusion.
    let est_hint: String = rule.estimated_duration.and_then(|est| {
        let spent = rule.started.elapsed();
        if spent >= est {
            None
        } else {
            Some(format!("  ~{}", fmt_duration(est - spent)))
        }
    }).unwrap_or_default();
    let counter = format!("{counter}{est_hint}");

    // Title bar has area.width columns; border chars take 1 each side.
    let title_bar_w = area.width.saturating_sub(2) as usize;
    let prefix_w = 2usize; // "⟳ "
    let counter_w = counter.chars().count();
    let names_w = full_name.chars().count();

    let title = if prefix_w + names_w + counter_w <= title_bar_w || area.width < 8 {
        // Fits without scrolling.
        if counter.is_empty() {
            Line::from(vec![
                Span::styled("⟳ ", Style::default().fg(Color::Yellow)),
                Span::raw(full_name.to_owned()),
            ])
        } else {
            Line::from(vec![
                Span::styled("⟳ ", Style::default().fg(Color::Yellow)),
                Span::raw(full_name.to_owned()),
                Span::styled(counter.clone(), Style::default().fg(Color::DarkGray)),
            ])
        }
    } else {
        // Too long: marquee-scroll the names from right to left.
        let names_avail = (title_bar_w * 2 / 3)
            .min(title_bar_w.saturating_sub(prefix_w + counter_w));
        let scrolled = marquee_window(full_name, elapsed_ms, names_avail);
        Line::from(vec![
            Span::styled("⟳ ", Style::default().fg(Color::Yellow)),
            Span::raw(scrolled),
            Span::styled(counter.clone(), Style::default().fg(Color::DarkGray)),
        ])
    };
    // Flash the border white for 400 ms when new output arrives, drawing the
    // user's eye to the active rule without requiring them to select it.
    let flash = rule
        .last_output
        .map(|t| t.elapsed() < std::time::Duration::from_millis(400))
        .unwrap_or(false);
    let border_color = if selected {
        Color::Cyan
    } else if flash {
        Color::White
    } else {
        Color::Yellow
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);
    let inner = block.inner(area);
    block.render(area, buf);

    // Auto-scroll: show the most-recent (last) tasks when they don't all fit.
    // When selected, use full (uncapped) per-task height so all output lines are
    // reachable via task_scroll.
    let height_fn =
        |t: &TaskEntry| -> u16 { if selected { t.full_height() } else { t.inline_height() } };
    let auto_start = {
        let mut rem = inner.height;
        let mut fit = 0usize;
        for t in rule.tasks.iter().rev() {
            let h = height_fn(t);
            if fit == 0 {
                fit += 1;
                rem = rem.saturating_sub(h);
            } else if h <= rem {
                fit += 1;
                rem -= h;
            } else {
                break;
            }
        }
        rule.tasks.len().saturating_sub(fit)
    };
    // When selected, allow scrolling back from the auto-follow position.
    let task_start = if selected {
        auto_start.saturating_sub(rule.task_scroll)
    } else {
        auto_start
    };

    let mut y = inner.y;
    for task in &rule.tasks[task_start..] {
        if y >= inner.y + inner.height {
            break;
        }
        let avail = inner.y + inner.height - y;
        let h = height_fn(task).min(avail);
        let widget = if selected {
            InlineTaskWidget::with_h_scroll(task, rule.h_scroll)
        } else {
            InlineTaskWidget::new(task)
        };
        widget.render(Rect { y, height: h, ..inner }, buf);
        y += h;
    }
}

/// Compact 1-line rendering for rules whose targets were already up to date.
fn render_uptodate_rule(rule: &RuleEntry, full_name: &str, elapsed_ms: u64, area: Rect, buf: &mut Buffer) {
    let prefix = "≡  ";
    let prefix_w = prefix.chars().count();
    let source_suffix: String = rule
        .source
        .as_deref()
        .map(|s| format!("  [{}]", s))
        .unwrap_or_default();
    let suffix_w = source_suffix.chars().count();
    let names_avail = (area.width as usize).saturating_sub(prefix_w + suffix_w);
    let name_text = marquee_window(full_name, elapsed_ms, names_avail);
    let pad_w = names_avail.saturating_sub(name_text.chars().count());
    let mut spans = vec![
        Span::styled(prefix, Style::default().fg(Color::Cyan)),
        Span::styled(name_text, Style::default().fg(Color::DarkGray)),
    ];
    if !source_suffix.is_empty() {
        spans.push(Span::raw(" ".repeat(pad_w)));
        spans.push(Span::styled(source_suffix, Style::default().fg(Color::DarkGray)));
    }
    Paragraph::new(Line::from(spans)).render(area, buf);
}

fn render_finished_rule(
    rule: &RuleEntry,
    selected: bool,
    is_success: bool,
    dur: &std::time::Duration,
    full_name: &str,
    elapsed_ms: u64,
    area: Rect,
    buf: &mut Buffer,
) {
    if selected {
        // ── Expanded detail view ─────────────────────────────────────────────
        let (icon, title_style) = if is_success {
            ("✓", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            ("✗", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        };
        let title_str = format!("{icon} {}  {full_name}  ", fmt_duration(*dur));
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Line::from(vec![
                Span::styled(title_str, title_style),
                Span::styled(
                    "esc/tab · ↑↓ scroll · ←→ h-scroll",
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        let inner = block.inner(area);
        block.render(area, buf);

        // Collect all task output lines: (text, Style)
        let mut all_lines: Vec<(String, Style)> = Vec::new();
        for task in &rule.tasks {
            let (t_icon, t_style) = match &task.status {
                TaskStatus::Running => (
                    "⠿",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                TaskStatus::Success(_) => (
                    "✓",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
                TaskStatus::Failed(_) => {
                    ("✗", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                },
            };
            let d_str = match &task.status {
                TaskStatus::Running => "  ??.??s".to_owned(),
                TaskStatus::Success(d) | TaskStatus::Failed(d) => {
                    format!("  {}", fmt_duration(*d))
                },
            };
            all_lines.push((format!("{t_icon}{d_str}  {}", task.task), t_style));
            for s in &task.stderr {
                all_lines.push((format!("  {s}"), Style::default().fg(Color::Red)));
            }
            for s in &task.stdout {
                all_lines.push((format!("  {s}"), Style::default()));
            }
        }

        // Vertical: task_scroll=0 → bottom (newest); larger → older lines
        let total = all_lines.len();
        let visible_h = inner.height as usize;
        let auto_start = total.saturating_sub(visible_h);
        let v_start = auto_start.saturating_sub(rule.task_scroll);
        let end = (v_start + visible_h).min(total);

        let h_off = rule.h_scroll;
        let w = inner.width as usize;
        for (row, (text, style)) in all_lines[v_start..end].iter().enumerate() {
            if row >= visible_h {
                break;
            }
            let chars: Vec<char> = text.chars().collect();
            let from = h_off.min(chars.len());
            let visible_str: String = chars[from..].iter().take(w).collect();
            Paragraph::new(visible_str)
                .style(*style)
                .render(Rect { y: inner.y + row as u16, height: 1, ..inner }, buf);
        }
    } else if is_success {
        // ── Success: compact 1-line view ──────────────────────────────────
        let prefix = format!("✓  {}  ", fmt_duration(*dur));
        let prefix_w = prefix.chars().count();
        let source_suffix: String = rule
            .source
            .as_deref()
            .map(|s| format!("  [{}]", s))
            .unwrap_or_default();
        let suffix_w = source_suffix.chars().count();
        let names_avail = (area.width as usize).saturating_sub(prefix_w + suffix_w);
        let name_text = marquee_window(full_name, elapsed_ms, names_avail);
        let pad_w = names_avail.saturating_sub(name_text.chars().count());
        let mut spans = vec![
            Span::styled(prefix, Style::default().fg(Color::Green)),
            Span::styled(name_text, Style::default().fg(Color::Green)),
        ];
        if !source_suffix.is_empty() {
            spans.push(Span::raw(" ".repeat(pad_w)));
            spans.push(Span::styled(source_suffix, Style::default().fg(Color::DarkGray)));
        }
        Paragraph::new(Line::from(spans)).render(area, buf);
    } else if area.height <= 1 {
        // ── Failed: 1-line fallback ──────────────────────────────────────────
        let prefix = format!("✗  {}  ", fmt_duration(*dur));
        let suffix = "  [FAILED]";
        let prefix_w = prefix.chars().count();
        let suffix_w = suffix.chars().count();
        let names_avail = (area.width as usize * 2 / 3)
            .min((area.width as usize).saturating_sub(prefix_w + suffix_w));
        let name_text = marquee_window(full_name, elapsed_ms, names_avail);
        Paragraph::new(Line::from(vec![
            Span::raw(prefix),
            Span::raw(name_text),
            Span::raw(suffix),
        ]))
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .render(area, buf);
    } else {
        // ── Failed: box view with tasks listed ───────────────────────────────
        let title_str =
            format!("✗ {}  {full_name}  [FAILED]", fmt_duration(*dur));
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .title(Line::from(Span::styled(
                title_str,
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
        let inner = block.inner(area);
        block.render(area, buf);
        let mut y_off = 0u16;
        for task in &rule.tasks {
            if y_off >= inner.height {
                break;
            }
            let avail = inner.height - y_off;
            let h = task.inline_height().min(avail);
            InlineTaskWidget::new(task).render(
                Rect { y: inner.y + y_off, height: h, ..inner },
                buf,
            );
            y_off += h;
        }
    }
}
