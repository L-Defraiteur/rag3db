#!/bin/bash
# Appelé exclusivement APRÈS acquisition de poste lourd ; pas de délai remis à zéro.
set -euo pipefail
source_faits="$1"
recette=/home/lucied/.cache/rag3weaver-build/cx-reprise-faits-lies-v3.py
preuve=/home/lucied/.cache/rag3weaver-build/cx-passe-faits-lies-v3
unit=cx-passe-faits-lies-v3.scope
[ ! -e "$preuve" ] || { echo 'ARRÊT : preuve existante, aucun rejeu automatique'; exit 2; }
debut=$(python3 -c 'import json; print(json.load(open("/home/lucied/.cache/rag3weaver-build/cx-passe-faits-lies-v1/source.json"))["initial_monotonic"])')
reste=$(python3 -c 'import sys,time; print(max(0,int(float(sys.argv[1])+3600-time.monotonic()-10)))' "$debut")
[ "$reste" -gt 30 ] || { echo 'ARRÊT : fenêtre initiale épuisée'; exit 2; }
finir() {
  rc=$?
  trap - EXIT TERM INT HUP
  etat=$(systemctl --user show "$unit" --property=ActiveState --value 2>/dev/null || true)
  if [[ "$etat" = active || "$etat" = activating || "$etat" = deactivating ]]; then
    systemctl --user stop "$unit" || true
    [ ! -d "$preuve" ] || printf 'ROUGE\n' > "$preuve/VERDICT"
    rc=1
  fi
  if [ "$rc" = 0 ]; then
    python3 - "$preuve" "$source_faits" <<'PY' || rc=$?
import json,pathlib,shutil,sys
p=pathlib.Path(sys.argv[1]);source=sys.argv[2]
assert (p/'VERDICT').read_text().strip()=='VERT'
assert json.loads((p/'quiescence.json').read_text())['live']==[]
tmp=pathlib.Path(json.loads((p/'tmp-navigateur.json').read_text())['path'])
assert tmp.parent==pathlib.Path('/var/tmp') and tmp.name.startswith('cx-fa.')
assert (tmp/'.possede-faits-lies').read_text().strip()==source
shutil.rmtree(tmp)
(p/'tmp-retire.json').write_text(json.dumps(dict(path=str(tmp),absent=not tmp.exists(),source=source))+'\n')
PY
    if [ "$rc" != 0 ]; then
      printf 'ROUGE\n' > "$preuve/VERDICT"
      printf 'ROUGE : nettoyage TMP privé échoué\n' > "$preuve/erreur-nettoyage.txt"
    fi
  fi
  exit "$rc"
}
trap finir EXIT
trap 'exit 124' TERM INT HUP
# L'échéance du driver part de début ; timeout et scope ne peuvent la prolonger.
timeout --kill-after=5s "$reste" systemd-run --user --scope --unit="$unit" \
  -p BindsTo=cx-passe-faits-lies-v3.service -p After=cx-passe-faits-lies-v3.service \
  -p RuntimeMaxSec="$((reste-5))s" -p TimeoutStopSec=5s -p MemoryMax=32G -p MemoryHigh=infinity -p MemorySwapMax=4G \
  python3 "$recette" "$source_faits" "$debut"
