#!/bin/bash
# Témoin de check_rel_directions (14 novembre 2026, relevé d'hotfixs) : une relation lisible par l'ancrage d'un seul
# sens, invisible à la lecture non ancrée, doit se voir « d'un seul côté ». Le cas réel : la table LOCKS de
# code-principal à luciepc, instantané `avant-373b5477b` (lockIndexForWrite → acquireLock, ligne 1173). L'outil
# d'avant, qui tirait ses bouts d'une seule lecture non ancrée, y voit 0 relation d'un seul côté ; l'outil ancré en voit.
# Le sens de chaque tranche est prouvé par son plan (`CHECK_REL_PLANS=1`) : SCAN_NODE_TABLE de la source puis
# `Direction: (a)-[r]->(b)` au sens avant, de la destination puis `Direction: (b)<-[r]-(a)` au sens arrière.
#
# **Il se lance par son nom, et seulement ainsi** (c7, 14 novembre) : il lui faut la base réelle, qui n'existe qu'à
# luciepc. Il ne fait partie d'aucun jeu qui tourne sans elle (CI, check_features, run_e2e.sh), et ne doit pas y entrer.
#
# Usage : test_check_rel_directions.sh <source .cpp> <dossier de la lib (rag3db.hpp, librag3db.so)> <copie jetable de la base>
# La base est une copie reflink, jamais l'instantané ni une base en service (l'outil l'ouvre en lecture seule). Sortie 0
# si tout est vert ; une base absente est un ROUGE (« non joué »), jamais un vert.
set -u
SRC=$(realpath "${1:?source}"); LIB=${2:?lib}; BASE=${3:?base}
[ -e "$BASE" ] || { echo "ROUGE  non joué : pas de base $BASE"; exit 1; }
D=$(mktemp -d); trap 'rm -rf "$D"' EXIT
nice -n 15 g++ -std=c++20 -O2 -I "$LIB" "$SRC" -L "$LIB" -lrag3db -Wl,-rpath,"$LIB" -o "$D/outil" || { echo "ROUGE  bâti"; exit 1; }
CHECK_REL_PLANS=1 "$D/outil" "$BASE" LOCKS > "$D/sortie" 2>&1; rc=$?
rouge=0
vert() { echo "vert   $1"; }
faux() { echo "ROUGE  $1"; rouge=1; }
[ $rc = 1 ] && grep -q '^LOCKS : .* — ÉCART' "$D/sortie" && vert "LOCKS rend un écart (code 1)" || faux "LOCKS : code $rc, $(grep '^LOCKS' "$D/sortie")"
n=$(grep -oE '^    [0-9]+ relations d.un seul côté' "$D/sortie" | grep -oE '[0-9]+')
[ "${n:-0}" -ge 1 ] && vert "$n relations d'un seul côté, que la lecture non ancrée perdait" || faux "aucune relation d'un seul côté : les bouts ne sont pas ancrés"
grep -qE '^    comptes : avant [0-9]+, arrière [0-9]+, union [0-9]+, perdues d.un seul sens [0-9]+$' "$D/sortie" && vert "les quatre comptes : $(grep -oE 'avant [0-9]+, arrière [0-9]+, union [0-9]+, perdues.*' "$D/sortie")" || faux "pas de ligne de comptes"
# Le sens de chaque tranche, par son plan : la table de nœuds parcourue, puis la direction de la relation.
avant=$(awk '/^    plan avant depuis /{p=1;next} /^    plan /{p=0} p' "$D/sortie")
arriere=$(awk '/^    plan arrière depuis /{p=1;next} /^    plan /{p=0} p' "$D/sortie")
echo "$avant" | grep -q 'Direction: (a)-\[r\]->(b)' && echo "$avant" | grep -q 'SCAN_NODE_TABLE' && vert "le plan d'une tranche avant : SCAN_NODE_TABLE puis (a)-[r]->(b)" || faux "plan avant sans (a)-[r]->(b)"
echo "$arriere" | grep -q 'Direction: (b)<-\[r\]-(a)' && echo "$arriere" | grep -q 'SCAN_NODE_TABLE' && vert "le plan d'une tranche arrière : SCAN_NODE_TABLE puis (b)<-[r]-(a)" || faux "plan arrière sans (b)<-[r]-(a)"
exit $rouge
