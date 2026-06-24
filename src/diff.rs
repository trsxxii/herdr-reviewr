//! The structured diff model: a file's changes as rows built from its old and new
//! content, syntax-highlighted, ready to paint.
//!
//! See `specs/diff-view.md`. This module is terminal-free — a `Span` carries an RGB
//! color, and `src/ui.rs` maps it to a ratatui color. Milestone 1 renders the whole
//! file (no folds, no word emphasis); both arrive in milestone 2.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

use similar::{ChangeTag, TextDiff};

use crate::highlight::Highlighter;

/// An 8-bit RGB color.
pub type Rgb = (u8, u8, u8);

/// A run of one line's text in a single color.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Span {
    pub text: String,
    pub color: Rgb,
}

/// A rendered diff row. Content rows (`Context`/`Deletion`/`Insertion`) are selectable
/// for comments; a `Fold` is a collapsed run of context lines it owns.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Row {
    Context { old_no: u32, new_no: u32, spans: Vec<Span> },
    Deletion { old_no: u32, spans: Vec<Span> },
    Insertion { new_no: u32, spans: Vec<Span> },
    Fold { lines: Vec<Row> },
}

impl Row {
    pub fn old_no(&self) -> Option<u32> {
        match self {
            Row::Context { old_no, .. } | Row::Deletion { old_no, .. } => Some(*old_no),
            Row::Insertion { .. } | Row::Fold { .. } => None,
        }
    }

    pub fn new_no(&self) -> Option<u32> {
        match self {
            Row::Context { new_no, .. } | Row::Insertion { new_no, .. } => Some(*new_no),
            Row::Deletion { .. } | Row::Fold { .. } => None,
        }
    }

    pub fn spans(&self) -> &[Span] {
        match self {
            Row::Context { spans, .. }
            | Row::Deletion { spans, .. }
            | Row::Insertion { spans, .. } => spans,
            Row::Fold { .. } => &[],
        }
    }

    /// The diff marker for this row: `' '`, `'-'`, or `'+'`; `' '` for a fold.
    pub fn marker(&self) -> char {
        match self {
            Row::Deletion { .. } => '-',
            Row::Insertion { .. } => '+',
            Row::Context { .. } | Row::Fold { .. } => ' ',
        }
    }

    /// Whether this row anchors a comment — every kind but a fold.
    pub fn is_content(&self) -> bool {
        !matches!(self, Row::Fold { .. })
    }

    /// The hidden line count of a fold, else 0.
    pub fn hidden(&self) -> usize {
        match self {
            Row::Fold { lines } => lines.len(),
            _ => 0,
        }
    }

    /// A fold's stable identity across rebuilds: the line number of its first hidden
    /// line. `None` for any other row.
    pub fn fold_anchor(&self) -> Option<u32> {
        match self {
            Row::Fold { lines } => lines.first().and_then(|r| r.new_no().or_else(|| r.old_no())),
            _ => None,
        }
    }

    /// The line's plain text, joined from its spans.
    pub fn text(&self) -> String {
        self.spans().iter().map(|s| s.text.as_str()).collect()
    }

    /// The line as a marker-prefixed diff line, for the export snippet.
    pub fn marker_text(&self) -> String {
        format!("{}{}", self.marker(), self.text())
    }
}

/// Whether the file renders as rows, or a notice instead.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileState {
    Normal,
    Binary,
    TooLarge,
}

/// The selected file modeled as rows.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FileDiff {
    pub path: String,
    pub language: Option<String>,
    pub state: FileState,
    pub rows: Vec<Row>,
}

/// A file beyond either budget renders as `too_large` rather than stalling the diff —
/// the byte budget also catches a single huge line that the line budget misses.
const MAX_LINES: usize = 50_000;
const MAX_BYTES: usize = 2_000_000;

impl FileDiff {
    /// An empty placeholder, for when no file is selected.
    pub fn empty() -> Self {
        Self { path: String::new(), language: None, state: FileState::Normal, rows: Vec::new() }
    }

    /// Build the model from `old` and `new` content, highlighting with `hl`.
    pub fn build(path: String, old: &str, new: &str, hl: &Highlighter) -> Self {
        let language = language_of(&path);
        let notice = |state| Self {
            path: path.clone(),
            language: language.clone(),
            state,
            rows: Vec::new(),
        };
        if old.contains('\0') || new.contains('\0') {
            return notice(FileState::Binary);
        }
        if old.len() + new.len() > MAX_BYTES
            || old.lines().count() + new.lines().count() > MAX_LINES
        {
            return notice(FileState::TooLarge);
        }

        let lang = language.as_deref();
        let old_spans = hl.highlight(old, lang);
        let new_spans = hl.highlight(new, lang);
        let line = |spans: &[Vec<Span>], i: usize| spans.get(i).cloned().unwrap_or_default();

        let mut rows = Vec::new();
        for change in TextDiff::from_lines(old, new).iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    let (oi, ni) = (change.old_index().unwrap(), change.new_index().unwrap());
                    rows.push(Row::Context {
                        old_no: oi as u32 + 1,
                        new_no: ni as u32 + 1,
                        spans: line(&new_spans, ni),
                    });
                }
                ChangeTag::Delete => {
                    let oi = change.old_index().unwrap();
                    rows.push(Row::Deletion { old_no: oi as u32 + 1, spans: line(&old_spans, oi) });
                }
                ChangeTag::Insert => {
                    let ni = change.new_index().unwrap();
                    rows.push(Row::Insertion {
                        new_no: ni as u32 + 1,
                        spans: line(&new_spans, ni),
                    });
                }
            }
        }
        Self { path, language, state: FileState::Normal, rows: collapse_context(&rows) }
    }
}

