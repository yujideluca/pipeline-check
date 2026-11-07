//! Check a: the project gate runs once; its exit code and duration are recorded.
use crate::config::Gate;
use crate::report::{Check, Status};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub const ID: &str = "gate";

/// PC-19: variables that tell git which repository to use. Git exports some of them to hooks (in a worktree
/// `GIT_DIR` points at the real repository), so a gate started by the pre-push hook would let any test that runs git
/// in a temporary repository act on the repository being pushed. The gate never inherits them; git finds the
/// repository from its working directory as in a terminal.
pub const GIT_LOCATION_VARS: [&str; 9] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
    "GIT_PREFIX",
    "GIT_QUARANTINE_PATH",
];

/// The gate command, started in `root` without the git location variables (PC-19).
pub fn command(root: &Path, cfg: &Gate) -> Command {
    let mut c = Command::new("sh");
    c.arg("-c").arg(&cfg.command).current_dir(root);
    for v in GIT_LOCATION_VARS {
        c.env_remove(v);
    }
    c
}

/// Returns the check and the gate's duration in seconds when it ran.
pub fn run(root: &Path, cfg: &Gate, skip: bool) -> (Check, Option<f64>) {
    if skip {
        return (Check::skipped(ID, cfg.required, "gate not run (--no-gate)"), None);
    }
    let start = Instant::now();
    let result = command(root, cfg).status();
    let secs = start.elapsed().as_secs_f64();
    match result {
        Ok(s) if s.success() => (Check::new(ID, Status::Ok, Some(secs), None, None), Some(secs)),
        Ok(s) => {
            let nota = format!("gate exited with {}", s.code().map_or("a signal".into(), |c| c.to_string()));
            (Check::new(ID, Status::Falha, Some(secs), None, Some(nota)), Some(secs))
        }
        Err(e) => (Check::new(ID, Status::Falha, None, None, Some(format!("cannot start the gate: {e}"))), None),
    }
}
