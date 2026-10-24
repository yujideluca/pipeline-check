# ADR-0002: Let a program measure and block, and a skill judge and propose

- **Status:** accepted
- **Date:** 2026-10-24
- **Type (ISO/IEC/IEEE 14764):** preventive
- **Decided by:** Giovanni (owner) / proposed by Claude Code

## Context
The quality gate was strong only because Claude remembered to run it. Mature teams make the machine refuse the merge.
The existing checks were scattered (a fitness script, a metrics script, the gate script) and the architecture check
only searched the Gradle file, not the source.

## Decision
Two pieces that talk only through `report.json`: this program (measures, blocks, deterministic, runs in CI without
Claude) and the skill `revisao-manutencao` (reads the report, never re-measures, proposes `debt` issues with priority
P1-P3 and a Preventive decision-log row). The skill proposes; Giovanni decides.

## Alternatives considered
- **Extend the Python scripts of each project** — Giovanni chose Rust for the program.
- **Let the skill measure** — judgement and measurement mix, and nothing blocks when Claude forgets.

## Consequences
CI builds a Rust program on each run (cached). Limits change only by Giovanni's decision. Blocking a merge also needs
branch protection, or on a private free-plan repository the local pre-push check (`consumer/pre-push`).
