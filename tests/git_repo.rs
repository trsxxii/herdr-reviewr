//! Integration tests for `git.rs` against real repositories.

mod common;

use std::collections::HashMap;

use common::Repo;
use herdr_review::git::{changed_files, file_content, merge_base};
use herdr_review::model::{ChangeKind, ChangedFile, Scope};

fn by_path(files: &[ChangedFile]) -> HashMap<&str, &ChangedFile> {
    files.iter().map(|f| (f.path.as_str(), f)).collect()
}

#[test]
fn lists_every_change_kind_with_stats() {
    let r = Repo::init();
    r.write("keep.rs", "fn a() {}\n");
    r.write("gone.rs", "fn g() {}\n");
    r.write("edit.rs", "one\ntwo\nthree\n");
    r.commit_all("init");

    r.write("edit.rs", "one\nTWO\nthree\nfour\n"); // modify
    r.write("added.rs", "new\n"); // staged add
    r.git(&["add", "added.rs"]);
    r.remove("gone.rs"); // delete
    r.write("untracked.rs", "u\n"); // untracked

    let files = changed_files(r.path(), Scope::Uncommitted, None).unwrap();
    let files = by_path(&files);

    assert_eq!(files["edit.rs"].kind, ChangeKind::Modified);
    assert_eq!(files["added.rs"].kind, ChangeKind::Added);
    assert_eq!(files["gone.rs"].kind, ChangeKind::Deleted);
    assert_eq!(files["untracked.rs"].kind, ChangeKind::Untracked);
    assert!(files["edit.rs"].additions >= 1, "additions counted");
    assert!(files["edit.rs"].deletions >= 1, "deletions counted");
}

#[test]
fn file_content_reads_the_committed_version_not_the_worktree() {
    let r = Repo::init();
    r.write("a.rs", "alpha\nbeta\ngamma\n");
    r.commit_all("init");
    r.write("a.rs", "alpha\nBETA\ngamma\n"); // the worktree moves on

    // The old side of a diff: HEAD's content, not the working tree.
    assert_eq!(file_content(r.path(), "HEAD", "a.rs"), "alpha\nbeta\ngamma\n");
}

#[test]
fn file_content_is_empty_for_a_path_absent_at_that_rev() {
    let r = Repo::init();
    r.write("seed.rs", "x\n");
    r.commit_all("init");
    r.write("fresh.rs", "line one\nline two\n"); // untracked — not in HEAD

    // An added/untracked file has no old side, so its HEAD content is empty.
    assert_eq!(file_content(r.path(), "HEAD", "fresh.rs"), "");
    let files = changed_files(r.path(), Scope::Uncommitted, None).unwrap();
    assert_eq!(by_path(&files)["fresh.rs"].additions, 2);
}

#[test]
fn merge_base_is_the_branch_point() {
    let r = Repo::init();
    r.write("base.rs", "1\n");
    r.commit_all("base");
    let branch_point = r.git(&["rev-parse", "HEAD"]).trim().to_string();
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("base.rs", "2\n");
    r.commit_all("diverge");

    assert_eq!(merge_base(r.path(), Some("main")), Some(branch_point));
}

#[test]
fn branch_scope_diffs_against_base_not_working_tree() {
    let r = Repo::init();
    r.write("base.rs", "1\n");
    r.commit_all("base");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("feature.rs", "new\n");
    r.commit_all("feature work");

    let branch = changed_files(r.path(), Scope::Branch, Some("main")).unwrap();
    assert!(branch.iter().any(|f| f.path == "feature.rs"), "branch shows committed work");

    // The tree is clean, so the uncommitted scope is empty.
    let uncommitted = changed_files(r.path(), Scope::Uncommitted, None).unwrap();
    assert!(uncommitted.is_empty(), "uncommitted is empty on a clean tree");
}

#[test]
fn branch_scope_falls_back_to_master_when_main_is_absent() {
    let r = Repo::init();
    r.write("base.rs", "1\n");
    r.commit_all("base");
    r.git(&["branch", "-m", "main", "master"]); // no `main` ref exists anymore
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("feature.rs", "x\n");
    r.commit_all("feature work");

    // base = None → the fallback chain (origin/main, origin/master, main, master) finds master.
    let files = changed_files(r.path(), Scope::Branch, None).unwrap();
    assert!(files.iter().any(|f| f.path == "feature.rs"), "resolved master as the base ref");
}

