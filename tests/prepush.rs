//! The local pre-push check, against real throwaway git repositories (PC-13).
use pipeline_check::config::Config;
use pipeline_check::github::Gh;
use pipeline_check::prepush::{changed_files, guarded, pre_push};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const ZERO: &str = "0000000000000000000000000000000000000000";

struct NoGh;
impl Gh for NoGh {
    fn run(&self, _: &[&str]) -> Result<String, String> {
        panic!("the pre-push check must not ask GitHub")
    }
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(root).args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A repo with `files` committed; returns (root, head sha).
fn repo(files: &[&str], gate: &str, guard: &str) -> (PathBuf, String, Config) {
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!("pc-prepush-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["config", "user.email", "t@t"]);
    git(&root, &["config", "user.name", "t"]);
    for f in files {
        std::fs::create_dir_all(root.join(f).parent().unwrap()).unwrap();
        std::fs::write(root.join(f), "x").unwrap();
    }
    let toml = format!("projeto = \"p\"\n[gate]\ncommand = \"{gate}\"\n[architecture]\npure_dirs = []\n[pre_push]\nguard = {guard}\n");
    std::fs::write(root.join("pipeline-check.toml"), toml).unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "one"]);
    let sha = git(&root, &["rev-parse", "HEAD"]);
    let cfg = Config::load(&root.join("pipeline-check.toml")).unwrap();
    (root, sha, cfg)
}

fn line(local: &str, remote_ref: &str, remote: &str) -> String {
    format!("refs/heads/main {local} {remote_ref} {remote}\n")
}

// REQ PC-13
#[test]
fn a_failing_check_blocks_a_push_of_main() {
    let (root, sha, cfg) = repo(&["src/a.kt"], "exit 1", "[]");
    assert_eq!(pre_push(&root, &cfg, &line(&sha, "refs/heads/main", ZERO), 0, &NoGh), 1);
}

// REQ PC-13
#[test]
fn a_passing_check_lets_main_through() {
    let (root, sha, cfg) = repo(&["src/a.kt"], "true", "[]");
    assert_eq!(pre_push(&root, &cfg, &line(&sha, "refs/heads/main", ZERO), 0, &NoGh), 0);
}

// REQ PC-13
#[test]
fn other_branches_and_deletions_are_never_checked() {
    let (root, sha, cfg) = repo(&["src/a.kt"], "exit 1", "[]");
    assert_eq!(pre_push(&root, &cfg, &line(&sha, "refs/heads/feature", ZERO), 0, &NoGh), 0);
    assert_eq!(pre_push(&root, &cfg, &line(ZERO, "refs/heads/main", &sha), 0, &NoGh), 0);
    assert_eq!(pre_push(&root, &cfg, "", 0, &NoGh), 0);
}

// REQ PC-13
#[test]
fn a_push_that_changes_only_unguarded_paths_is_not_checked() {
    let (root, first, cfg) = repo(&["src/a.kt", "events/e1.md"], "exit 1", "[\"src/\"]");
    std::fs::write(root.join("events/e2.md"), "y").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "event"]);
    let second = git(&root, &["rev-parse", "HEAD"]);
    assert_eq!(changed_files(&root, &first, &second), ["events/e2.md"]);
    assert_eq!(pre_push(&root, &cfg, &line(&second, "refs/heads/main", &first), 0, &NoGh), 0);
    std::fs::write(root.join("src/b.kt"), "z").unwrap();
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "code"]);
    let third = git(&root, &["rev-parse", "HEAD"]);
    assert_eq!(pre_push(&root, &cfg, &line(&third, "refs/heads/main", &second), 0, &NoGh), 1);
}

// REQ PC-13
#[test]
fn a_new_main_or_an_unknown_remote_commit_checks_every_file() {
    let (root, sha, cfg) = repo(&["src/a.kt"], "exit 1", "[\"src/\"]");
    assert_eq!(changed_files(&root, ZERO, &sha).len(), 2); // src/a.kt and pipeline-check.toml
    let unknown = "1111111111111111111111111111111111111111";
    assert_eq!(pre_push(&root, &cfg, &line(&sha, "refs/heads/main", unknown), 0, &NoGh), 1);
}

// REQ PC-13
#[test]
fn the_guard_matches_by_prefix_and_an_empty_guard_matches_everything() {
    let files = vec!["docs/a.md".to_string()];
    assert!(guarded(&files, &[]));
    assert!(guarded(&files, &["docs/".to_string()]));
    assert!(!guarded(&files, &["doc/x".to_string(), "src/".to_string()]));
    assert!(!guarded(&[], &[]), "nothing changed, nothing to check");
}

// REQ PC-13
#[test]
fn the_binary_reads_the_refs_from_stdin_and_exits_1_to_block() {
    let (root, sha, _) = repo(&["src/a.kt"], "exit 1", "[]");
    let mut child = Command::new(env!("CARGO_BIN_EXE_pipeline-check"))
        .args(["--pre-push", "--root"]).arg(&root)
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().unwrap();
    std::io::Write::write_all(child.stdin.as_mut().unwrap(), line(&sha, "refs/heads/main", ZERO).as_bytes()).unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(1));
}
