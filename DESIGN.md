# pipeline-check: design

Last updated: 2026-10-24

## 1. What it is
A Rust program that measures a project (gate, architecture rule, fitness functions, DORA figures, read-only GitHub
checks) and writes `report.json`; see `README.md`. Consumers: QuizApp, QuizWeb, wiki, workflow-memory, project-template.

## 2. Structure
`src/lib.rs` runs the checks in order (gate, architecture, fitness, GitHub, DORA) and builds the `Report`;
`config.rs` reads `pipeline-check.toml`; `report.rs` holds the status model and `classify`; `arch.rs`, `fitness.rs`,
`gate.rs`, `dora.rs`, `github.rs` are the checks (GitHub behind a trait so tests need no network); `fsutil.rs` has the
file walk and date helpers; `main.rs` is the command line. `consumer/` holds what a project copies: the runner
script and the optional pre-push hook.

## 3. Release notes
| Tag | Notes |
|---|---|
| v0.1.0 | First release: PC-1..PC-12 (everything from QuizApp's `tools/pipeline-check`, the six defects fixed, `--no-github`, `--version`). Its copied hook had a defect: a first push of main was never checked. |
| v0.2.0 | PC-13: the pre-push check moved into the program (`--pre-push`, `[pre_push] guard`), tested against real git repositories; the copied hook is now a three-line wrapper. Upgrade: change `pipeline-check.version`, replace `tools/hooks/pre-push` by `consumer/pre-push`, add `[pre_push]` to the toml. |
| v0.3.1 | PC-14: the requirement traceability check moved into the program (`--traceability`, `[traceability]` with `[[traceability.sources]]`), replacing the `tools/check_traceability.py` copied in four repositories. Upgrade: change `pipeline-check.version`, add `[traceability]` to the toml (folders, extensions, label/env as the old script had them), replace `python3 tools/check_traceability.py` in the gate by `tools/pipeline-check.sh --traceability`, delete the script, commit the regenerated matrix. |
| v0.4.0 | PC-15: `[shellcheck]` lints every tracked `*.sh` (and listed extra files) through the check that every gate already runs (`--measure-only`); `pulado` when shellcheck is not installed. Upgrade: change `pipeline-check.version`, add `[shellcheck]` to the toml (extra_files = ["tools/hooks/pre-push"]). |

## 4. Decision log

| Date | Type | Decision | Why |
|---|---|---|---|
| 2026-10-24 | Corrective | **The pre-push logic moved from a copied shell hook into the program** (PC-13, v0.2.0). A test of the hook in a throwaway repository showed that the first push of main was never checked (`git diff` against a single commit compares the work tree) | The same lesson as ADR-0003: logic copied into six repositories cannot be fixed or tested once. Now it is tested against real git repositories |
| 2026-10-24 | Preventive | **One repository, consumed by pinned version** ([ADR-0003](docs/adr/0003-one-repository-consumed-by-version.md)); the repository is public so CI needs no token; `--no-github` for the local pre-push check; v0.1.0 | Five copies in five repositories were already drifting. Root cause and rule in the ADR |
| 2026-10-24 | Preventive | **The program measures, a skill judges** ([ADR-0002](docs/adr/0002-program-measures-skill-judges.md)); bug-finding tests found and fixed six defects (`docs/TESTES-AUTOMATIZADOS.md`) | The gate held only because Claude remembered to run it |
