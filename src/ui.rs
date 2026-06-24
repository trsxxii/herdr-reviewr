//! Rendering the Changes view: tab bar, file list, diff, comment box, list, status.
//!
//! See `specs/tui.md`. The layout is a header tab bar, a body split into the diff
//! (left) and the file list (right), and a status bar. While composing, the comment
//! box is spliced inline into the diff under the selected line; the comments-list
//! overlay is drawn on top when open. Rendering reads `App` only; all state changes
//! live in `app.rs`.

use std::rc::Rc;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::{App, Focus, Mode};
use crate::diff::{FileState, Row};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let rows = vrows(area);
    let (diff_area, files_area) = body_split(&rows);

    render_tab_bar(frame, app, rows[0]);
    render_diff_view(frame, app, diff_area);
    render_file_list(frame, app, files_area);
    render_status_bar(frame, app, rows[2]);

    if app.mode == Mode::List {
        render_comments_list(frame, app, area);
    }
}

/// The vertical bands: tab bar, body, status. The comment input is inline in the
/// diff, not a band of its own. The status band grows so its hints wrap rather than
/// truncate at narrow widths.
fn vrows(area: Rect) -> Rc<[Rect]> {
    let footer = footer_height(area.width);
    Layout::vertical([Constraint::Length(1), Constraint::Min(3), Constraint::Length(footer)])
        .split(area)
}

/// Rows the status band needs for the longest hint line to wrap at `width` (1–3).
fn footer_height(width: u16) -> u16 {
    // Widest footer (counts + status + the Normal-mode hints) is ~150 columns.
    const FOOTER_COLS: u16 = 150;
    FOOTER_COLS.div_ceil(width.max(1)).clamp(1, 3)
}

/// The body split into `(diff, files)` outer rects.
fn body_split(rows: &[Rect]) -> (Rect, Rect) {
    let body =
        Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)]).split(rows[1]);
    (body[0], body[1])
}

/// The file index a click at `(col, row)` lands on, or `None` if outside the list.
#[must_use]
pub fn hit_file(area: Rect, col: u16, row: u16, n_files: usize) -> Option<usize> {
    let rows = vrows(area);
    let (_, files_area) = body_split(&rows);
    let inner = inner_rect(files_area);
    if !contains(inner, col, row) {
        return None;
    }
    let idx = (row - inner.y) as usize;
    (idx < n_files).then_some(idx)
}

/// The diff-line index a click at `(col, row)` lands on, or `None` if outside the
/// diff pane. `diff_scroll` fixes the window so the mapping matches the paint.
#[must_use]
pub fn hit_diff(
    area: Rect,
    col: u16,
    row: u16,
    diff_len: usize,
    diff_scroll: usize,
) -> Option<usize> {
    let rows = vrows(area);
    let (diff_area, _) = body_split(&rows);
    let inner = inner_rect(diff_area);
    if !contains(inner, col, row) {
        return None;
    }
    let start = diff_scroll.min(diff_len.saturating_sub(inner.height as usize));
    let idx = start + (row - inner.y) as usize;
    (idx < diff_len).then_some(idx)
}

/// The number of diff rows visible in the diff pane, used to clamp the scroll.
#[must_use]
pub fn diff_viewport_height(area: Rect) -> usize {
    let rows = vrows(area);
    let (diff_area, _) = body_split(&rows);
    inner_rect(diff_area).height as usize
}

/// Rows the inline comment box occupies: one per input line plus the border.
#[must_use]
pub fn composer_height(app: &App) -> usize {
    app.input.split('\n').count() + 2
}

/// A clickable region in the header.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeaderHit {
    Scope,
    Send,
}

