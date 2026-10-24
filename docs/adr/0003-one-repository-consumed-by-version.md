# ADR-0003: Keep the program in one repository and consume it by pinned version

- **Status:** accepted
- **Date:** 2026-10-24
- **Type (ISO/IEC/IEEE 14764):** preventive
- **Decided by:** Giovanni (owner) / proposed by Claude Code

## Context
The first rollout copied the whole crate into five repositories (QuizApp, project-template, wiki, QuizWeb,
workflow-memory). Within hours six defects had been fixed in one copy; the others would have needed the same fixes by hand.

**Why it was not built this way from the start (root cause):**
1. The brief said "project-template: include the program", and the template's own convention is copy-on-start
   (`cp -r project-template NewProject; rm -rf .git`). Following the brief literally turned a tool into a template file.
2. The design was written around "first QuizApp, then copy to the others"; its acceptance criteria tested the program,
   never how it would be distributed, and neither the design review nor the builder check asked "who maintains N copies?".
3. A shared home needs CI in private repositories to fetch another repository, which the default CI token cannot do;
   that constraint was not examined, so copying looked like the only way that worked.
4. Nothing in the standing rules said where a tool shared by several projects should live.

## Decision
The program lives only here. A consumer project pins a git tag in `pipeline-check.version`, and its
`tools/pipeline-check.sh` (about 15 lines, the one thing still copied) clones that tag and builds it once per version.
This repository is public so that CI needs no secret (Giovanni's decision, 2026-10-24); it contains no private data.
Rule: a tool used by more than one project gets its own repository and is consumed by version, never copied.

## Alternatives considered
- **Private repository plus an access token in every consumer** — manual setup in six places and a token that expires.
- **Git submodule** — needs the same access in CI and makes every clone fiddlier.
- **Keep the copies** — the defect that triggered this ADR.

## Consequences
One fix, one tag, then each project upgrades on its own schedule after its gate passes. A consumer can fall behind
(visible in `pipeline-check.version`). The runner script is still duplicated; it is small and stable.
