#!/bin/bash
# Témoin de scripts/poste, le verrou du poste, sur des verrous d'essai
# (POSTE_LOCK_DIR) : jamais les verrous réels.
# Usage : scripts/test_poste.sh [chemin/vers/poste]
# Rend 0 si tout tient, 1 sinon ; chaque cas écrit OK ou ÉCHEC.
#
# Ticket : docs/tickets/2026-10-11-une-mesure-attend-sans-fin-derriere-des-lourds-qui-entrent-quand-meme.md
set -u
POSTE="${1:-$(cd "$(dirname "$0")" && pwd)/poste}"
ICI="$(mktemp -d "${TMPDIR:-/tmp}/test-poste.XXXXXX")"
trap 'rm -rf "$ICI"' EXIT
export POSTE_LOCK_DIR="$ICI/verrous" POSTE_PUSH_DIR="$ICI/push" POSTE_PUSH_HOTE=
mkdir -p "$POSTE_LOCK_DIR" "$POSTE_PUSH_DIR"
# L'ancien « entre quand même » au bout d'une seconde, au lieu de 600 : sans la
# priorité de la mesure, un lourd arrivé derrière elle passe devant.
export POSTE_PORTE_MAX=1
echec=0
cas() { if [ "$1" = 0 ]; then echo "OK     $2"; else echo "ÉCHEC  $2"; echec=1; fi; }
J="$ICI/journal"
# note <qui> <début|fin> : l'instant, en millisecondes.
note() { echo "$1 $2 $(date +%s%3N)" >> "$J"; }
export -f note 2>/dev/null
export J
instant() { awk -v q="$1" -v e="$2" '$1==q && $2==e {print $3}' "$J"; }
travail() { # <mode> <qui> <secondes>
  "$POSTE" "$1" bash -c "note $2 début; sleep $3; note $2 fin" > /dev/null 2>&1
}

# ── 1. La mesure passe avant les lourds arrivés derrière elle ──────────────
: > "$J"
travail lourd L1 3 &
sleep 0.5
travail mesure M 1 &
sleep 0.5
travail lourd L2 2 &
sleep 1.5
travail lourd L3 2 &
wait
dM=$(instant M début); fM=$(instant M fin); fL1=$(instant L1 fin); dL2=$(instant L2 début); dL3=$(instant L3 début)
echo "       (ms) fin L1 $fL1 · M $dM-$fM · début L2 $dL2 · début L3 $dL3"
[ -n "$dM" ] && [ "$dM" -ge "$fL1" ]
cas $? "la mesure attend la fin du lourd déjà dans la place"
[ -n "$fM" ] && [ -n "$dL2" ] && [ "$fM" -le "$dL2" ] && [ "$fM" -le "$dL3" ]
cas $? "dès qu'une mesure attend, les nouveaux lourds attendent derrière elle (plus de « entre quand même »)"

# ── 2. Le délai d'une mesure ne court qu'une fois la place prise ──────────
: > "$J"
travail lourd L1 3 &
sleep 0.5
POSTE_MESURE_MAX=2 "$POSTE" mesure bash -c 'sleep 1' > /dev/null 2>&1
rc=$?
wait
[ "$rc" = 0 ]
cas $? "une mesure qui a attendu plus que son délai court quand même (code $rc)"
POSTE_MESURE_MAX=1 "$POSTE" mesure bash -c 'sleep 4' > /dev/null 2>&1
rc=$?
[ "$rc" = 124 ]
cas $? "une mesure qui dépasse son délai une fois dans la place est arrêtée (code $rc, attendu 124)"

# ── 3. poste --qui dit qui tient et qui attend ────────────────────────────
: > "$J"
travail lourd L1 4 &
sleep 0.5
travail mesure M 1 &
sleep 1
qui=$("$POSTE" --qui 2>&1)
wait
echo "$qui" | sed 's/^/       /'
echo "$qui" | grep -q "tient.*lourd" && echo "$qui" | grep -q "attend.*mesure" && echo "$qui" | grep -q "depuis"
cas $? "poste --qui dit le lourd qui tient et la mesure qui attend, depuis quand"
[ -z "$(ls -A "$POSTE_LOCK_DIR/fiches" 2>/dev/null)" ]
cas $? "à la fin, aucune fiche ne reste"

# ── 4. Un poste sous un poste passe : il reconnaît son ancêtre tenant ─────
# (ticket 2026-10-12-un-poste-sous-un-poste-attend-sa-propre-place) : une
# mesure dont la commande appelle poste lourd (run_e2e.sh le fait) attendait
# sa propre place sans fin.
sous() { # <mode extérieur> <mode intérieur>
  timeout 20 "$POSTE" "$1" bash -c "\"$POSTE\" $2 bash -c 'echo dedans'" 2>/dev/null
}
[ "$(sous mesure lourd)" = dedans ]
cas $? "une mesure dont la commande appelle poste lourd finit"
[ "$(sous lourd lourd)" = dedans ]
cas $? "un lourd sous un lourd finit"
[ "$(sous lourd mesure)" = dedans ]
cas $? "une mesure sous un lourd finit (sans prendre l'exclusif qu'un ancêtre tient en partagé)"
[ "$(sous mesure mesure)" = dedans ]
cas $? "une mesure sous une mesure finit"
[ -z "$(ls -A "$POSTE_LOCK_DIR/fiches" 2>/dev/null)" ]
cas $? "à la fin, aucune fiche ne reste"

exit $echec
