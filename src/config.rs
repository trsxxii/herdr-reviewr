//! Configuration: command-line flags plus the `config.toml` keep list.
//!
//! See `specs/tui.md`, `specs/herdr-host.md`, and `specs/config.md`. Flags override
//! defaults; the positional argument (if any) is the repo path, else the current
//! directory. The `keep` list is read from herdr's per-plugin config dir.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// Resolved runtime configuration.
#[derive(Clone, Debug)]
pub struct Config {
    pub repo: PathBuf,
    pub poll: Duration,
    pub base: Option<String>,
    pub theme: Option<String>,
    /// `Some(false)` when `--wrap off` is passed; `None` keeps the default (wrap on).
    pub wrap: Option<bool>,
    /// The `config.toml` under `$HERDR_PLUGIN_CONFIG_DIR`, re-read each reload; `None`
    /// outside a herdr pane.
    pub config_path: Option<PathBuf>,
}

/// The reviewr config file under herdr's per-plugin config dir (`specs/config.md`), or
/// `None` when `HERDR_PLUGIN_CONFIG_DIR` is unset — outside a herdr pane.
pub fn config_path() -> Option<PathBuf> {
    std::env::var_os("HERDR_PLUGIN_CONFIG_DIR").map(|d| PathBuf::from(d).join("config.toml"))
}

#[derive(Debug, Default, serde::Deserialize)]
struct ConfigFile {
    #[serde(default)]
    keep: Vec<String>,
}

/// The `keep` patterns from a `config.toml`. `Ok(empty)` when the file is absent or sets
/// no `keep`; `Err(message)` when the file exists but does not parse (`specs/config.md`).
pub fn load_keep(path: &Path) -> Result<Vec<String>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
    };
    toml::from_str::<ConfigFile>(&text).map(|c| c.keep).map_err(|e| e.to_string())
}

impl Config {
    /// Parse `args` (the process arguments *after* argv\[0\]).
    ///
    /// Recognises `--poll <ms>` (min 200, default 2000), `--base <ref>`,
    /// `--theme <name>`, and `--wrap on|off`; the first non-flag token is the repo path.
    pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Self {
        let mut repo: Option<PathBuf> = None;
        let mut poll_ms: u64 = 2000;
        let mut base: Option<String> = None;
        let mut theme: Option<String> = None;
        let mut wrap: Option<bool> = None;
        let mut it = args.into_iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--poll" => {
                    if let Some(v) = it.next() {
                        poll_ms = v.parse().unwrap_or(poll_ms);
                    }
                }
                "--base" => base = it.next(),
                "--theme" => theme = it.next(),
                "--wrap" => wrap = it.next().map(|v| v != "off"),
                other if !other.starts_with('-') => repo = Some(PathBuf::from(other)),
                _ => {}
            }
        }
        let repo =
            repo.or_else(|| std::env::current_dir().ok()).unwrap_or_else(|| PathBuf::from("."));
        Self {
            repo,
            poll: Duration::from_millis(poll_ms.max(200)),
            base,
            theme,
            wrap,
            config_path: config_path(),
        }
    }

    /// Parse from the real process arguments.
    pub fn from_env() -> Self {
        Self::parse(std::env::args().skip(1))
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use std::time::Duration;

    fn parse(args: &[&str]) -> Config {
        Config::parse(args.iter().map(|s| (*s).to_string()))
    }

    #[test]
    fn defaults_when_no_args() {
        let c = parse(&[]);
        assert_eq!(c.poll, Duration::from_secs(2));
        assert_eq!(c.base, None);
    }

    #[test]
    fn flags_and_positional_repo() {
        let c = parse(&["--poll", "500", "--base", "origin/dev", "/tmp/work"]);
        assert_eq!(c.poll, Duration::from_millis(500));
        assert_eq!(c.base.as_deref(), Some("origin/dev"));
        assert_eq!(c.repo.to_str(), Some("/tmp/work"));
    }

    #[test]
    fn poll_has_a_floor() {
        assert_eq!(parse(&["--poll", "10"]).poll, Duration::from_millis(200));
        assert_eq!(parse(&["--poll", "garbage"]).poll, Duration::from_secs(2));
    }

    #[test]
    fn load_keep_reads_the_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "keep = [\"docs/plans/\", \".env.example\"]\n").unwrap();
        assert_eq!(super::load_keep(&path).unwrap(), ["docs/plans/", ".env.example"]);
    }

    #[test]
    fn load_keep_is_empty_when_absent_or_unset() {
        let dir = tempfile::tempdir().unwrap();
        assert!(super::load_keep(&dir.path().join("nope.toml")).unwrap().is_empty());
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "# no keep key\n").unwrap();
        assert!(super::load_keep(&path).unwrap().is_empty());
    }

    #[test]
    fn load_keep_errors_on_malformed_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "keep = [unterminated\n").unwrap();
        assert!(super::load_keep(&path).is_err());
    }
}