/// Which header control a click at `(col, row)` lands on, if any.
#[must_use]
pub fn hit_header(area: Rect, app: &App, col: u16, row: u16) -> Option<HeaderHit> {
    if row != area.y {
        return None;
    }
    let scope_start = HEADER_PREFIX.len() as u16;
    let scope_end = scope_start + scope_chip(app).len() as u16;
    let button_start = area.width.saturating_sub(send_button(app).len() as u16);
    if (scope_start..scope_end).contains(&col) {
        Some(HeaderHit::Scope)
    } else if col >= button_start && col < area.width {
        Some(HeaderHit::Send)
    } else {
        None
    }
}

const HEADER_PREFIX: &str = " Changes  ";

fn scope_chip(app: &App) -> String {
    format!("[{}]", app.scope.label())
}

fn send_button(app: &App) -> String {
    format!("[ Send ({}) ]", app.store.len())
}

fn render_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let chip = scope_chip(app);
    let suffix = format!("  {} file(s)", app.files.len());
    let button = send_button(app);
    let used = HEADER_PREFIX.len() + chip.len() + suffix.len() + button.len();
    let pad = (area.width as usize).saturating_sub(used);

    // A quiet surface bar: the title in lavender, the clickable scope and Send controls
    // accented so they read as buttons without a loud full-width fill.
    let bar = Style::default().bg(cat::SURFACE0);
    let send_fg = if app.store.is_empty() { cat::OVERLAY0 } else { cat::GREEN };
    let line = Line::from(vec![
        Span::styled(HEADER_PREFIX, bar.fg(cat::LAVENDER).add_modifier(Modifier::BOLD)),
        Span::styled(chip, bar.fg(cat::YELLOW).add_modifier(Modifier::BOLD)),
        Span::styled(suffix, bar.fg(cat::OVERLAY0)),
        Span::styled(" ".repeat(pad), bar),
        Span::styled(button, bar.fg(send_fg).add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_file_list(frame: &mut Frame, app: &App, area: Rect) {
    let block = bordered("Files", app.focus == Focus::Files);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.files.is_empty() {
        frame.render_widget(dim_paragraph("no changes"), inner);
        return;
    }

    let width = inner.width as usize;
    let items: Vec<ListItem> = app
        .files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let marker = Span::styled(
                format!("{} ", f.kind.marker()),
                Style::default().fg(kind_color(f.kind.marker())),
            );
            let name = Span::styled(f.path.clone(), text_style());
            let stat = Span::styled(
                format!("  +{} -{}", f.additions, f.deletions),
                Style::default().fg(cat::OVERLAY0),
            );
            selectable_row(vec![marker, name, stat], width, i == app.file_cursor)
        })
        .collect();
    frame.render_widget(List::new(items), inner);
}

fn render_diff_view(frame: &mut Frame, app: &App, area: Rect) {
    let title = app.diff_path.clone().unwrap_or_else(|| "Diff".to_string());
    let block = bordered(&title, app.focus == Focus::Diff);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.visible.is_empty() {
        let msg = match app.diff.state {
            FileState::Binary => "binary — no line comments",
            FileState::TooLarge => "file too large to diff",
            FileState::Normal => "no diff",
        };
        frame.render_widget(dim_paragraph(msg), inner);
        return;
    }

    let height = inner.height as usize;
    if height == 0 {
        return;
    }
    let width = inner.width as usize;
    // Size the gutter to the file's largest line number, so it does not resize when a
    // fold toggles (folds hide lines but keep the numbering).
    let total_lines: usize =
        app.diff.rows.iter().map(|r| if r.is_content() { 1 } else { r.hidden() }).sum();
    let gutter_w = gutter_width(total_lines);
    let commented = app.commented_lines();
    let (lo, hi) = app.selection_range();
    let selecting = app.focus == Focus::Diff && app.select_anchor.is_some();

    let row_line = |i: usize| -> Line {
        let row = &app.visible[i];
        let cursor = app.focus == Focus::Diff && i == app.diff_cursor;
        let selected = selecting && i >= lo && i <= hi;
        render_row(row, gutter_w, width, commented.contains(&i), cursor, selected)
    };

    let rows = app.visible.len();
    if !app.composing() {
        let start = app.diff_scroll.min(rows.saturating_sub(height));
        let end = (start + height).min(rows);
        frame.render_widget(Paragraph::new((start..end).map(&row_line).collect::<Vec<_>>()), inner);
        return;
    }

    // Composing: splice the input box in directly under the last selected line, so the
    // diff lines below it shift down. The diff shares the pane with the box, so clamp the
    // window top against the reduced diff budget — matching the scroll reserved in the
    // event loop — rather than the full height.
    // Cap the box at height-1 so a comment taller than the viewport can't hide its anchor.
    let box_h = composer_height(app).min(height.saturating_sub(1)).max(1);
    let diff_rows = height - box_h;
    let start = app.diff_scroll.min(rows.saturating_sub(diff_rows));
    let last = rows - 1;
    let anchor = hi.clamp(start, last);
    let above_n = (anchor + 1 - start).min(diff_rows);
    let below_start = start + above_n;
    let below_n = diff_rows - above_n;
    let slots = Layout::vertical([
        Constraint::Length(above_n as u16),
        Constraint::Length(box_h as u16),
        Constraint::Length(below_n as u16),
    ])
    .split(inner);

    if above_n > 0 {
        frame.render_widget(
            Paragraph::new((start..start + above_n).map(&row_line).collect::<Vec<_>>()),
            slots[0],
        );
    }
    render_composer(frame, app, slots[1]);
    if below_n > 0 {
        let end = (below_start + below_n).min(rows);
        frame.render_widget(
            Paragraph::new((below_start..end).map(&row_line).collect::<Vec<_>>()),
            slots[2],
        );
    }
}

// --- Catppuccin Mocha palette (one source for every color the chrome and diff use,
// so the frame and the syntect-highlighted body share a single theme) ----------------

mod cat {
    use ratatui::style::Color::{self, Rgb};
    // Surfaces, dark to light.
    pub(super) const SURFACE0: Color = Rgb(0x31, 0x32, 0x44);
    pub(super) const SURFACE1: Color = Rgb(0x45, 0x47, 0x5a);
    pub(super) const SURFACE2: Color = Rgb(0x58, 0x5b, 0x70);
    pub(super) const OVERLAY0: Color = Rgb(0x6c, 0x70, 0x86);
    // Text.
    pub(super) const SUBTEXT0: Color = Rgb(0xa6, 0xad, 0xc8);
    pub(super) const TEXT: Color = Rgb(0xcd, 0xd6, 0xf4);
    // Accents.
    pub(super) const RED: Color = Rgb(0xf3, 0x8b, 0xa8);
    pub(super) const GREEN: Color = Rgb(0xa6, 0xe3, 0xa1);
    pub(super) const YELLOW: Color = Rgb(0xf9, 0xe2, 0xaf);
    pub(super) const PEACH: Color = Rgb(0xfa, 0xb3, 0x87);
    pub(super) const MAUVE: Color = Rgb(0xcb, 0xa6, 0xf7);
    pub(super) const LAVENDER: Color = Rgb(0xb4, 0xbe, 0xfe);
}

// Structural diff fills tuned for the dark base; syntax token colors come from the theme.
const DEL_BG: Color = Color::Rgb(0x3a, 0x27, 0x30);
const INS_BG: Color = Color::Rgb(0x22, 0x33, 0x2b);
const CURSOR_BG: Color = cat::SURFACE2;
const SEL_BG: Color = cat::SURFACE0;
const FOLD_BG: Color = cat::SURFACE1;

/// The line-number column width for a diff of `rows` lines.
fn gutter_width(rows: usize) -> usize {
    rows.to_string().len().max(3)
}

/// One diff row as a full-width line: a left change bar, the line number, then
/// syntax-colored code, tinted red/green for a change and padded so the row
/// background fills the pane.
fn render_row(
    row: &Row,
    gutter_w: usize,
    width: usize,
    commented: bool,
    cursor: bool,
    selected: bool,
) -> Line<'static> {
    if let Row::Fold { .. } = row {
        // Quiet by default; the focused fold reveals its action (the footer also lists `o`).
        let label = if cursor {
            format!("  ⋯  {} unmodified lines — ⏎ expand", row.hidden())
        } else {
            format!("  ⋯  {} unmodified lines", row.hidden())
        };
        let mut line = Line::from(Span::styled(label, Style::default().fg(cat::SUBTEXT0)));
        if let Some(pad) = width.checked_sub(line.width()).filter(|p| *p > 0) {
            line.push_span(Span::raw(" ".repeat(pad)));
        }
        let bg = if cursor { CURSOR_BG } else { FOLD_BG };
        return line.style(Style::default().bg(bg).add_modifier(Modifier::BOLD));
    }
    let num = row.new_no().or_else(|| row.old_no()).map_or(String::new(), |n| n.to_string());
    let num_color = if commented { cat::YELLOW } else { cat::OVERLAY0 };
    let (bar, bar_color) = match row.marker() {
        '-' => ("▌", cat::RED),
        '+' => ("▌", cat::GREEN),
        _ => (" ", cat::OVERLAY0),
    };

    // Bar first, at the far left edge, then the line number, then the code.
    let mut spans = vec![
        Span::styled(bar, Style::default().fg(bar_color)),
        Span::styled(format!("{num:>gutter_w$} "), Style::default().fg(num_color)),
    ];
    for s in row.spans() {
        spans.push(Span::styled(s.text.clone(), Style::default().fg(rgb(s.color))));
    }
    // Pad to the pane width so the row background fills the line, not just the text.
    // `Line::width` is display-width aware, so wide (CJK/emoji) glyphs pad correctly.
    let mut line = Line::from(spans);
    if let Some(pad) = width.checked_sub(line.width()).filter(|p| *p > 0) {
        line.push_span(Span::raw(" ".repeat(pad)));
    }

    let bg = if cursor {
        Some(CURSOR_BG)
    } else if selected {
        Some(SEL_BG)
    } else {
        match row.marker() {
            '-' => Some(DEL_BG),
            '+' => Some(INS_BG),
            _ => None,
        }
    };
    match bg {
        Some(bg) => line.style(Style::default().bg(bg)),
        None => line,
    }
}

