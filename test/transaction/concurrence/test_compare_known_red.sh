#!/bin/bash
# Témoin de compare_known_red.cmake, sans bâtir le banc : un faux banc écrit un résultat gtest JSON, et le juge le compare.
# La ligne finale doit dire les cas longs non joués quand CONCURRENCE_LONG n'est pas posé (15 novembre 2026).
# Usage : bash test/transaction/concurrence/test_compare_known_red.sh [compare_known_red.cmake] — sortie 0 si tout est vert.
set -u
J=$(realpath "${1:-$(dirname "$0")/compare_known_red.cmake}")
D=$(mktemp -d -p "${TMPDIR:-$HOME/.cache}"); trap 'rm -rf "$D"' EXIT
cat > $D/banc <<'B'
#!/bin/bash
# le faux banc : A.ok vert, B.rouge rouge connu [check: x], et C.long vert s'il n'est pas filtré
out=""; filtre=""; for a in "$@"; do case "$a" in --gtest_output=json:*) out=${a#--gtest_output=json:} ;; --gtest_filter=*) filtre=${a#--gtest_filter=} ;; esac; done
long=',{"name":"long","result":"COMPLETED"}'; case "$filtre" in *C.long*) long="" ;; esac
printf '{"testsuites":[{"name":"A","testsuite":[{"name":"ok","result":"COMPLETED"}]},{"name":"B","testsuite":[{"name":"rouge","result":"COMPLETED","failures":[{"failure":"x [check: x]"}]}]},{"name":"C","testsuite":[{"name":"autre","result":"COMPLETED"}%s]}]}' "$long" > "$out"
B
chmod +x $D/banc; printf 'B.rouge x\n' > $D/known_red.txt; : > $D/probabilistic.txt; printf 'C.long\n' > $D/long.txt
juge() { cmake -DBENCH=$D/banc -DKNOWN_RED_FILE=$D/known_red.txt -DPROBABILISTIC_FILE=$D/probabilistic.txt -DLONG_FILE=$D/long.txt -DRESULT_FILE=$D/resultat.json -P "$J" 2>&1; }
rouge=0
sortie=$(env -u CONCURRENCE_LONG bash -c "$(declare -f juge); D=$D; J=$J; juge"); rc=$?
[ $rc = 0 ] && echo "$sortie" | grep -q 'bench matches' && echo "$sortie" | grep -q 'INCOMPLETE: 1 long cases not run.*C.long' \
  && echo "vert   sans CONCURRENCE_LONG, la ligne finale dit le cas long non joué" || { echo "ROUGE  sans CONCURRENCE_LONG : rc $rc — $(echo "$sortie" | tail -1)"; rouge=1; }
sortie=$(CONCURRENCE_LONG=1 bash -c "$(declare -f juge); D=$D; J=$J; juge"); rc=$?
[ $rc = 0 ] && echo "$sortie" | grep -q 'bench matches' && ! echo "$sortie" | grep -q 'INCOMPLETE' \
  && echo "vert   avec CONCURRENCE_LONG=1, le cas long joue et la ligne finale ne dit rien d'incomplet" || { echo "ROUGE  avec CONCURRENCE_LONG=1 : rc $rc — $(echo "$sortie" | tail -1)"; rouge=1; }
exit $rouge
