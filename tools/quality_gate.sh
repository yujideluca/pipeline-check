#!/usr/bin/env bash
# Quality gate of the program itself: tests, traceability, and the hand-made mutation check.
set -euo pipefail
cd "$(dirname "$0")/.."
[ -f ~/.cargo/env ] && source ~/.cargo/env
step() { printf '\n== %s\n' "$1"; }
step "0/3 test scratch folders delete themselves"
# A test that builds temp_dir()/name-PID by hand leaves it behind for good (each run has a new PID,
# and /tmp is RAM). Integration tests take folders only from tests/common (Scratch, removed on drop);
# a source file whose unit tests use temp_dir() must also remove what it made.
stray=$(grep -rln 'temp_dir()' tests | grep -vx 'tests/common/mod.rs' || true)
for f in $(grep -rl 'temp_dir()' src || true); do grep -q 'remove_dir_all' "$f" || stray="$stray $f"; done
[ -z "${stray// /}" ] || { echo "hand-built temp folder (use tests/common Scratch, or remove it): $stray"; exit 1; }
step "1/3 cargo test"
cargo test --quiet
step "2/3 requirement traceability"
cargo run --quiet -- --traceability
git diff --quiet -- docs/TRACEABILITY.md || { echo "docs/TRACEABILITY.md changed: commit the regenerated matrix"; exit 1; }
step "3/3 mutation check (every planted fault must make the tests fail)"
python3 -I mutants.py | tee /tmp/pipeline-check-mutants.txt | grep -E "^(killed|SURVIVED|COMPILE-ERR)" | awk -F' [|] ' '{print $1}' | sort | uniq -c
! grep -q "^SURVIVED" /tmp/pipeline-check-mutants.txt || { echo "a planted fault survived: a test is missing"; exit 1; }
printf '\nquality gate passed\n'
