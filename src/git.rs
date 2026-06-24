//! Read-only git access: scopes, changed files, and diffs.
//!
//! See `specs/review-model.md`. Every call here only reads — it never commits,
//! stages, or mutates the worktree or refs.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::model::{ChangeKind, ChangedFile, Scope};

/// Run `git -C <repo> <args>` and return stdout. Errors on non-zero exit.
fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "core.quotepath=false"])
        .args(args)
        .output()
        .with_context(|| format!("running git {args:?}"))?;
    if !out.status.success() {
        bail!("git {args:?} failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Like [`git`], but returns stdout even on non-zero exit (e.g. `diff --no-index`).
fn git_lenient(repo: &Path, args: &[&str]) -> String {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "core.quotepath=false"])
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// Whether `path` is inside a git work tree.
pub fn is_repo(path: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The git top-level of `path`, or `None` if it is not a repo.
pub fn toplevel(path: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let top = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!top.is_empty()).then(|| PathBuf::from(top))
}

/// Whether `git_ref` resolves in `repo`.
fn ref_exists(repo: &Path, git_ref: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--verify", "--quiet", git_ref])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The base ref for branch scope: `base` if it resolves, otherwise the first of
/// `origin/main`, `origin/master`, `main`, `master`.
fn base_ref(repo: &Path, base: Option<&str>) -> Option<String> {
    if let Some(b) = base
        && !b.is_empty()
        && ref_exists(repo, b)
    {
        return Some(b.to_string());
    }
    ["origin/main", "origin/master", "main", "master"]
        .into_iter()
        .find(|cand| ref_exists(repo, cand))
        .map(String::from)
}

/// The diff range for a scope. `None` means working tree vs `HEAD`.
fn range(repo: &Path, scope: Scope, base: Option<&str>) -> Option<String> {
    match scope {
        Scope::Uncommitted => None,
        // `base...HEAD` diffs against the merge-base, which is what branch scope means.
        Scope::Branch => base_ref(repo, base).map(|b| format!("{b}...HEAD")),
    }
}

/// The merge-base commit of `base` and `HEAD`, the old side of a branch-scope diff.
pub fn merge_base(repo: &Path, base: Option<&str>) -> Option<String> {
    let base = base_ref(repo, base)?;
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["merge-base", &base, "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let mb = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!mb.is_empty()).then_some(mb)
}

/// The content of `path` at `rev` (`git show <rev>:<path>`). Empty when the path does
/// not exist at that rev — an added file against its old side, say.
pub fn file_content(repo: &Path, rev: &str, path: &str) -> String {
    git_lenient(repo, &["show", &format!("{rev}:{path}")])
}

/// The changed files for `scope`, sorted by path. `base` overrides the branch base ref.
pub fn changed_files(repo: &Path, scope: Scope, base: Option<&str>) -> Result<Vec<ChangedFile>> {
    let (numstat, name_status) = match scope {
        Scope::Uncommitted => (
            git(repo, &["diff", "HEAD", "--numstat"])?,
            git(repo, &["diff", "HEAD", "--name-status"])?,
        ),
        Scope::Branch => match range(repo, scope, base) {
            Some(r) => {
                (git(repo, &["diff", &r, "--numstat"])?, git(repo, &["diff", &r, "--name-status"])?)
            }
            None => return Ok(Vec::new()),
        },
    };

    let counts = parse_numstat(&numstat);
    let mut seen = HashSet::new();
    let mut files = Vec::new();
    for (kind, path) in parse_name_status(&name_status) {
        if !seen.insert(path.clone()) {
            continue;
        }
        let (additions, deletions) = counts.get(&path).copied().unwrap_or((0, 0));
        files.push(ChangedFile { path, kind, additions, deletions });
    }

    if scope == Scope::Uncommitted {
        for path in untracked(repo)? {
            if seen.insert(path.clone()) {
                let additions = untracked_additions(repo, &path);
                files.push(ChangedFile {
                    path,
                    kind: ChangeKind::Untracked,
                    additions,
                    deletions: 0,
                });
            }
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// Untracked file paths from `git status --porcelain`.
fn untracked(repo: &Path) -> Result<Vec<String>> {
    let status = git(repo, &["status", "--porcelain"])?;
    Ok(status
        .lines()
        .filter_map(|l| l.strip_prefix("?? "))
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect())
}

/// Addition count of an untracked file via `diff --no-index` (0 for binary).
fn untracked_additions(repo: &Path, path: &str) -> u32 {
    let ns = git_lenient(repo, &["diff", "--no-index", "--numstat", "--", "/dev/null", path]);
    ns.lines().next().and_then(|l| l.split('\t').next()).and_then(|a| a.parse().ok()).unwrap_or(0)
}

// --- pure parsers (unit-tested without a repo) ---------------------------------

/// Map of path to `(additions, deletions)` from `git diff --numstat`.
fn parse_numstat(out: &str) -> HashMap<String, (u32, u32)> {
    let mut map = HashMap::new();
    for line in out.lines() {
        let mut it = line.split('\t');
        let add = it.next().unwrap_or("0").parse().unwrap_or(0);
        let del = it.next().unwrap_or("0").parse().unwrap_or(0);
        if let Some(path) = it.next()
            && !path.is_empty()
        {
            map.insert(path.to_string(), (add, del));
        }
    }
    map
}

/// `(kind, path)` pairs from `git diff --name-status`; renames take the new path.
fn parse_name_status(out: &str) -> Vec<(ChangeKind, String)> {
    let mut rows = Vec::new();
    for line in out.lines() {
        let mut it = line.split('\t');
        let Some(status) = it.next() else { continue };
        let first = it.next().unwrap_or("");
        let second = it.next();
        let (kind, path) = match status.chars().next() {
            Some('A') => (ChangeKind::Added, first),
            Some('D') => (ChangeKind::Deleted, first),
            Some('R') => (ChangeKind::Renamed, second.unwrap_or(first)),
            _ => (ChangeKind::Modified, first),
        };
        if !path.is_empty() {
            rows.push((kind, path.to_string()));
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::{ChangeKind, parse_name_status, parse_numstat};

    #[test]
    fn numstat_parses_counts_and_ignores_binary() {
        let m = parse_numstat("18\t8\tsrc/a.rs\n-\t-\tassets/logo.png\n");
        assert_eq!(m["src/a.rs"], (18, 8));
        assert_eq!(m["assets/logo.png"], (0, 0));
    }

    #[test]
    fn name_status_kinds_and_rename_target() {
        let rows =
            parse_name_status("M\tsrc/a.rs\nA\tsrc/b.rs\nD\tsrc/c.rs\nR100\told.rs\tnew.rs\n");
        assert_eq!(rows[0], (ChangeKind::Modified, "src/a.rs".to_string()));
        assert_eq!(rows[1], (ChangeKind::Added, "src/b.rs".to_string()));
        assert_eq!(rows[2], (ChangeKind::Deleted, "src/c.rs".to_string()));
        assert_eq!(rows[3], (ChangeKind::Renamed, "new.rs".to_string()));
    }
}
