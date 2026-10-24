//! The local stand-in for branch protection: `git push` of main runs the checks first (PC-13).
//! Git feeds the hook one line per ref on stdin: `<local ref> <local sha> <remote ref> <remote sha>`.
use crate::config::Config;
use crate::github::Gh;
use crate::report::Mode;
use crate::{run, Options};
use std::path::Path;
use std::process::Command;

const ZERO: &str = "0000000000000000000000000000000000000000";

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(root).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Files a push would change: the diff when the remote commit is known here, else every file of the pushed commit.
pub fn changed_files(root: &Path, remote_sha: &str, local_sha: &str) -> Vec<String> {
    let known = remote_sha != ZERO && git(root, &["cat-file", "-e", remote_sha]).is_some();
    let text = if known {
        git(root, &["diff", "--name-only", remote_sha, local_sha])
    } else {
        git(root, &["ls-tree", "-r", "--name-only", local_sha])
    };
    text.unwrap_or_default().lines().map(str::to_string).collect()
}

pub fn guarded(files: &[String], guard: &[String]) -> bool {
    !files.is_empty() && (guard.is_empty() || files.iter().any(|f| guard.iter().any(|g| f.starts_with(g.as_str()))))
}

/// Exit code for the hook: 1 blocks the push.
pub fn pre_push(root: &Path, cfg: &Config, stdin: &str, now: u64, gh: &dyn Gh) -> i32 {
    for line in stdin.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let [_, local_sha, remote_ref, remote_sha] = f[..] else { continue };
        if remote_ref != "refs/heads/main" || local_sha == ZERO {
            continue;
        }
        let files = changed_files(root, remote_sha, local_sha);
        if !guarded(&files, &cfg.pre_push.guard) {
            continue;
        }
        println!("pre-push: running pipeline-check before pushing main (skip with git push --no-verify)");
        let opts = Options { mode: Mode::Local, skip_gate: false, measure_only: false, no_github: true, now };
        let report = run(root, cfg, &opts, gh);
        for c in &report.checks {
            println!("{:<8} {:<22} {}", format!("{:?}", c.status).to_uppercase(), c.id, c.nota.as_deref().unwrap_or(""));
        }
        if report.exit_code() != 0 {
            eprintln!("pre-push: a check failed; main was not pushed");
            return 1;
        }
    }
    0
}