fn rgb(c: crate::diff::Rgb) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

/// The inline comment input box, drawn at `area` (under the selection in the diff).
fn render_composer(frame: &mut Frame, app: &App, area: Rect) {
    let loc = app.pending_location().unwrap_or_else(|| "comment".to_string());
    let editing = matches!(app.mode, Mode::Composing { editing: Some(_) });
    let title = if editing { format!("edit · {loc}") } else { format!("comment · {loc}") };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(cat::PEACH))
        .title(title);
    let input_lines: Vec<&str> = app.input.split('\n').collect();
    let last = input_lines.len() - 1;
    let lines: Vec<Line> = input_lines
        .iter()
        .enumerate()
        .map(|(i, text)| {
            if i == last {
                Line::from(vec![
                    Span::raw((*text).to_string()),
                    Span::styled("█", Style::default().fg(cat::PEACH)),
                ])
            } else {
                Line::from((*text).to_string())
            }
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).block(block).wrap(Wrap { trim: false }), area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let left = format!(" {} file(s) · {} comment(s) ", app.files.len(), app.store.len());
    let mid = if app.status.is_empty() { String::new() } else { format!("· {} ", app.status) };
    let hints = match app.mode {
        Mode::Composing { .. } => "enter save · alt/shift+enter newline · esc cancel",
        Mode::List => "↑↓ move · s send · y copy · e edit · d delete · esc close",
        Mode::Normal => {
            "tab focus · u/b scope · v select · c comment · s send · y copy · n/N jump · l list · r refresh · q quit"
        }
    };
    let line = Line::from(vec![
        Span::styled(left, Style::default().fg(cat::TEXT).bg(cat::SURFACE0)),
        Span::styled(format!(" {mid}"), Style::default().fg(cat::PEACH)),
        Span::styled(format!("  {hints}"), Style::default().fg(cat::OVERLAY0)),
    ]);
    frame.render_widget(Paragraph::new(line).wrap(Wrap { trim: false }), area);
}

