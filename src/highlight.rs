//! Syntax highlighting via `syntect`, themed with a bundled Catppuccin Mocha theme.
//!
//! See `specs/diff-view.md`. The highlighter loads once at startup (the syntax set is
//! expensive to build) and produces per-line foreground spans; the pane keeps the
//! terminal's own background, so only token colors come from the theme.

use std::fmt;
use std::io::Cursor;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use std::sync::OnceLock;

use crate::diff::{Rgb, Span};

/// The Catppuccin Mocha theme, bundled so the binary needs no theme file at runtime.
const CATPPUCCIN_MOCHA: &[u8] = include_bytes!("../assets/Catppuccin Mocha.tmTheme");

/// The broad bat/two-face syntax set, built once per process (it is expensive to
/// deserialize) and shared across every `Highlighter`.
fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(two_face::syntax::extra_newlines)
}

/// Holds the active theme; highlights file content into spans against the shared
/// syntax set.
pub struct Highlighter {
    theme: Theme,
    default_fg: Rgb,
}

impl fmt::Debug for Highlighter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Highlighter").finish_non_exhaustive()
    }
}

impl Highlighter {
    /// Build with the named theme, or the bundled Catppuccin Mocha when `name` is unset
    /// or unknown. Most files color out of the box via the broad two-face syntax set.
    pub fn new(name: Option<&str>) -> Self {
        let theme = name
            .and_then(|n| ThemeSet::load_defaults().themes.remove(n))
            .unwrap_or_else(catppuccin_mocha);
        let default_fg = theme.settings.foreground.map_or((0xcd, 0xd6, 0xf4), |c| (c.r, c.g, c.b));
        Self { theme, default_fg }
    }

    /// Highlight `content` line by line. Each inner `Vec` is one line's spans. With no
    /// known `language`, every line is a single plain span in the theme's default color.
    pub fn highlight(&self, content: &str, language: Option<&str>) -> Vec<Vec<Span>> {
        let syntaxes = syntaxes();
        let syntax = language.and_then(|ext| syntaxes.find_syntax_by_extension(ext));
        let Some(syntax) = syntax else {
            return content
                .lines()
                .map(|l| vec![Span { text: l.to_string(), color: self.default_fg }])
                .collect();
        };
        let mut h = HighlightLines::new(syntax, &self.theme);
        let mut out = Vec::new();
        for line in LinesWithEndings::from(content) {
            let spans = match h.highlight_line(line, syntaxes) {
                Ok(regions) => regions
                    .into_iter()
                    .map(|(style, text)| Span {
                        text: text.trim_end_matches('\n').to_string(),
                        color: (style.foreground.r, style.foreground.g, style.foreground.b),
                    })
                    .collect(),
                // A grammar error degrades to plain text rather than blocking the diff.
                Err(_) => vec![Span {
                    text: line.trim_end_matches('\n').to_string(),
                    color: self.default_fg,
                }],
            };
            out.push(spans);
        }
        out
    }
}

/// Parse the bundled Catppuccin Mocha `.tmTheme`.
fn catppuccin_mocha() -> Theme {
    ThemeSet::load_from_reader(&mut Cursor::new(CATPPUCCIN_MOCHA))
        .expect("bundled Catppuccin Mocha theme parses")
}

#[cfg(test)]
mod tests {
    use super::Highlighter;

    #[test]
    fn highlights_rust_into_colored_spans() {
        let h = Highlighter::new(None);
        let lines = h.highlight("let x = 1;\n", Some("rs"));
        assert_eq!(lines.len(), 1);
        let spans = &lines[0];
        assert!(spans.len() > 1, "rust tokenizes into several spans");
        assert_eq!(spans.iter().map(|s| s.text.as_str()).collect::<String>(), "let x = 1;");
        // The Catppuccin keyword color (mauve) differs from the default text color.
        assert!(spans.iter().any(|s| s.text == "let" && s.color != (0xcd, 0xd6, 0xf4)));
    }

    #[test]
    fn unknown_language_is_one_plain_span_per_line() {
        let h = Highlighter::new(None);
        let lines = h.highlight("alpha\nbeta\n", None);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], vec![super::Span { text: "alpha".into(), color: (0xcd, 0xd6, 0xf4) }]);
    }
}
