#!/bin/bash
# reconstruire.sh <base.rag3db> <dossier du relevé> <table>… (14 nov. 2026, hotfixs) — reconstruit des tables de
# relations dont les deux rangements divergent : DROP, CREATE au même schéma (types et défauts de table_info, bouts
# de relation:<T> dans _catalog_meta), COPY de <table>-final.tsv (l'accord des deux sens, plus les lectures
# plausibles ; final.py), CHECKPOINT, puis le contrôle des deux sens de G sur toute la base.
# Sur une COPIE, ou une base fermée dont on a l'instantané. ECRIRE, REQUETE et CONTROLE bâtis contre la lib de la base.
set -u
B=${1:?base}; W=${2:?dossier}; shift 2
: "${ECRIRE:?}" "${REQUETE:?}" "${CONTROLE:?}"
H=$(cd "$(dirname "$0")" && pwd)
ordres=()
for t in "$@"; do
  "$REQUETE" "$B" "CALL table_info('$t') RETURN *" > "$W/$t-types.txt" 2>/dev/null
  grep -q '^> ' "$W/$t-types.txt" && ! grep -q '^ERREUR' "$W/$t-types.txt" || { echo "ROUGE : table_info $t"; exit 1; }
  bouts=$("$REQUETE" "$B" "MATCH (m:_catalog_meta {_key: 'relation:$t'}) RETURN m._value" 2>/dev/null | grep -v '^>' | grep . | python3 -I -c "import json,sys; d=json.loads(sys.stdin.read()); print(d['from'], d['to'])")
  read -r F T <<< "$bouts"; [ -n "$F" ] && [ -n "$T" ] || { echo "ROUGE : bouts de $t"; exit 1; }
  python3 -I - "$W" "$t" <<'PY'
import sys
D, t = sys.argv[1], sys.argv[2]
props = next(l.rstrip('\n').split('\t')[1:] for l in open(f"{D}/{t}.tsv") if l.startswith('# props'))
types = {c[1]: c[2] for c in (l.strip().split('|') for l in open(f"{D}/{t}-types.txt")) if len(c) >= 3 and c[0].isdigit()}
NUL = "__NUL_RECONSTRUCTION__"
cell = lambda v, ty: NUL if v == '\\N' else (v if ty == 'INT64' else '"' + v.replace('"', '""') + '"')
with open(f"{D}/{t}-final.csv", "w") as out:
    for l in open(f"{D}/{t}-final.tsv"):
        c = l.rstrip('\n').split('\t')
        out.write(','.join([cell(c[0], 'STRING'), cell(c[1], 'STRING')] + [cell(v, types[p]) for v, p in zip(c[2:], props)]) + '\n')
PY
  props=$(grep '^# props' "$W/$t.tsv" | cut -f2- | tr '\t' ',')
  cols=$(python3 -I -c "
import sys
out=[]
for l in open('$W/$t-types.txt'):
    c=l.strip().split('|')
    if len(c)>=4 and c[0].isdigit(): out.append(c[1]+' '+c[2]+('' if c[3]=='NULL' else ' DEFAULT '+c[3]))
print(', '.join(out))")
  ordres+=("DROP TABLE $t" "CREATE REL TABLE $t(FROM $F TO $T${cols:+, $cols})"
    "COPY $t${props:+ ($props)} FROM '$W/$t-final.csv' (from='$F', to='$T', escaped_newlines=true, auto_detect=false, null_strings=['__NUL_RECONSTRUCTION__'])")
  echo "$t : $(wc -l < "$W/$t-final.tsv") arêtes à reposer, de $F vers $T"
done
ordres+=("CHECKPOINT")
"$ECRIRE" "$B" "${ordres[@]}" > "$W/reconstruction.txt" 2>&1 || { grep -v VECTOR "$W/reconstruction.txt" | tail -5; echo "ROUGE : un ordre a échoué"; exit 1; }
grep -E 'tuples have been copied' "$W/reconstruction.txt"
"$CONTROLE" "$B" > "$W/controle-final.log" 2>&1; code=$?
grep -E 'ÉCART|BASE ATTEINTE|Aucun écart' "$W/controle-final.log"
for t in "$@"; do grep -E "^$t :" "$W/controle-final.log"; done
[ $code = 0 ] && echo VERT || echo "ROUGE : le contrôle des deux sens rend $code"
