#!/bin/bash
# Témoin de scripts/poste, le verrou du poste, sur des verrous d'essai
# (POSTE_LOCK_DIR) : jamais les verrous réels.
# Usage : scripts/test_poste.sh [chemin/vers/poste]
# Rend 0 si tout tient, 1 sinon ; chaque cas écrit OK ou ÉCHEC.
#
# Ticket : docs/tickets/2026-10-11-une-mesure-attend-sans-fin-derriere-des-lourds-qui-entrent-quand-meme.md
#
# **Deux règles de ce témoin** (12 octobre 2026, après l'incident de 3 h : un
# motif de ce fichier a tué trois pushes de codeparsers, un de F et un de la
# recherche) :
# 1. il ne termine que ses propres pids — ceux qu'il a lancés (`$!`) et leurs
#    enfants (`ps --ppid`), relevés avant le signal ; jamais un motif ni un nom
#    sur tout le poste ;
# 2. il joue sur SES fichiers de verrou, dans un dossier à lui (POSTE_LOCK_DIR,
#    POSTE_PUSH_DIR), jamais sur les verrous réels de ~/.cache/rag3weaver-build ;
#    un témoin qui a besoin des verrous réels n'est pas un témoin.
set -u
POSTE="${1:-$(cd "$(dirname "$0")" && pwd)/poste}"
ICI="$(mktemp -d "${TMPDIR:-/tmp}/test-poste.XXXXXX")"
trap 'rm -rf "$ICI"' EXIT
export POSTE_LOCK_DIR="$ICI/verrous" POSTE_PUSH_DIR="$ICI/push" POSTE_PUSH_HOTE=
case "$(realpath -m "$POSTE_LOCK_DIR") $(realpath -m "$POSTE_PUSH_DIR")" in
  *"$HOME/.cache/rag3weaver-build "*|*"$HOME/.cache/rag3weaver-build") echo "refus : le témoin ne joue jamais sur les verrous réels" >&2; exit 2 ;;
esac
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

# ── 5. Le verrou de push : un seul tenant, et --qui le nomme ─────────────────
# (relevé du cœur, 12 octobre : un tenant tué par signal rendait le verrou
# pendant que sa commande poussait encore ; deux commandes de push en même
# temps, et --qui ne nommait que la seconde.)
: > "$J"
"$POSTE" push A -- bash -c 'note A début; sleep 4; note A fin' > /dev/null 2>&1 &
sleep 1
"$POSTE" push B -- bash -c 'note B début; sleep 1; note B fin' > /dev/null 2>&1 &
sleep 1
qui=$("$POSTE" push --qui 2>&1)
echo "$qui" | sed 's/^/       /'
echo "$qui" | grep -q "tenu — A " && ! echo "$qui" | grep -q " B "
cas $? "poste push --qui nomme le seul tenant (A), pas celui qui attend"
wait
[ "$(instant B début)" -ge "$(instant A fin)" ]
cas $? "deux poste push en même temps : un seul tenant à la fois"

: > "$J"
"$POSTE" push A -- bash -c 'note A début; sleep 4; note A fin' > /dev/null 2>&1 &
pa=$!
sleep 1
# Les processus du tenant de A, comme les arrête une session « par ses pids » :
# le script lancé ici ($pa) et, dans l'ancienne forme, son enfant flock qui
# tenait le verrou. Jamais la commande, et JAMAIS un motif sur tout le poste :
# le 12 octobre à 3 h, un motif « flock … master-push.lock » de ce témoin a tué
# trois pushes réels d'une autre session.
tenant="$pa $(ps -o pid=,comm= --ppid "$pa" | awk '$2=="flock" {print $1}')"
kill -TERM $tenant
cas $? "les processus du tenant de A, et eux seuls, signalés (pid $tenant)"
sleep 0.5
"$POSTE" push B -- bash -c 'note B début; note B fin' > /dev/null 2>&1 &
wait
[ -n "$(instant A fin)" ] && [ "$(instant B début)" -ge "$(instant A fin)" ]
cas $? "un tenant qui reçoit un signal garde le verrou jusqu'à la fin de sa commande"
[ ! -s "$POSTE_PUSH_DIR/master-push.tenant" ] && [ -z "$(ls -A "$POSTE_PUSH_DIR/tenants" 2>/dev/null)" ]
cas $? "à la fin, aucune fiche de push ne reste"

# Deux pushes en attente derrière un tenant qui finit : les deux survivent et
# passent, chacun son tour (la piste de l'orchestration pour le TERM de 3 h).
: > "$J"
"$POSTE" push A -- bash -c 'note A début; sleep 2; note A fin' > /dev/null 2>&1 &
pa=$!
sleep 0.5
"$POSTE" push B -- bash -c 'note B début; sleep 1; note B fin' > /dev/null 2>&1 &
pb=$!
"$POSTE" push C -- bash -c 'note C début; sleep 1; note C fin' > /dev/null 2>&1 &
pc=$!
wait $pa; ra=$?; wait $pb; rb=$?; wait $pc; rc=$?
[ "$ra$rb$rc" = 000 ] && [ -n "$(instant B fin)" ] && [ -n "$(instant C fin)" ]
cas $? "deux pushes en attente survivent à la fin du tenant et passent (codes $ra $rb $rc)"
dB=$(instant B début); fB=$(instant B fin); dC=$(instant C début); fC=$(instant C fin)
[ "$dB" -ge "$(instant A fin)" ] && [ "$dC" -ge "$(instant A fin)" ] && { [ "$dC" -ge "$fB" ] || [ "$dB" -ge "$fC" ]; }
cas $? "chacun son tour : jamais deux commandes de push en même temps"

# ── 6. Un poste qui attend s'annule par un signal, et ne lance rien ─────────
# (12 octobre, luciepc : deux bâtis annulés restaient bloqués dans flock, et un
# flock tué laissait le script continuer jusqu'à la commande.)
: > "$J"
travail mesure M 4 &
sleep 0.5
"$POSTE" lourd bash -c 'note L début' > /dev/null 2>&1 &
attente=$!
sleep 1
# Le flock de CE poste seulement, relevé avant le signal (jamais un motif sur
# tout le poste : les attentes réelles des autres sessions y sont aussi).
fl=$(ps -o pid=,comm= --ppid "$attente" | awk '$2=="flock" {print $1}')
kill -TERM "$attente"
d=$(date +%s); wait "$attente" 2>/dev/null; rc=$?; f=$(date +%s)
[ $((f - d)) -le 2 ]
cas $? "un poste qui attend sort au signal ($((f - d)) s, code $rc)"
[ -n "$fl" ] && kill -TERM $fl 2>/dev/null
wait
[ -z "$(instant L début)" ]
cas $? "un poste annulé ne lance jamais sa commande, même si son flock meurt"
[ -z "$(ls -A "$POSTE_LOCK_DIR/fiches" 2>/dev/null)" ]
cas $? "à la fin, aucune fiche ne reste"

exit $echec
