//! PC-15: the shellcheck check, with a stand-in program so the tests do not depend on shellcheck being installed.
use pipeline_check::config::Shellcheck;
use pipeline_check::report::{Check, Status};
use pipeline_check::shellcheck::check_with;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn repo(files: &[(&str, &str)]) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!("pc-sc-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&root).unwrap();
    for (p, t) in files {
        std::fs::create_dir_all(root.join(p).parent().unwrap()).unwrap();
        std::fs::write(root.join(p), t).unwrap();
    }
    assert!(Command::new("git").args(["init", "-q"]).current_dir(&root).status().unwrap().success());
    assert!(Command::new("git").args(["add", "-A"]).current_dir(&root).status().unwrap().success());
    root
}

/// A program that logs its arguments and fails (printing an `In ...` line) when any file it is given contains BAD.
fn fake(dir: &Path) -> String {
    let path = dir.join("fake-shellcheck");
    std::fs::write(&path, "#!/bin/bash\necho \"$@\" > \"$(dirname \"$0\")/args.txt\"\nshift 4\nfor f in \"$@\"; do grep -q BAD \"$f\" && { echo \"In $f line 1:\"; exit 1; }; done\nexit 0\n").unwrap();
    Command::new("chmod").arg("+x").arg(&path).status().unwrap();
    path.display().to_string()
}

fn cfg(extra: &[&str]) -> Shellcheck {
    Shellcheck { severity: "warning".into(), exclude: vec!["SC1090".into(), "SC1091".into()], extra_files: extra.iter().map(|s| s.to_string()).collect() }
}

fn show(c: &Check) -> (Status, String) {
    (c.status, c.nota.clone().unwrap_or_default())
}

// REQ PC-15
#[test]
fn a_clean_project_is_ok_and_the_tracked_scripts_and_existing_extra_files_are_linted_at_the_severity() {
    let root = repo(&[("a.sh", "echo ok\n"), ("tools/b.sh", "echo ok\n"), ("tools/hooks/pre-push", "echo ok\n"), ("README.md", "BAD but not a script\n")]);
    let program = fake(&root);
    let c = check_with(&root, &cfg(&["tools/hooks/pre-push", "missing/file"]), &program);
    assert_eq!(show(&c), (Status::Ok, "3 scripts".to_string()));
    let args = std::fs::read_to_string(root.join("args.txt")).unwrap();
    assert_eq!(args.trim(), "-S warning -e SC1090,SC1091 a.sh tools/b.sh tools/hooks/pre-push");
}

// REQ PC-15
#[test]
fn a_finding_fails_and_names_the_first_one() {
    let root = repo(&[("a.sh", "echo ok\n"), ("z.sh", "BAD\n")]);
    let c = check_with(&root, &cfg(&[]), &fake(&root));
    assert_eq!(show(&c), (Status::Falha, "In z.sh line 1:".to_string()));
}

// REQ PC-15
#[test]
fn no_scripts_is_ok_and_a_missing_shellcheck_or_a_folder_outside_git_is_pulado() {
    let none = repo(&[("README.md", "hi\n")]);
    assert_eq!(show(&check_with(&none, &cfg(&[]), "no-such-shellcheck")), (Status::Ok, "no shell scripts".to_string()));
    let some = repo(&[("a.sh", "echo ok\n")]);
    assert_eq!(check_with(&some, &cfg(&[]), "no-such-shellcheck").status, Status::Pulado);
    let outside = std::env::temp_dir().join(format!("pc-sc-nogit-{}", std::process::id()));
    std::fs::create_dir_all(&outside).unwrap();
    assert_eq!(check_with(&outside, &cfg(&[]), "no-such-shellcheck").status, Status::Pulado);
}

// REQ PC-15
#[test]
fn the_real_shellcheck_when_installed_agrees_with_a_clean_and_a_broken_script() {
    if Command::new("shellcheck").arg("--version").output().is_err() {
        return;
    }
    let clean = repo(&[("a.sh", "#!/bin/bash\necho \"$HOME\"\n")]);
    assert_eq!(check_with(&clean, &cfg(&[]), "shellcheck").status, Status::Ok);
    let sourcing = repo(&[("a.sh", "#!/bin/bash\nsource \"$HOME/.cargo/env\"\n")]);
    assert_eq!(check_with(&sourcing, &cfg(&[]), "shellcheck").status, Status::Ok, "sourcing a computed path is allowed by default");
    let broken = repo(&[("a.sh", "#!/bin/bash\nrm -rf $UNSET_DIR/*\ncd \"~/x\"\n")]);
    assert_eq!(check_with(&broken, &cfg(&[]), "shellcheck").status, Status::Falha);
}

// REQ PC-15
#[test]
fn the_check_is_part_of_the_measure_only_run_that_every_gate_calls() {
    use pipeline_check::config::Config;
    use pipeline_check::github::Gh;
    use pipeline_check::report::Mode;
    use pipeline_check::{run, Options};
    struct NoGh;
    impl Gh for NoGh {
        fn run(&self, _: &[&str]) -> Result<String, String> {
            Err("gh unavailable".into())
        }
    }
    let root = repo(&[("a.sh", "echo ok\n")]);
    let toml = "projeto = \"x\"\n[gate]\ncommand = \"true\"\n[architecture]\npure_dirs = []\n[shellcheck]\n";
    std::fs::write(root.join("pipeline-check.toml"), toml).unwrap();
    let cfg = Config::load(&root.join("pipeline-check.toml")).unwrap();
    let r = run(&root, &cfg, &Options { mode: Mode::Local, skip_gate: true, measure_only: true, no_github: true, now: 0 }, &NoGh);
    assert!(r.checks.iter().any(|c| c.id == "shellcheck"));
}