fn render_comments_list(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered(area, 80, 60);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(cat::MAUVE))
        .title(format!("Comments ({})", app.store.len()));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let width = inner.width as usize;
    let stale = app.stale_files();
    let items: Vec<ListItem> = app
        .store
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let loc = Span::styled(
                c.location(),
                Style::default().fg(cat::MAUVE).add_modifier(Modifier::BOLD),
            );
            let mut spans = vec![loc, Span::styled(format!("  {}", c.text), text_style())];
            // A comment whose file has left the changeset is flagged but kept.
            if stale.contains(&c.file) {
                spans.push(Span::styled("  (stale)", Style::default().fg(cat::RED)));
            }
            selectable_row(spans, width, i == app.list_cursor)
        })
        .collect();
    frame.render_widget(List::new(items), inner);
}

/// The default body text color.
fn text_style() -> Style {
    Style::default().fg(cat::TEXT)
}

/// A list row, highlighted with the shared selection fill (`surface2` + bold, full
/// width) when `selected` — the same treatment the diff cursor uses, so every cursor
/// in the UI reads the same. The fill is applied per span (with a trailing pad) so it
/// spans the full width under the `List` widget, matching the diff's `Paragraph` rows.
fn selectable_row(
    mut spans: Vec<Span<'static>>,
    width: usize,
    selected: bool,
) -> ListItem<'static> {
    if selected {
        let used: usize = spans.iter().map(Span::width).sum();
        if width > used {
            spans.push(Span::raw(" ".repeat(width - used)));
        }
        for s in &mut spans {
            s.style = s.style.bg(CURSOR_BG).add_modifier(Modifier::BOLD);
        }
    }
    ListItem::new(Line::from(spans))
}

