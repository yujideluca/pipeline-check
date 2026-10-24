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
| v0.1.0 | First release: PC-1..PC-12 (everything from QuizApp's `tools/pipeline-check`, the six defects fixed, `--no-github`, `--version`). |

## 4. Decision log

| Date | Type | Decision | Why |
|---|---|---|---|
| 2026-10-24 | Preventive | **One repository, consumed by pinned version** ([ADR-0003](docs/adr/0003-one-repository-consumed-by-version.md)); the repository is public so CI needs no token; `--no-github` for the local pre-push check; v0.1.0 | Five copies in five repositories were already drifting. Root cause and rule in the ADR |
| 2026-10-24 | Preventive | **The program measures, a skill judges** ([ADR-0002](docs/adr/0002-program-measures-skill-judges.md)); bug-finding tests found and fixed six defects (`docs/TESTES-AUTOMATIZADOS.md`) | The gate held only because Claude remembered to run it |
