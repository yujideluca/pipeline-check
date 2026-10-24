# pipeline-check: working notes for Claude Code

The one program that measures every software project of Giovanni's (QuizApp, QuizWeb, wiki, workflow-memory,
project-template). Rust, Giovanni's choice (work.ask-language). Read `README.md` first.

## Principles (each one is checked by something)

1. **Requirement, then test, then code**: `docs/REQUIREMENTS.md` → test tagged `REQ <ID>` → code. Checked by `tools/check_traceability.py`.
2. **The program measures, a skill judges** (ADR-0002): it never decides priorities; checked by the report having only measurements.
3. **A tool used by more than one project lives here once and is consumed by pinned version, never copied** (ADR-0003).
4. **The gate decides**: `tools/quality_gate.sh` (tests, traceability, hand-made mutation check), run by CI on every push.
5. **Decisions are written where the code is**: decision-log row with its Type in `DESIGN.md`; an ADR for structural ones.

## Working rules

- Build and test: `tools/quality_gate.sh`. After pushing, the CI run must be green.
- A change to behaviour is released as a new git tag (`vX.Y.Z`); projects upgrade by editing their `pipeline-check.version`
  after the gate of that project passes with `PIPELINE_CHECK_DIR`. Never move or delete a tag a project uses.
- After coding anything, do a first-line debug pass: gate, then run it against a real project (QuizApp).
- This repository is public: no secrets, no personal or clinical data, nothing about other people.
- Bug-finding tests: `docs/TESTES-AUTOMATIZADOS.md` (skill `gerar-teste-automatizado`, update mode).
- At the end of every task update the wiki (`wiki publish`), then `mem use`. Never force-push. Ask before anything outward-facing.
