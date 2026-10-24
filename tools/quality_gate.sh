#!/usr/bin/env bash
# Quality gate of the program itself: tests, traceability, and the hand-made mutation check.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -f ~/.cargo/env ] && source ~/.cargo/env
step() { printf '\n== %s\n' "$1"; }
step "1/3 cargo test"
cargo test --quiet
step "2/3 requirement traceability"
python3 tools/check_traceability.py
git diff --quiet -- docs/TRACEABILITY.md || { echo "docs/TRACEABILITY.md changed: commit the regenerated matrix"; exit 1; }
step "3/3 mutation check (every planted fault must make the tests fail)"
python3 -I mutants.py | tee /tmp/pipeline-check-mutants.txt | grep -E "^(killed|SURVIVED|COMPILE-ERR)" | awk -F' [|] ' '{print $1}' | sort | uniq -c
! grep -q "^SURVIVED" /tmp/pipeline-check-mutants.txt || { echo "a planted fault survived: a test is missing"; exit 1; }
printf '\nquality gate passed\n'
