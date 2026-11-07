//! Requirement traceability (PC-14): every requirement in the requirements table has a test that carries its tag,
//! and every tag names a requirement that exists. Writes the matrix so it is always current.
use crate::config::{Source, Traceability};
use crate::fsutil;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub struct Analysis {
    pub reqs: Vec<(String, String)>,
    /// requirement id -> test files (labelled) that carry its tag, sorted
    pub found: BTreeMap<String, Vec<String>>,
    pub unknown: Vec<String>,
    /// requirement cells and `REQ` tags shaped like an id that is not one (PC-17)
    pub malformed: Vec<String>,
}

impl Analysis {
    pub fn missing(&self) -> Vec<&str> {
        self.reqs.iter().filter(|(i, _)| self.found[i].is_empty()).map(|(i, _)| i.as_str()).collect()
    }
    pub fn ok(&self) -> bool {
        self.missing().is_empty() && self.unknown.is_empty() && self.malformed.is_empty()
    }
}

const ID_FORMAT: &str = "is not an id (two capital letters, a dash and a number, like AN-29)";

/// Length of the id `XX-<digits>` at the start of `s`, if there is one.
fn is_id_at(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    if b.len() < 4 || !b[0].is_ascii_uppercase() || !b[1].is_ascii_uppercase() || b[2] != b'-' {
        return None;
    }
    let digits = b[3..].iter().take_while(|c| c.is_ascii_digit()).count();
    (digits > 0).then_some(3 + digits)
}

/// `s` is exactly one id.
pub fn is_id(s: &str) -> bool {
    is_id_at(s) == Some(s.len())
}

/// PC-17: `s` looks like an attempt at an id (a capital, then capitals or digits, then a dash) but is not exactly
/// `XX-<digits>`. Such an id used to be skipped without a word, so its requirement and tests were never checked.
pub fn malformed_id(s: &str) -> bool {
    let b = s.as_bytes();
    let head = b.iter().take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit()).count();
    let shaped = !b.is_empty() && b[0].is_ascii_uppercase() && b.get(head) == Some(&b'-');
    shaped && !is_id(s)
}

/// A valid or malformed id, in any letter case (`ab-1` is a mistyped id, `tags` is a word).
fn id_shaped(w: &str) -> bool {
    let u = w.to_ascii_uppercase();
    is_id(&u) || malformed_id(&u)
}

