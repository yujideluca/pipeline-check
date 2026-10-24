//! Check: the shell scripts of the project pass `shellcheck` (PC-15). Not installed means `pulado`, never `ok`.
use crate::config::Shellcheck;
use crate::report::{Check, Status};
use std::path::Path;
use std::process::Command;

pub const ID: &str = "shellcheck";

pub fn check(root: &Path, cfg: &Shellcheck) -> Check {
    check_with(root, cfg, "shellcheck")
}

/// `program` is a parameter so tests can stand in for the real shellcheck.
pub fn check_with(root: &Path, cfg: &Shellcheck, program: &str) -> Check {
    let listed = Command::new("git").args(["ls-files", "*.sh"]).current_dir(root).output();
    let Ok(listed) = listed.map_err(|e| e.to_string()).and_then(|o| if o.status.success() { Ok(o) } else { Err("git ls-files failed".into()) }) else {
        return Check::skipped(ID, false, "not a git repository");
    };
    let mut files: Vec<String> = String::from_utf8_lossy(&listed.stdout).lines().map(String::from).collect();
    files.extend(cfg.extra_files.iter().filter(|f| root.join(f).is_file()).cloned());
    if files.is_empty() {
        return Check::new(ID, Status::Ok, Some(0.0), None, Some("no shell scripts".into()));
    }
    let run = Command::new(program).args(["-S", &cfg.severity, "-e", &cfg.exclude.join(",")]).args(&files).current_dir(root).output();
    match run {
        Err(_) => Check::skipped(ID, false, "shellcheck is not installed"),
        Ok(o) if o.status.success() => Check::new(ID, Status::Ok, Some(files.len() as f64), None, Some(format!("{} scripts", files.len()))),
        Ok(o) => {
            let out = String::from_utf8_lossy(&o.stdout);
            let first = out.lines().find(|l| l.starts_with("In ")).unwrap_or("shellcheck reported problems");
            Check::new(ID, Status::Falha, Some(files.len() as f64), None, Some(first.to_string()))
        }
    }
}
