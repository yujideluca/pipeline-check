# Requirements

Every promise the program makes, with an ID. A test that checks it carries the tag `REQ <ID>` in a comment;
`pipeline-check --traceability` fails when a requirement has no test or a tag names an unknown requirement.
Source: G = stated by Giovanni, D = derived by Claude Code and accepted.

| ID | Requirement | Source |
|---|---|---|
| PC-1 | `pipeline-check` writes `report.json` with `schema_version`, project, date, mode (`local` or `ci`), the list of checks (id, status `ok`/`aviso`/`falha`/`pulado`, value, limit) and the `dora` block. The exit code is non-zero if and only if some check is `falha`; an `aviso` never fails. | G |
| PC-2 | A fitness function is `ok` below the warning share of its limit (default 80 %, set per project), `aviso` from there up to the limit, and `falha` at 100 % of the limit or more. | G |
| PC-3 | The architecture check fails when any Kotlin or Java source in the pure module imports `android.*` or `androidx.*`, or its build file names Android, even if the compiler would not notice. | G |
| PC-4 | A check that cannot run in the current mode or environment is reported `pulado`, never `ok`; a `pulado` check that the config marks required is `falha`. The report states the mode that ran. | D |
| PC-5 | The project gate runs once; a non-zero exit is `falha`, and its duration feeds the gate-time fitness function. | D |
| PC-6 | DORA figures come from git, the release feed and the decision log, are `null` when there is no data, and never fail the run. | D |
| PC-7 | The GitHub checks (last CI run green, branch protection requires the CI job) only read, never change anything; an unavailable `gh` or no network is `pulado`, a missing protection is `falha`. | G |
| PC-8 | A missing or invalid limits file stops the program with exit code 2 and a clear message; it never falls back to silent defaults. | D |
| PC-9 | The architecture check also catches `import static android...` (Java), fully qualified uses such as `android.util.Log.d(...)` without an import, and ignores `android` inside comments and inside a longer name (`myandroid.Foo`, `dev.x.android.Foo`). | D |
| PC-10 | Walking a source directory terminates and finishes even when it contains a symbolic link that points back up (no infinite recursion). | D |
| PC-11 | A cancelled or skipped CI run is neither green nor red: "last CI run is green" looks at the most recent run that succeeded or failed. | D |
| PC-12 | With `--no-github` the program runs the gate and the local checks but never asks GitHub (the local pre-push check uses it). | D |
| PC-13 | `pipeline-check --pre-push` is a git pre-push hook: it runs the full local checks (without asking GitHub) before main is pushed and exits 1 to block the push when a check fails. Pushes of other branches, deletions and pushes that change only paths outside `[pre_push] guard` (path prefixes; empty means all) are not checked; a new main or a remote commit unknown locally counts every file as changed. | D |
| PC-14 | `pipeline-check --traceability` reads the requirements table (`| XX-1 | text |` rows), collects the `REQ <ID>[, <ID>]` tags that comment lines carry in the configured test folders, and exits 1 when a requirement has no tagged test or a tag names an unknown requirement. It writes the matrix (requirement to test files, labelled by source) in every case; a missing requirements file or config section is exit 2, never a silent pass. This replaces the `check_traceability.py` that each project used to copy. | D |
| PC-15 | With a `[shellcheck]` section the program lints every `git ls-files '*.sh'` plus the listed extra files at the configured severity (default `warning`): a clean run is `ok`, a finding is `falha` and names the first one, no shell scripts is `ok`, and an absent `shellcheck` or a folder that is not a git repository is `pulado`, never `ok`. It also runs under `--measure-only`, which every project gate calls. | D |
| PC-16 | With `[github] protection_exception = "<reason>"` a branch that lacks the required protection (no protection at all, or the CI job not required) is reported `aviso` instead of `falha`, and the detail states the reason; a protected branch stays `ok` and an unreadable protection stays `pulado`, so the exception never turns a gap into `ok`. An empty reason is a configuration error (exit 2). | G |
