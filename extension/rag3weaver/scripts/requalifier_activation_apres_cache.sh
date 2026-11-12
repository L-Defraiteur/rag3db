#!/bin/bash
# À appeler APRÈS acquisition réelle du poste ; conserver l'échéance originale.
# Ticket docs/embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md
set -euo pipefail
ICI="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ICI"
: "${ACTIVATION_PREUVE_ORIGINALE:?preuve originale requise}"
restant=$(python3 - "$ICI/scripts/confiner_validation.py" "$ACTIVATION_PREUVE_ORIGINALE" <<'PY'
import importlib.util,json,pathlib,sys,time
spec=importlib.util.spec_from_file_location('confinement',sys.argv[1])
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
if module.poste_tenu(module.processus()) is None:
    raise SystemExit('ROUGE : acquérir le vrai poste avant de calculer le restant')
preuve=pathlib.Path(sys.argv[2])
autorite=json.loads((preuve/'autorite.json').read_text())
recette=json.loads((preuve/'recette.json').read_text())
restant=int(autorite['echeance']-time.monotonic())
if autorite['etat']!='termine' or recette['delai']!=7200 or restant<=0:
    raise SystemExit('ROUGE : échéance originale expirée ou preuve inattendue, aucun reset')
print(restant)
PY
)
echo "Requalification après correction du cache : $restant s maximum, échéance originale conservée"
export RAG3WEAVER_VALIDATION_TIMEOUT="$restant"
exec bash "$ICI/scripts/qualifier_activation_modeles.sh" lib
