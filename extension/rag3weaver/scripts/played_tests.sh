#!/bin/bash
# **The tests a pass played**, read from its journal (run_e2e.sh): zero tests is never a success. Without a filter,
# each suite asked must play at least one test; with a filter, the pass must play at least one. Gives 1 and says so
# when the rule is broken, 0 otherwise.
#
# Usage: scripts/played_tests.sh <journal> [<filter>]
set -u
journal="$1"
filter="${2:-}"
# **The binary's own safety nets do not count** (`Running tests/e2e_filets.rs`): they play first in every pass, two
# tests, whatever the filter; counted, they made a filter that names nothing look like a pass (11 November 2026).
results=$(awk '/^ *Running / { nets = ($0 ~ /e2e_filets\.rs/) } /^test result:/ && !nets' "$journal")
vides=$(printf '%s\n' "$results" | grep -c '^test result: .* 0 passed; 0 failed;' || true)
total=0
# The tests played, passed or not: a filter that names a failing test names something.
for n in $(printf '%s\n' "$results" | grep -oP '\d+(?= (passed|failed);)'); do total=$((total + n)); done
if [ -z "$filter" ] && [ "${vides:-0}" -gt 0 ]; then
  echo "✗ ${vides} suite(s) n'ont joué aucun test : feature manquante, ou fichier sans test ?"
  exit 1
fi
if [ -n "$filter" ] && [ "${total:-0}" -eq 0 ]; then
  echo "✗ le filtre « $filter » n'a désigné aucun test"
  exit 1
fi
exit 0