/// The cells of a table row: split at `|` that is neither escaped (`\|`) nor inside a backtick code span.
fn cells(row: &str) -> Vec<String> {
    let (mut out, mut cur, mut code, mut prev) = (Vec::new(), String::new(), false, ' ');
    for c in row.trim().chars() {
        if c == '`' {
            code = !code;
        }
        if c == '|' && !code && prev != '\\' {
            out.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
        prev = c;
    }
    out.push(cur.trim().to_string());
    // a row starts and ends with `|`: drop the empty pieces outside them
    if out.first().is_some_and(String::is_empty) {
        out.remove(0);
    }
    if out.last().is_some_and(String::is_empty) {
        out.pop();
    }
    out
}

/// PC-18: every table row of the requirements file is the header (first cell `ID`), the separator, or a
/// requirement `| XX-1 | text | ... |`; anything else is a problem, never skipped. Returns (requirements, problems).
pub fn read_requirements(text: &str) -> (Vec<(String, String)>, Vec<String>) {
    let (mut reqs, mut problems) = (Vec::<(String, String)>::new(), Vec::new());
    let mut seen = BTreeMap::<String, usize>::new();
    for (n, line) in text.lines().enumerate().map(|(i, l)| (i + 1, l)) {
        if !line.trim_start().starts_with('|') {
            continue;
        }
        let c = cells(line);
        let first = c.first().map_or("", String::as_str);
        if first == "ID" || (!c.is_empty() && c.iter().all(|x| !x.is_empty() && x.trim_matches(':').chars().all(|ch| ch == '-'))) {
            continue;
        }
        if !is_id(first) {
            let what = if first.is_empty() { "an empty first cell" } else { first };
            problems.push(format!("requirements line {n}: {what} {ID_FORMAT}"));
            continue;
        }
        let body = c.get(1).map_or("", String::as_str);
        if body.is_empty() {
            problems.push(format!("requirements line {n}: requirement {first} has no text"));
            continue;
        }
        if let Some(m) = seen.get(first) {
            problems.push(format!("requirements line {n}: requirement {first} is listed twice (first on line {m})"));
            continue;
        }
        seen.insert(first.to_string(), n);
        reqs.push((first.to_string(), body.to_string()));
    }
    (reqs, problems)
}

/// Rows `| XX-1 | text |` of the requirements table.
pub fn parse_requirements(text: &str) -> Vec<(String, String)> {
    read_requirements(text).0
}

/// The ids named by tags on a line (see `scan_line`).
pub fn tags_in_line(line: &str, prefixes: &[String]) -> Vec<String> {
    scan_line(line, prefixes).0
}

fn word_at(line: &str, i: usize) -> bool {
    let before = line[..i].chars().next_back().is_none_or(|c| !c.is_alphanumeric() && c != '_');
    let after = line[i + 3..].chars().next().is_none_or(|c| !c.is_alphanumeric() && c != '_');
    before && after
}

/// PC-18: a tag is the word `REQ` followed by ids, where `REQ` is the first word of a comment (also a comment after
/// code: `#[test] // REQ AB-1`) or the first word of a string (a docstring or test name: `"""REQ AB-1: ..."""`).
/// Ids are separated by spaces, commas or semicolons, a repeated `REQ` continues the list, punctuation after an
/// id ends it, and the list ends at the first other word. Any other `REQ`/`req` followed by an id-shaped word,
/// outside a backtick code span and outside a string, is a problem: it reads like a tag but would not count.
/// Returns (ids, problems).
pub fn scan_line(line: &str, prefixes: &[String]) -> (Vec<String>, Vec<String>) {
    let (mut ids, mut problems) = (Vec::new(), Vec::new());
    let (mut in_str, mut code, mut esc, mut opened) = (false, false, false, usize::MAX);
    let mut skip_to = 0;
    for (i, c) in line.char_indices() {
        // `REQ` right after the quote that opened the string (`"REQ`, `"""REQ`), not after an escaped `\"`
        let quote_before = in_str && i == opened;
        let word = line.get(i..i + 3).filter(|w| w.eq_ignore_ascii_case("REQ"));
        if i >= skip_to && !code && word.is_some() && word_at(line, i) {
            let before = line[..i].trim_end();
            let opener = !in_str
                && prefixes.iter().any(|p| {
                    before.ends_with(p.as_str())
                        && before[..before.len() - p.len()].chars().next_back().is_none_or(char::is_whitespace)
                });
            let rest = &line[i + 3..];
            if &line[i..i + 3] == "REQ" && (opener || quote_before) && rest.starts_with(' ') {
                let (got, bad, end) = tag_list(&rest[1..]);
                if got.is_empty() && bad.is_empty() && rest.trim().is_empty() {
                    problems.push("REQ with no id".to_string());
                }
                ids.extend(got);
                problems.extend(bad.into_iter().map(|w| format!("REQ {w} {ID_FORMAT}")));
                skip_to = i + 4 + end;
            } else if (opener || quote_before) && rest.trim().is_empty() {
                problems.push("REQ with no id".to_string());
            } else if !in_str || quote_before {
                let next = rest.trim_start_matches([':', ' ', '\t']);
                let w: String = next.chars().take_while(|c| !c.is_whitespace() && !",;)\"".contains(*c)).collect();
                let w = w.trim_end_matches([':', '.']);
                if id_shaped(w) {
                    problems.push(format!(
                        "{} {w} is not read as a tag: write REQ as the first word of a comment or of a test name",
                        &line[i..i + 3]
                    ));
                }
            }
        }
        if i < skip_to {
            continue;
        }
        match c {
            '\\' if in_str => {
                esc = !esc;
                continue;
            }
            '"' if !code && !esc => {
                in_str = !in_str;
                if in_str {
                    opened = i + 1;
                }
            }
            '`' if !in_str => code = !code,
            _ => {}
        }
        esc = false;
    }
    (ids, problems)
}

/// Ids after `REQ `: (valid ids, malformed id-shaped words, byte length consumed).
fn tag_list(rest: &str) -> (Vec<String>, Vec<String>, usize) {
    let (mut ids, mut bad, mut end) = (Vec::new(), Vec::new(), 0);
    let mut pos = 0;
    for word in rest.split_inclusive([' ', ',', ';']) {
        let w = word.trim_end_matches([' ', ',', ';']);
        let start = pos;
        pos += word.len();
        if w.is_empty() {
            continue;
        }
        if w == "REQ" {
            end = pos;
            continue;
        }
        // punctuation right after an id ends it: `REQ AB-1: text`, `(REQ AB-1)`, `"""REQ AB-1."""`
        let core = w.trim_end_matches([':', '.', ')', '"', '\'', '`']);
        if is_id(core) {
            ids.push(core.to_string());
        } else if id_shaped(core) {
            bad.push(core.to_string());
        } else {
            return (ids, bad, start);
        }
        end = start + w.len();
        if core.len() != w.len() {
            return (ids, bad, end);
        }
    }
    (ids, bad, end)
}

/// `files`: (display name, content) of every test file.
pub fn analyze(requirements: &str, files: &[(String, String)], prefixes: &[String]) -> Analysis {
    let (reqs, mut malformed) = read_requirements(requirements);
    let mut found: BTreeMap<String, Vec<String>> = reqs.iter().map(|(i, _)| (i.clone(), Vec::new())).collect();
    let mut unknown = Vec::new();
    for (name, text) in files {
        for (n, line) in text.lines().enumerate().map(|(i, l)| (i + 1, l)) {
            let (tags, problems) = scan_line(line, prefixes);
            malformed.extend(problems.iter().map(|p| format!("{name}:{n}: {p}")));
            for tag in tags {
                match found.get_mut(&tag) {
                    None => unknown.push(format!("{name}:{n} names unknown requirement {tag}")),
                    Some(v) if !v.contains(name) => v.push(name.clone()),
                    Some(_) => {}
                }
            }
        }
    }
    for v in found.values_mut() {
        v.sort();
    }
    Analysis { reqs, found, unknown, malformed }
}

pub fn matrix(a: &Analysis) -> String {
    let mut out = String::from("# Traceability matrix\n\nGenerated by `pipeline-check --traceability`; do not edit.\n\n| Requirement | Verified by |\n|---|---|\n");
    for (i, t) in &a.reqs {
        let short: String = t.chars().take(70).collect();
        let dots = if t.chars().count() > 70 { "…" } else { "" };
        let by = if a.found[i].is_empty() { "**none**".to_string() } else { a.found[i].join(", ") };
        out.push_str(&format!("| {i} {short}{dots} | {by} |\n"));
    }
    out
}

/// A source folder: `~/x` is under the home folder, anything else is relative to the project root.
pub fn resolve(root: &Path, raw: &str, home: &str) -> PathBuf {
    match raw.strip_prefix("~/") {
        Some(rest) => Path::new(home).join(rest),
        None => root.join(raw),
    }
}

fn source_dir(root: &Path, s: &Source) -> PathBuf {
    let raw = s.env.as_ref().and_then(|v| std::env::var(v).ok()).unwrap_or_else(|| s.dir.clone());
    resolve(root, &raw, &std::env::var("HOME").unwrap_or_default())
}

/// Runs the check; prints the problems; returns the process exit code (0 ok, 1 gaps, 2 unreadable input).
pub fn run(root: &Path, cfg: &Traceability) -> i32 {
    let Ok(requirements) = std::fs::read_to_string(root.join(&cfg.requirements)) else {
        eprintln!("pipeline-check: cannot read {}", cfg.requirements);
        return 2;
    };
    let mut files = Vec::new();
    for s in &cfg.sources {
        let dir = source_dir(root, s);
        if !dir.is_dir() {
            // PC-18: a folder that is not there would be read as "no tags" without a word
            eprintln!("pipeline-check: traceability source folder {} does not exist", dir.display());
            return 2;
        }
        for path in fsutil::walk(&dir) {
            if !fsutil::has_ext(&path, &s.extensions) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let file = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            files.push((if s.label.is_empty() { file } else { format!("{}:{file}", s.label) }, text));
        }
    }
    let a = analyze(&requirements, &files, &cfg.comment_prefixes);
    if std::fs::write(root.join(&cfg.matrix), matrix(&a)).is_err() {
        eprintln!("pipeline-check: cannot write {}", cfg.matrix);
        return 2;
    }
    for m in a.missing() {
        println!("requirement {m} has no test");
    }
    for u in a.unknown.iter().chain(&a.malformed) {
        println!("{u}");
    }
    if !a.ok() {
        return 1;
    }
    println!("traceability ok: {} requirements, all covered by tagged tests", a.reqs.len());
    0
}
