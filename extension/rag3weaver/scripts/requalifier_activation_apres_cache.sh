#!/bin/bash
# À appeler APRÈS acquisition réelle du poste ; conserver l'échéance originale.
# Ticket docs/embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md
set -euo pipefail
ICI="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ICI"
: "${ACTIVATION_PREUVE_ORIGINALE:?preuve originale requise}"
mode=${1:-lib}
case "$mode" in
    lib) plafond=7200; verrou=lourd ;;
    demon) plafond=1800; verrou=mesure ;;
    spans) plafond=600; verrou=lourd ;;
    *) echo 'phase inconnue' >&2; exit 2 ;;
esac
if [ -n "${ACTIVATION_PHASE_PLAFOND:-}" ]; then
    [[ "$ACTIVATION_PHASE_PLAFOND" =~ ^[0-9]+$ ]] || exit 2
    [ "$ACTIVATION_PHASE_PLAFOND" -ge 3 ] && [ "$ACTIVATION_PHASE_PLAFOND" -le "$plafond" ] || exit 2
    plafond="$ACTIVATION_PHASE_PLAFOND"
fi
restant=$(python3 - "$ICI/scripts/confiner_validation.py" "$ACTIVATION_PREUVE_ORIGINALE" "$plafond" "$verrou" <<'PY'
import importlib.util,json,os,pathlib,sys,time
spec=importlib.util.spec_from_file_location('confinement',sys.argv[1])
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
poste=module.poste_tenu(module.processus())
if poste is None:
    raise SystemExit('ROUGE : acquérir le vrai poste avant de calculer le restant')
fiche=pathlib.Path(os.environ.get('POSTE_LOCK_DIR',str(pathlib.Path.home()/'.cache/rag3weaver-build')))/'fiches'/str(poste)
if not fiche.read_text().startswith('tient '+sys.argv[4]+' '):
    raise SystemExit('ROUGE : mode réel du poste différent de la phase')
preuve=pathlib.Path(sys.argv[2])
autorite=json.loads((preuve/'autorite.json').read_text())
recette=json.loads((preuve/'recette.json').read_text())
restant=min(int(sys.argv[3]),int(autorite['echeance']-time.monotonic()))
if autorite['etat']!='termine' or recette['delai'] not in (7200,1800,600) or restant<3:
    raise SystemExit('ROUGE : échéance originale expirée ou preuve inattendue, aucun reset')
print(restant)
PY
)
echo "Phase $mode : $restant s maximum, échéance originale conservée"
export RAG3WEAVER_VALIDATION_TIMEOUT="$restant"
if [ "$mode" = lib ]; then exec bash "$ICI/scripts/qualifier_activation_modeles.sh" lib; fi
exec python3 "$ICI/scripts/confiner_validation.py" lancer --mode "$verrou" --delai "$restant" -- bash "$ICI/scripts/qualifier_activation_e2e.sh" "$mode"
