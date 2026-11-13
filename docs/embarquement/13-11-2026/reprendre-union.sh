#!/bin/bash
# APRÈS vraie acquisition lourd, continuer le budget hex7oup7 sans renouvellement.
set -euo pipefail
W=/home/lucied/git_workspaces/cx-activation-modeles
cd "$W/extension/rag3weaver"
restant=$(python3 - <<'PY'
import importlib.util,json,pathlib,time
spec=importlib.util.spec_from_file_location('confinement','scripts/confiner_validation.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
pid=m.poste_tenu(m.processus())
assert pid is not None,'poste réel absent'
fiche=pathlib.Path.home()/'.cache/rag3weaver-build/fiches'/str(pid)
assert fiche.read_text().startswith('tient lourd '),'poste lourd requis'
p=pathlib.Path.home()/'.cache/rag3weaver-build/validations/validation-hex7oup7'
a=json.loads((p/'autorite.json').read_text());r=json.loads((p/'recette.json').read_text())
assert a['etat']=='termine' and r['delai']==2400
restant=int(a['echeance']-time.monotonic())
assert restant>20,'échéance originale expirée : reprise refusée'
print(restant)
PY
)
echo "Union : $restant secondes restantes, échéance hex7oup7 conservée"
: "${ACTIVATION_UNION_SUFFIX:?nouveau suffixe de preuve requis}"
exec python3 scripts/confiner_validation.py lancer --mode lourd --delai "$restant" -- python3 "$W/docs/embarquement/13-11-2026/qualifier-union.py"
