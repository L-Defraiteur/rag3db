#!/bin/bash
# Witness of scripts/played_tests.sh (ticket docs/recherche/tickets/2026-11-11-un-filtre-e2e-vide-peut-donner-un-code-de-sortie-vert.md):
# synthetic journals of run_e2e.sh, the verdict each must give. Prints VERT or ROUGE and exits 0 or 1.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
failures=0
expect() { # <name> <expected exit> <journal> [filter]
  local name="$1" want="$2" journal="$3" filter="${4:-}" got
  bash "$HERE/played_tests.sh" "$journal" "$filter" > "$T/out" 2>&1; got=$?
  if [ "$got" = "$want" ]; then echo "ok    $name"; else echo "ROUGE $name: exit $got, expected $want — $(cat "$T/out")"; failures=$((failures + 1)); fi
}
# The binary's own safety nets play first, in every pass (« Running tests/e2e_filets.rs »): two tests.
filets='     Running tests/e2e_filets.rs (binaire e2e unique, ses propres filets)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 943 filtered out; finished in 0.06s'
printf '%s\n     Running tests/e2e_lectures.rs (binaire e2e unique, filtre e2e_lectures::)\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 945 filtered out; finished in 0.01s\n' "$filets" > "$T/empty_filter.log"
expect "a filter that names no test of the suite is refused, whatever the nets played (the ticket's case)" 1 "$T/empty_filter.log" qu_on_laisse_tomber
printf '%s\n     Running tests/e2e_lectures.rs (binaire e2e unique, filtre e2e_lectures::)\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 944 filtered out; finished in 0.20s\n' "$filets" > "$T/one.log"
expect "a filter that names one test passes" 0 "$T/one.log" a_real_test
printf '%s\n     Running tests/e2e_lectures.rs (binaire e2e unique, filtre e2e_lectures::)\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 944 filtered out; finished in 0.20s\n' "$filets" > "$T/failing.log"
expect "a filter that names a failing test names something" 0 "$T/failing.log" a_failing_test
expect "without a filter, a suite that played nothing is refused" 1 "$T/empty_filter.log"
printf '%s\n     Running tests/e2e_lectures.rs (binaire e2e unique, filtre e2e_lectures::)\ntest result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 920 filtered out; finished in 12s\n' "$filets" > "$T/full.log"
expect "without a filter, a suite that played its tests passes" 0 "$T/full.log"
if [ "$failures" -eq 0 ]; then echo VERT; exit 0; else echo ROUGE; exit 1; fi
