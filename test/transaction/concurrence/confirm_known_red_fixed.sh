#!/bin/bash
# Confirmer qu'un cas de known_red.txt est corrigé (18 novembre 2026, ticket du cœur) : le juge du banc ne garde aucun
# état d'un tour à l'autre, et un connu rouge peut passer une fois dans le banc entier par un effet d'ordre ou de graine.
# Ce script joue le cas SEUL, N fois, chacune dans un processus neuf, et ne demande le retrait de sa ligne que si les N
# passages sont verts.
# Usage : confirm_known_red_fixed.sh <binaire du banc> <nom gtest du cas> [N=3]
# Sortie 0 et « fixed: … » si N/N verts ; 1 et « still red: … (k/N green) » sinon ; 2 si l'appel est mal formé.
set -u
BENCH=${1:?binaire du banc}; NAME=${2:?nom du cas}; N=${3:-3}
case "$N" in ''|*[!0-9]*|0) echo "N doit être un entier positif" >&2; exit 2 ;; esac
[ -x "$BENCH" ] || { echo "binaire absent ou non exécutable : $BENCH" >&2; exit 2; }
green=0
for i in $(seq 1 "$N"); do
  out=$("$BENCH" --gtest_filter="$NAME" 2>&1); rc=$?
  if [ $rc = 0 ] && echo "$out" | grep -q '\[  PASSED  \] 1 test'; then green=$((green + 1)); echo "run $i/$N : green"
  else echo "run $i/$N : red (rc $rc)"; fi
done
if [ "$green" = "$N" ]; then
  echo "fixed: remove $NAME from known_red.txt ($green/$N green, each run alone in a fresh process)"; exit 0
fi
echo "still red: $NAME ($green/$N green) — keep its line in known_red.txt"; exit 1