// --- helpers -------------------------------------------------------------------

fn bordered(title: &str, focused: bool) -> Block<'_> {
    // A focused pane gets a lavender border; an unfocused one recedes to a surface tone.
    let color = if focused { cat::LAVENDER } else { cat::SURFACE2 };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(title.to_string())
}

fn dim_paragraph(text: &str) -> Paragraph<'_> {
    Paragraph::new(text).style(Style::default().fg(cat::OVERLAY0))
}

/// The Catppuccin accent for a change marker, matched to the diff's add/remove hues.
fn kind_color(marker: char) -> Color {
    match marker {
        'A' | '?' => cat::GREEN,
        'D' => cat::RED,
        'R' => cat::MAUVE,
        _ => cat::YELLOW,
    }
}

/// Whether `(col, row)` falls inside `rect`.
fn contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x
        && col < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}

/// The content area inside a one-cell border.
fn inner_rect(outer: Rect) -> Rect {
    Rect {
        x: outer.x.saturating_add(1),
        y: outer.y.saturating_add(1),
        width: outer.width.saturating_sub(2),
        height: outer.height.saturating_sub(2),
    }
}

/// A `Rect` centered in `area` at `pct_x` × `pct_y` percent of its size.
fn centered(area: Rect, pct_x: u16, pct_y: u16) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - pct_y) / 2),
        Constraint::Percentage(pct_y),
        Constraint::Percentage((100 - pct_y) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - pct_x) / 2),
        Constraint::Percentage(pct_x),
        Constraint::Percentage((100 - pct_x) / 2),
    ])
    .split(v[1])[1]
}
