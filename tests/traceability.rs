//! PC-14: the traceability check, against small fixture projects.
use pipeline_check::config::{Source, Traceability};
use pipeline_check::traceability::{analyze, matrix, parse_requirements, run, tags_in_line};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

fn prefixes() -> Vec<String> {
    ["//", "#", "--", "/**", "*"].iter().map(|s| s.to_string()).collect()
}
const TABLE: &str = "intro\n| ID | Requirement |\n|---|---|\n| AB-1 | first | G |\n| AB-2 | second thing | D |\nnot | a row\n";

fn files(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

// REQ PC-14
#[test]
fn requirement_rows_are_read_and_header_rows_are_not() {
    assert_eq!(parse_requirements(TABLE), vec![("AB-1".into(), "first".into()), ("AB-2".into(), "second thing".into())]);
    assert!(parse_requirements("| ab-1 | lower | G |\n| A-1 | short | G |\n| AB- | none | G |\n| AB-1 |  | G |").is_empty());
}

// REQ PC-14
#[test]
fn tags_are_found_after_each_comment_opener_and_several_ids_per_line() {
    for line in ["// REQ AB-1", "  # REQ AB-1", "-- REQ AB-1", "/** REQ AB-1", " * REQ AB-1", "//REQ AB-1"] {
        assert_eq!(tags_in_line(line, &prefixes()), vec!["AB-1"], "{line}");
    }
    assert_eq!(tags_in_line("// REQ AB-1, AB-22 CD-3", &prefixes()), vec!["AB-1", "AB-22", "CD-3"]);
    for line in ["let x = 1; // REQ AB-1", "// REQ", "// REQ ab-1", "// see REQ AB-1", "REQ AB-1", "// REQAB-1"] {
        assert!(tags_in_line(line, &prefixes()).is_empty(), "{line}");
    }
}

// REQ PC-14
#[test]
fn an_untested_requirement_and_an_unknown_tag_both_fail_and_are_named() {
    let a = analyze(TABLE, &files(&[("t.rs", "// REQ AB-1\n// REQ ZZ-9\n")]), &prefixes());
    assert!(!a.ok());
    assert_eq!(a.missing(), vec!["AB-2"]);
    assert_eq!(a.unknown, vec!["t.rs names unknown requirement ZZ-9"]);
    let ok = analyze(TABLE, &files(&[("t.rs", "// REQ AB-1\n"), ("u.rs", "// REQ AB-2\n// REQ AB-2\n"), ("v.rs", "// REQ AB-1\n")]), &prefixes());
    assert!(ok.ok());
    assert_eq!(ok.found["AB-1"], vec!["t.rs", "v.rs"]);
    assert_eq!(ok.found["AB-2"], vec!["u.rs"], "a file is listed once");
}

// REQ PC-14
#[test]
fn the_matrix_lists_files_and_marks_gaps_and_shortens_long_text() {
    let long = format!("| AB-3 | {} | G |\n", "x".repeat(80));
    let a = analyze(&format!("{TABLE}{long}"), &files(&[("web:t.js", "// REQ AB-1\n")]), &prefixes());
    let m = matrix(&a);
    assert!(m.contains("| AB-1 first | web:t.js |"));
    assert!(m.contains("| AB-2 second thing | **none** |"));
    assert!(m.contains(&format!("| AB-3 {}… | **none** |", "x".repeat(70))));
    let exact = analyze(&format!("| AB-4 | {} | G |\n", "y".repeat(70)), &[], &prefixes());
    assert!(matrix(&exact).contains(&format!("| AB-4 {} | **none** |", "y".repeat(70))), "70 characters fit, no ellipsis");
    let over = analyze(&format!("| AB-5 | {} | G |\n", "z".repeat(71)), &[], &prefixes());
    assert!(matrix(&over).contains(&format!("| AB-5 {}… | **none** |", "z".repeat(70))), "71 characters are cut");
}

fn project(table: &str, tests: &[(&str, &str)]) -> (PathBuf, Traceability) {
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!("pc-trace-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/REQUIREMENTS.md"), table).unwrap();
    for (p, t) in tests {
        std::fs::create_dir_all(root.join(p).parent().unwrap()).unwrap();
        std::fs::write(root.join(p), t).unwrap();
    }
    let cfg = Traceability {
        requirements: "docs/REQUIREMENTS.md".into(),
        matrix: "docs/TRACEABILITY.md".into(),
        comment_prefixes: prefixes(),
        sources: vec![Source { dir: "tests".into(), extensions: vec!["rs".into()], label: String::new(), env: None }],
    };
    (root, cfg)
}

// REQ PC-14
#[test]
fn run_returns_the_exit_code_and_always_writes_the_matrix() {
    let (root, cfg) = project(TABLE, &[("tests/a.rs", "// REQ AB-1\n"), ("tests/note.txt", "// REQ AB-2\n")]);
    assert_eq!(run(&root, &cfg), 1, "only files with a configured extension count");
    assert!(std::fs::read_to_string(root.join("docs/TRACEABILITY.md")).unwrap().contains("| AB-2 second thing | **none** |"));
    std::fs::write(root.join("tests/b.rs"), "// REQ AB-2\n").unwrap();
    assert_eq!(run(&root, &cfg), 0);
    assert!(std::fs::read_to_string(root.join("docs/TRACEABILITY.md")).unwrap().contains("| AB-2 second thing | b.rs |"));
}

// REQ PC-14
#[test]
fn a_missing_requirements_file_is_exit_2_not_a_pass() {
    let (root, cfg) = project(TABLE, &[]);
    std::fs::remove_file(root.join("docs/REQUIREMENTS.md")).unwrap();
    assert_eq!(run(&root, &cfg), 2);
}

// REQ PC-14
#[test]
fn a_source_can_be_labelled_and_redirected_by_an_environment_variable() {
    let (root, mut cfg) = project(TABLE, &[("tests/a.rs", "// REQ AB-1\n"), ("elsewhere/w.rs", "// REQ AB-2\n")]);
    let var = format!("PC_TRACE_DIR_{}", std::process::id());
    // SAFETY: the variable name is unique to this process and only this test reads it.
    unsafe { std::env::set_var(&var, root.join("elsewhere")) };
    cfg.sources.push(Source { dir: "does-not-exist".into(), extensions: vec!["rs".into()], label: "web".into(), env: Some(var) });
    assert_eq!(run(&root, &cfg), 0);
    assert!(std::fs::read_to_string(root.join("docs/TRACEABILITY.md")).unwrap().contains("| AB-2 second thing | web:w.rs |"));
}
