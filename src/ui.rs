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
use unicode_width::UnicodeWidthChar;

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

/// The logical diff-row index a click at `(col, row)` lands on, or `None` if outside the
/// diff pane. `heights` (display rows per logical row) and `diff_scroll` reproduce the
/// painted window, so a click on any display line of a wrapped row maps to that row.
#[must_use]
pub fn hit_diff(
    area: Rect,
    col: u16,
    row: u16,
    heights: &[usize],
    diff_scroll: usize,
) -> Option<usize> {
    let rows = vrows(area);
    let (diff_area, _) = body_split(&rows);
    let inner = inner_rect(diff_area);
    if !contains(inner, col, row) {
        return None;
    }
    let target = (row - inner.y) as usize;
    let mut acc = 0;
    for (li, h) in heights.iter().enumerate().skip(diff_scroll) {
        acc += h;
        if target < acc {
            return Some(li);
        }
    }
    None
}

/// The number of diff rows visible in the diff pane, used to clamp the scroll.
#[must_use]
pub fn diff_viewport_height(area: Rect) -> usize {
    let rows = vrows(area);
    let (diff_area, _) = body_split(&rows);
    inner_rect(diff_area).height as usize
}

/// The display height (rows on screen) of each visible logical diff row, honoring wrap.
#[must_use]
pub fn diff_row_heights(app: &App, area: Rect) -> Vec<usize> {
    let rows = vrows(area);
    let (diff_area, _) = body_split(&rows);
    let width = inner_rect(diff_area).width as usize;
    let total_lines: usize =
        app.diff.rows.iter().map(|r| if r.is_content() { 1 } else { r.hidden() }).sum();
    let gutter_w = gutter_width(total_lines);
    app.visible.iter().map(|r| row_height(r, gutter_w, width, app.wrap)).collect()
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
    let layout = RowLayout { gutter_w, width, h_scroll: app.h_scroll, wrap: app.wrap };
    let commented = app.commented_lines();
    let (lo, hi) = app.selection_range();
    let selecting = app.focus == Focus::Diff && app.select_anchor.is_some();

    // One logical row → 1+ display lines (wrapping); the cursor/selection apply to all
    // its display lines.
    let row_lines = |i: usize| -> Vec<Line> {
        let state = RowState {
            commented: commented.contains(&i),
            cursor: app.focus == Focus::Diff && i == app.diff_cursor,
            selected: selecting && i >= lo && i <= hi,
        };
        render_row(&app.visible[i], layout, state)
    };
    // Display lines for the logical rows in `range`, in order.
    let display = |range: std::ops::Range<usize>| -> Vec<Line> {
        range.flat_map(&row_lines).collect::<Vec<_>>()
    };

    let rows = app.visible.len();
    if !app.composing() {
        // Fill the pane from `diff_scroll`'s first display line; clamp keeps the cursor in.
        let mut out = display(app.diff_scroll..rows);
        out.truncate(height);
        frame.render_widget(Paragraph::new(out), inner);
        return;
    }

    // Composing: splice the input box under the last selected line, in display rows.
    // Cap the box at height-1 so a comment taller than the viewport can't hide its anchor.
    let box_h = composer_height(app).min(height.saturating_sub(1)).max(1);
    let diff_budget = height - box_h;
    let anchor = hi.clamp(app.diff_scroll, rows.saturating_sub(1));
    let above = display(app.diff_scroll..anchor + 1);
    // Keep the anchor's last display line just above the box when `above` overflows.
    let above: Vec<Line> =
        if above.len() > diff_budget { above[above.len() - diff_budget..].to_vec() } else { above };
    let remaining = diff_budget - above.len();
    let mut below = display(anchor + 1..rows);
    below.truncate(remaining);

    let slots = Layout::vertical([
        Constraint::Length(above.len() as u16),
        Constraint::Length(box_h as u16),
        Constraint::Length(below.len() as u16),
    ])
    .split(inner);
    if !above.is_empty() {
        frame.render_widget(Paragraph::new(above), slots[0]);
    }
    render_composer(frame, app, slots[1]);
    if !below.is_empty() {
        frame.render_widget(Paragraph::new(below), slots[2]);
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
// Word-emphasis fills — a brighter shade of the row tint over the changed words.
const EMPH_DEL_BG: Color = Color::Rgb(0x6e, 0x34, 0x46);
const EMPH_INS_BG: Color = Color::Rgb(0x30, 0x55, 0x3f);
const CURSOR_BG: Color = cat::SURFACE2;
const SEL_BG: Color = cat::SURFACE0;
const FOLD_BG: Color = cat::SURFACE1;

/// The line-number column width for a diff of `rows` lines.
fn gutter_width(rows: usize) -> usize {
    rows.to_string().len().max(3)
}

/// The gutter prefix width: the change bar plus the right-aligned line number and a space.
fn gutter_prefix_width(gutter_w: usize) -> usize {
    1 + gutter_w + 1
}

/// How many display rows a row needs: 1 for a fold or with wrap off, else the number of
/// word-wrapped segments its (tab-expanded) content fills. Shares [`wrap_segments`] with
/// the renderer so per-row geometry stays aligned with what gets painted.
fn row_height(row: &Row, gutter_w: usize, width: usize, wrap: bool) -> usize {
    if !wrap || matches!(row, Row::Fold { .. }) {
        return 1;
    }
    let code_width = width.saturating_sub(gutter_prefix_width(gutter_w)).max(1);
    wrap_segments(&code_cells(row, false), code_width).len()
}

/// The diff-pane layout: constant for a frame.
#[derive(Clone, Copy)]
struct RowLayout {
    gutter_w: usize,
    width: usize,
    h_scroll: usize,
    wrap: bool,
}

/// A row's per-row highlight state.
#[derive(Clone, Copy)]
struct RowState {
    commented: bool,
    cursor: bool,
    selected: bool,
}

/// A diff row as one or more full-width display lines: a left change bar, the line
/// number, then syntax-colored code tinted red/green. With wrap on, a long line breaks
/// into `code_width`-wide rows; a continuation row carries a blank gutter so numbers
/// stay aligned. With wrap off, the line is one row scrolled by `h_scroll`.
fn render_row(row: &Row, layout: RowLayout, state: RowState) -> Vec<Line<'static>> {
    let RowLayout { gutter_w, width, h_scroll, wrap } = layout;
    let RowState { commented, cursor, selected } = state;
    if let Row::Fold { .. } = row {
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
        return vec![line.style(Style::default().bg(bg).add_modifier(Modifier::BOLD))];
    }
    let num = row.new_no().or_else(|| row.old_no()).map_or(String::new(), |n| n.to_string());
    let num_color = if commented { cat::YELLOW } else { cat::OVERLAY0 };
    let (bar, bar_color) = match row.marker() {
        '-' => ("▌", cat::RED),
        '+' => ("▌", cat::GREEN),
        _ => (" ", cat::OVERLAY0),
    };
    let row_bg = if cursor {
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

    // Word emphasis brightens the changed words, unless the row's fill is a cursor or
    // selection bg, which wins for readability.
    let emph_on = !cursor && !selected;
    let emph_bg = match row.marker() {
        '-' => EMPH_DEL_BG,
        '+' => EMPH_INS_BG,
        _ => INS_BG,
    };
    let cells = code_cells(row, emph_on);

    let prefix_w = gutter_prefix_width(gutter_w);
    let code_width = width.saturating_sub(prefix_w).max(1);
    // Without wrap the line is one chunk scrolled by `h_scroll`; with wrap, word-wrapped
    // segments, the first numbered and the rest blank-gutter.
    let chunks: Vec<&[Cell]> = if wrap {
        wrap_segments(&cells, code_width).into_iter().map(|(s, e)| &cells[s..e]).collect()
    } else {
        vec![cells.get(skip_columns(&cells, h_scroll)..).unwrap_or(&[])]
    };

    chunks
        .into_iter()
        .enumerate()
        .map(|(k, chunk)| {
            let gutter = if k == 0 {
                vec![
                    Span::styled(bar, Style::default().fg(bar_color)),
                    Span::styled(format!("{num:>gutter_w$} "), Style::default().fg(num_color)),
                ]
            } else {
                // A continuation row keeps the change bar but blanks the number column.
                vec![
                    Span::styled(bar, Style::default().fg(bar_color)),
                    Span::raw(" ".repeat(prefix_w - 1)),
                ]
            };
            let mut spans = gutter;
            spans.extend(cells_to_spans(chunk, emph_bg));
            let mut line = Line::from(spans);
            if let Some(pad) = width.checked_sub(line.width()).filter(|p| *p > 0) {
                line.push_span(Span::raw(" ".repeat(pad)));
            }
            match row_bg {
                Some(bg) => line.style(Style::default().bg(bg)),
                None => line,
            }
        })
        .collect()
}

fn rgb(c: crate::diff::Rgb) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

/// Tabs expand to this many columns.
const TAB: usize = 4;

/// Greedy word wrap over display cells into half-open ranges, one per display row.
///
/// Breaks at the last space that fits within `width`, falling back to a hard break when a
/// single word is wider than the column. Leading spaces on a continuation are dropped so a
/// break landing just before a space doesn't leave an almost-empty row. An empty line still
/// yields one (empty) range so it occupies a row. The renderer and [`row_height`] share this
/// so what's measured matches what's painted.
fn wrap_segments(cells: &[Cell], width: usize) -> Vec<(usize, usize)> {
    if cells.is_empty() {
        return vec![(0, 0)];
    }
    let mut segs = Vec::new();
    let mut start = 0;
    while start < cells.len() {
        // Take as many cells as fit within `width` columns, always at least one (so a glyph
        // wider than the column still gets its own row rather than stalling).
        let mut col = 0;
        let mut limit = start;
        while limit < cells.len() {
            let cw = cells[limit].w;
            if col + cw > width && limit > start {
                break;
            }
            col += cw;
            limit += 1;
        }
        if limit == cells.len() {
            segs.push((start, cells.len()));
            break;
        }
        // More cells follow; prefer breaking just after the last space that fits.
        let brk = (start..limit).rev().find(|&i| cells[i].ch == ' ').map(|i| i + 1);
        let end = brk.filter(|&e| e > start).unwrap_or(limit);
        segs.push((start, end));
        start = end;
        while start < cells.len() && cells[start].ch == ' ' {
            start += 1;
        }
    }
    segs
}

/// The first cell index lying at or past `cols` display columns — the no-wrap horizontal
/// scroll offset, snapping past a wide glyph that straddles the boundary rather than
/// splitting it.
fn skip_columns(cells: &[Cell], cols: usize) -> usize {
    let mut col = 0;
    let mut i = 0;
    while i < cells.len() && col < cols {
        col += cells[i].w;
        i += 1;
    }
    i
}

/// One display cell of a code line: a glyph, its terminal width in columns (1 for most
/// text, 2 for wide CJK/emoji, 0 for a combining mark), its syntax color, and whether it
/// falls in a word-emphasis range.
struct Cell {
    ch: char,
    w: usize,
    fg: Color,
    emph: bool,
}

/// Expand a row's spans into display cells: tabs become spaces to the next tab stop, and
/// each char carries its column width, color, and (when `emph_on`) its word-emphasis flag.
/// Width comes from `unicode-width` so wide glyphs measure as the two columns they paint.
fn code_cells(row: &Row, emph_on: bool) -> Vec<Cell> {
    let emphasis = if emph_on { row.emphasis() } else { &[] };
    let in_emph = |i: u32| emphasis.iter().any(|&(a, b)| i >= a && i < b);
    let mut cells = Vec::new();
    let mut idx = 0u32;
    let mut col = 0usize; // display column, so tab stops land right after wide glyphs too
    for s in row.spans() {
        let fg = rgb(s.color);
        for ch in s.text.chars() {
            let emph = in_emph(idx);
            if ch == '\t' {
                for _ in 0..(TAB - col % TAB) {
                    cells.push(Cell { ch: ' ', w: 1, fg, emph });
                    col += 1;
                }
            } else {
                let w = UnicodeWidthChar::width(ch).unwrap_or(0);
                cells.push(Cell { ch, w, fg, emph });
                col += w;
            }
            idx += 1;
        }
    }
    cells
}

/// Build spans from display cells, merging runs of equal color/emphasis; an emphasized
/// run takes `emph_bg` as its background.
fn cells_to_spans(cells: &[Cell], emph_bg: Color) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut buf = String::new();
    let mut cur: Option<(Color, bool)> = None;
    for c in cells {
        let key = (c.fg, c.emph);
        if cur != Some(key) {
            if let Some((fg, emph)) = cur {
                spans.push(cell_span(std::mem::take(&mut buf), fg, emph, emph_bg));
            }
            cur = Some(key);
        }
        buf.push(c.ch);
    }
    if let Some((fg, emph)) = cur {
        spans.push(cell_span(buf, fg, emph, emph_bg));
    }
    spans
}

fn cell_span(text: String, fg: Color, emph: bool, emph_bg: Color) -> Span<'static> {
    let style = Style::default().fg(fg);
    Span::styled(text, if emph { style.bg(emph_bg) } else { style })
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