/// Context lines kept adjacent to each change; longer unchanged runs collapse to a fold.
const FOLD_MARGIN: usize = 3;

/// Replace each run of unchanged `Context` rows that exceeds the margin with a single
/// `Fold` owning the hidden rows, keeping `FOLD_MARGIN` lines next to every change and
/// at the file head and tail.
fn collapse_context(rows: &[Row]) -> Vec<Row> {
    let n = rows.len();
    let mut keep = vec![false; n];
    for (i, row) in rows.iter().enumerate() {
        if matches!(row, Row::Context { .. }) {
            continue;
        }
        let lo = i.saturating_sub(FOLD_MARGIN);
        let hi = (i + FOLD_MARGIN).min(n - 1);
        keep[lo..=hi].iter_mut().for_each(|k| *k = true);
    }

    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if keep[i] {
            out.push(rows[i].clone());
            i += 1;
            continue;
        }
        let start = i;
        while i < n && !keep[i] {
            i += 1;
        }
        // A single hidden line is shown as-is — a `⋯ 1 line` fold would save nothing.
        if i - start > 1 {
            out.push(Row::Fold { lines: rows[start..i].to_vec() });
        } else {
            out.extend(rows[start..i].iter().cloned());
        }
    }
    out
}

/// The extension used to pick a syntax, e.g. `rs` for `src/app.rs`; `None` when the
/// file name has no extension.
fn language_of(path: &str) -> Option<String> {
    Path::new(path).extension().and_then(|e| e.to_str()).map(str::to_string)
}

/// Caches built `FileDiff`s by content, so an unchanged poll skips diffing and
/// highlighting.
#[derive(Default, Debug)]
pub struct DiffCache {
    entries: HashMap<String, (u64, FileDiff)>,
}

/// Cap the cache so a long session browsing many files cannot grow it without bound;
/// at the cap it is cleared (only the open file is ever rebuilt).
const CACHE_CAP: usize = 256;

impl DiffCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the cached diff when `old`/`new` are unchanged for `path`, else build,
    /// cache, and return it.
    pub fn get(&mut self, path: String, old: &str, new: &str, hl: &Highlighter) -> FileDiff {
        let key = content_hash(old, new);
        if let Some((cached, diff)) = self.entries.get(&path)
            && *cached == key
        {
            return diff.clone();
        }
        let diff = FileDiff::build(path.clone(), old, new, hl);
        if self.entries.len() >= CACHE_CAP {
            self.entries.clear();
        }
        self.entries.insert(path, (key, diff.clone()));
        diff
    }
}

fn content_hash(old: &str, new: &str) -> u64 {
    let mut h = DefaultHasher::new();
    old.hash(&mut h);
    new.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::{DiffCache, FileDiff, FileState, Row, language_of};
    use crate::highlight::Highlighter;

    fn build(old: &str, new: &str) -> FileDiff {
        let hl = Highlighter::new(None);
        FileDiff::build("a.rs".into(), old, new, &hl)
    }

    #[test]
    fn rows_carry_sides_numbers_and_markers() {
        let d = build("alpha\nbeta\ngamma\n", "alpha\nBETA\ngamma\n");
        assert_eq!(d.state, FileState::Normal);
        let del = d.rows.iter().find(|r| matches!(r, Row::Deletion { .. })).unwrap();
        let ins = d.rows.iter().find(|r| matches!(r, Row::Insertion { .. })).unwrap();
        assert_eq!(del.old_no(), Some(2));
        assert_eq!(del.new_no(), None);
        assert_eq!(ins.new_no(), Some(2));
        assert_eq!(del.marker_text(), "-beta");
        assert_eq!(ins.marker_text(), "+BETA");
        // The whole file is shown — context rows surround the change (no folds in M1).
        assert!(d.rows.iter().filter(|r| matches!(r, Row::Context { .. })).count() >= 2);
    }

    #[test]
    fn long_unchanged_runs_collapse_to_a_fold() {
        use std::fmt::Write as _;
        let mut old = String::new();
        for i in 0..40 {
            writeln!(old, "line {i}").unwrap();
        }
        let new = old.replace("line 20", "LINE 20");
        let d = build(&old, &new);
        // The middle is one change with 3 context lines each side; the long head and tail
        // unchanged runs each collapse to a fold.
        let folds = d.rows.iter().filter(|r| matches!(r, Row::Fold { .. })).count();
        assert_eq!(folds, 2, "leading and trailing runs fold");
        let change = d.rows.iter().find(|r| matches!(r, Row::Insertion { .. })).unwrap();
        assert_eq!(change.new_no(), Some(21)); // line 20 is 1-based line 21
    }

    #[test]
    fn binary_content_is_flagged_not_rowed() {
        let d = build("ok\n", "bin\0ary\n");
        assert_eq!(d.state, FileState::Binary);
        assert!(d.rows.is_empty());
    }

    #[test]
    fn language_comes_from_the_extension() {
        assert_eq!(language_of("src/app.rs").as_deref(), Some("rs"));
        assert_eq!(language_of("Makefile"), None);
        assert_eq!(language_of("a/b.tar.gz").as_deref(), Some("gz"));
    }

    #[test]
    fn cache_reuses_an_unchanged_build() {
        let hl = Highlighter::new(None);
        let mut cache = DiffCache::new();
        let d1 = cache.get("a.rs".into(), "x\n", "y\n", &hl);
        let d2 = cache.get("a.rs".into(), "x\n", "y\n", &hl);
        assert_eq!(d1, d2);
    }
}
