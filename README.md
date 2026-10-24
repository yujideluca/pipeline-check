# pipeline-check

A small Rust program that measures a software project against its limits and writes `report.json`. It exits 1 when
a check is `falha` (failure), so a CI job built on it fails by itself; a warning (`aviso`) never fails.
It measures; it never judges. Judging (priorities, proposals) is done by a separate Claude skill that reads the report.

Checks: the project's own gate runs once; an architecture rule read from the source (no `android.*`/`androidx.*`
import in the pure module); fitness functions with a limit each (warning from 80 %, failure at 100 %); DORA figures
from git, a releases repository and the decision log; and, in local mode only, whether the last CI run is green and
branch protection requires the CI job (read-only, through `gh`). A check that could not run is `pulado`, never `ok`.

## Use it in a project

1. Copy `consumer/pipeline-check.sh` to `tools/pipeline-check.sh` and pin a version: `echo v0.1.0 > pipeline-check.version`.
2. Write `pipeline-check.toml` (see `docs/pipeline-check.example.toml`).
3. In CI: set up Rust (`dtolnay/rust-toolchain`) and run `tools/pipeline-check.sh --mode ci`.
4. Optional local stand-in for branch protection: copy `consumer/pre-push` to `tools/hooks/pre-push`, set
   `[pre_push] guard = ["path/prefix/"]` in `pipeline-check.toml` (empty: every push of main is checked) and run
   `git config core.hooksPath tools/hooks`. The logic lives in the program (`--pre-push`), so an upgrade fixes every project.

The script clones this repository at the pinned tag and builds it once per version. To upgrade, change the tag in
`pipeline-check.version` after reading the release notes in `DESIGN.md`.

## Develop it

`tools/quality_gate.sh` (tests, traceability, mutation check). `PIPELINE_CHECK_DIR=$PWD` makes a consumer project use
this working copy instead of its pinned tag. Requirements are in `docs/REQUIREMENTS.md`; tests carry `REQ <ID>` tags.