#[test]
fn rename_is_reported_at_the_new_path() {
    let r = Repo::init();
    r.write("old_name.rs", "stable contents that survive the move\n");
    r.commit_all("init");
    r.git(&["mv", "old_name.rs", "new_name.rs"]);

    let files = changed_files(r.path(), Scope::Uncommitted, None).unwrap();
    let renamed = files.iter().find(|f| f.kind == ChangeKind::Renamed).expect("a renamed file");
    assert_eq!(renamed.path, "new_name.rs");
    // The old path is carried so the diff can read the old content and show `old → new`.
    assert_eq!(renamed.previous_path.as_deref(), Some("old_name.rs"));
}

#[test]
fn a_directory_removing_rename_keeps_its_stats() {
    // Regression for the `-z` migration: `a/b/f.rs -> a/f.rs` once produced a `a//f.rs`
    // numstat key that never matched, so the renamed+edited file showed +0 -0.
    let r = Repo::init();
    r.write("a/b/file.rs", "one\ntwo\nthree\nfour\nfive\nsix\n");
    r.commit_all("init");
    r.git(&["mv", "a/b/file.rs", "a/file.rs"]);
    r.write("a/file.rs", "one\nTWO\nthree\nfour\nfive\nsix\n"); // small edit keeps it a rename

    let files = changed_files(r.path(), Scope::Uncommitted, None).unwrap();
    let renamed = files.iter().find(|f| f.kind == ChangeKind::Renamed).expect("a renamed file");
    assert_eq!(renamed.path, "a/file.rs");
    assert_eq!(renamed.previous_path.as_deref(), Some("a/b/file.rs"));
    assert!(renamed.additions + renamed.deletions > 0, "the edit's stats are counted");
}

#[test]
fn untracked_paths_with_spaces_survive_verbatim() {
    // `-z` status never quotes or trims, so a name with spaces round-trips byte-for-byte.
    let r = Repo::init();
    r.write("seed.rs", "x\n");
    r.commit_all("init");
    r.write("a file with spaces.rs", "u\n");

    let files = changed_files(r.path(), Scope::Uncommitted, None).unwrap();
    let f = by_path(&files)["a file with spaces.rs"];
    assert_eq!(f.kind, ChangeKind::Untracked);
    assert_eq!(f.additions, 1);
}

#[test]
fn untracked_files_in_a_new_directory_are_listed_individually() {
    // git collapses a brand-new directory to one `dir/` entry by default; `--untracked-files=all`
    // expands it so each new file is reviewable, not the directory.
    let r = Repo::init();
    r.write("seed.rs", "x\n");
    r.commit_all("init");
    r.write("docs/new/a.md", "alpha\n");
    r.write("docs/new/b.md", "beta\n");

    let files = changed_files(r.path(), Scope::Uncommitted, None).unwrap();
    let by = by_path(&files);
    assert!(by.contains_key("docs/new/a.md"), "the file is listed, not the directory");
    assert!(by.contains_key("docs/new/b.md"));
    assert!(!by.contains_key("docs/new/"), "the bare directory is not an entry");
    assert_eq!(by["docs/new/a.md"].kind, ChangeKind::Untracked);
}

#[test]
fn a_binary_change_lists_with_zero_stats() {
    let r = Repo::init();
    r.write("blob.bin", "\0\0seed\0\0");
    r.commit_all("init");
    r.write("blob.bin", "\0\0changed\0\0\0");

    let files = changed_files(r.path(), Scope::Uncommitted, None).unwrap();
    let f = by_path(&files)["blob.bin"];
    assert_eq!(f.kind, ChangeKind::Modified);
    assert_eq!((f.additions, f.deletions), (0, 0));
}

#[test]
fn git_access_never_mutates_the_repo() {
    let r = Repo::init();
    r.write("a.rs", "x\n");
    r.commit_all("init");
    r.write("a.rs", "y\n");

    let head_before = r.git(&["rev-parse", "HEAD"]);
    let status_before = r.git(&["status", "--porcelain"]);

    let _ = changed_files(r.path(), Scope::Uncommitted, None).unwrap();
    let _ = file_content(r.path(), "HEAD", "a.rs");
    let _ = changed_files(r.path(), Scope::Branch, Some("main")).unwrap();

    assert_eq!(head_before, r.git(&["rev-parse", "HEAD"]), "HEAD unchanged");
    assert_eq!(status_before, r.git(&["status", "--porcelain"]), "working tree unchanged");
}
