#!/bin/bash
# Durée de vie de l'état possédé et garde réel de run_e2e.sh, sans Cargo,
# moteur, modèle ni serveur. Le chemin fourni en option permet la revue d'un
# autre lanceur ; les fonctions testées sont ses vraies définitions.
set -euo pipefail
ICI=$(cd "$(dirname "$0")/.." && pwd)
SCRIPT=${1:-$ICI/run_e2e.sh}
T=$(mktemp -d "${TMPDIR:-/tmp}/test-etat-suite-e2e.XXXXXX")
trap 'unset -f rm 2>/dev/null || true; command rm -rf -- "$T"' EXIT
for nom in creer_etat_suite_e2e terminer_etat_suite_e2e residus_de_la_suite; do
  definition=$(sed -n "/^$nom() {/,/^}/p" "$SCRIPT")
  [ -n "$definition" ] || { echo "ROUGE : fonction $nom absente"; exit 1; }
  eval "$definition"
done
E2E_TMP_PARENT=$T/tmp
LISTE_DES_RESIDUS=$T/residus-connus
TEST_FILTER=''
: > "$LISTE_DES_RESIDUS"
mkdir -p "$E2E_TMP_PARENT/e2e_demon_hote" "$T/disque"
root="$E2E_TMP_PARENT/e2e_demon_hote"

etat=$(creer_etat_suite_e2e "$root")
printf 'livre-a-preserver\n' > "$etat/livre.json"
terminer_etat_suite_e2e "$etat" 0 '' "$T/disque/non-utilise"
[ ! -e "$etat" ]
if ! residus_de_la_suite e2e_demon_hote; then echo 'ROUGE : état nettoyé refusé'; exit 1; fi
printf 'OK état possédé seul : nettoyage et garde vert\n'

etat=$(creer_etat_suite_e2e "$root")
touch "$root/non-possede"
terminer_etat_suite_e2e "$etat" 0 '' "$T/disque/non-utilise"
if residus_de_la_suite e2e_demon_hote; then echo 'ROUGE : résidu arbitraire masqué'; exit 1; fi
[ -f "$root/non-possede" ]
command rm "$root/non-possede"
printf 'OK résidu non possédé : garde rouge conservé\n'

for statut in 101 124; do
  etat=$(creer_etat_suite_e2e "$root")
  printf 'preuve-%s\n' "$statut" > "$etat/livre.json"
  terminer_etat_suite_e2e "$etat" "$statut" '' "$T/disque/non-utilise"
  grep -qx "preuve-$statut" "$etat/livre.json"
  if residus_de_la_suite e2e_demon_hote; then echo 'ROUGE : état échoué invisible'; exit 1; fi
  command rm -rf "$etat"
done
printf 'OK échec et expiration : état original conservé\n'

etat=$(creer_etat_suite_e2e "$root")
printf 'octets-demandes\n' > "$etat/livre.json"
terminer_etat_suite_e2e "$etat" 0 1 "$T/disque/etat-garde"
[ ! -e "$etat" ] && grep -qx 'octets-demandes' "$T/disque/etat-garde/livre.json"
if ! residus_de_la_suite e2e_demon_hote; then echo 'ROUGE : état gardé refusé'; exit 1; fi
printf 'OK GARDER=1 : octets conservés, TMPDIR propre\n'

etat=$(creer_etat_suite_e2e "$root")
rm() { return 1; }
if terminer_etat_suite_e2e "$etat" 0 '' "$T/disque/non-utilise"; then echo 'ROUGE : échec nettoyage ignoré'; exit 1; fi
unset -f rm
if residus_de_la_suite e2e_demon_hote; then echo 'ROUGE : nettoyage échoué masqué'; exit 1; fi
command rm -rf "$etat"
printf 'OK défaut de nettoyage : reste rouge\n'

mkdir "$T/etat-non-possede"
printf 'ne-pas-effacer\n' > "$T/etat-non-possede/livre.json"
if terminer_etat_suite_e2e "$T/etat-non-possede" 0 '' "$T/disque/non-utilise"; then echo 'ROUGE : chemin non possédé accepté'; exit 1; fi
grep -qx 'ne-pas-effacer' "$T/etat-non-possede/livre.json"
printf 'OK chemin non possédé : effacement refusé\n'
printf 'ETAT_SUITE_E2E VERT\n'
