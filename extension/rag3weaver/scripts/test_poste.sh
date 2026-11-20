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
# Le refus d'une mesure derrière un lourd long (cas 43) est coupé pour les autres cas, dont les lourds n'ont pas de timeout.
export POSTE_MESURE_ATTENTE_MAX=0
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

# ── 7. Au plus N lourds à la fois, les suivants dans l'ordre d'arrivée ────────
# (7 novembre 2026, luciepc : sept passes tenaient la place partagée ensemble,
# et des témoins rougissaient sur « pas indexé en 60 s ».) Sans place réservée
# déclarée, les sessions ont les N places.
: > "$J"
echo 2 > "$POSTE_LOCK_DIR/lourds-max"
travail lourd L1 2 &
sleep 0.3
travail lourd L2 4 &
sleep 0.3
travail lourd L3 1 &
sleep 1
qui=$("$POSTE" --qui 2>&1)
wait
echo "$qui" | sed 's/^/       /'
dL2=$(instant L2 début); fL1=$(instant L1 fin); dL3=$(instant L3 début)
echo "       (ms) fin L1 $fL1 · début L2 $dL2 · début L3 $dL3"
[ -n "$dL2" ] && [ "$dL2" -lt "$fL1" ]
cas $? "deux places, aucune réservée : le deuxième lourd passe pendant que le premier tourne"
[ -n "$dL3" ] && [ "$dL3" -ge "$fL1" ] && [ "$dL3" -lt "$(instant L2 fin)" ]
cas $? "le troisième attend le premier qui finit, et prend sa place sans attendre le deuxième"
echo "$qui" | grep -q "lourds : 2/2 places tenues, en file dans l'ordre : pid [0-9]"
cas $? "poste --qui dit les places tenues et la file"

: > "$J"
echo 1 > "$POSTE_LOCK_DIR/lourds-max"
travail lourd L1 2 &
sleep 0.3
travail lourd L2 1 &
sleep 0.3
travail lourd L3 1 &
wait
[ "$(instant L2 début)" -ge "$(instant L1 fin)" ] && [ "$(instant L3 début)" -ge "$(instant L2 fin)" ]
cas $? "une place : les lourds passent un par un, dans l'ordre d'arrivée"

: > "$J"
echo 1 > "$POSTE_LOCK_DIR/lourds-max"
travail lourd L1 3 &
sleep 0.5
POSTE_PUSH_LOURD_MAX=1 "$POSTE" push P --lourd -- bash -c 'note P début' > /dev/null 2>&1
rc=$?
wait
[ "$rc" = 75 ] && [ -z "$(instant P début)" ]
cas $? "un push lourd compte parmi les places : places prises, il rend 75 sans rien lancer (code $rc)"
# La voie du pousseur (19 novembre 2026, c7 et G : sept lourds en file devant sa passe, que tout le monde attend pour
# atterrir) : un lourd sous POSTE_VOIE=pousseur prend la tête de la file des lourds ; une mesure en attente reste devant.
: > "$J"
echo 1 > "$POSTE_LOCK_DIR/lourds-max"
travail lourd L1 2 &
sleep 0.3
travail lourd L2 1 &
sleep 0.3
POSTE_VOIE=pousseur travail lourd P 1 &
wait
[ -n "$(instant P début)" ] && [ "$(instant P début)" -ge "$(instant L1 fin)" ] && [ "$(instant L2 début)" -ge "$(instant P fin)" ]
cas $? "la voie du pousseur : arrivé après L2, il passe avant lui, à la première place libre"
: > "$J"
travail lourd L1 2 &
sleep 0.3
travail mesure M 1 &
sleep 0.3
POSTE_VOIE=pousseur travail lourd P 1 &
wait
[ -n "$(instant P début)" ] && [ "$(instant P début)" -ge "$(instant M fin)" ]
cas $? "la voie du pousseur reste derrière une mesure en attente"
# La place réservée (19 novembre 2026, c7 : quatre mesures longues de sessions ont figé la file une heure) : sur un
# poste qui la déclare (place-pousseur), à partir de deux places, les sessions n'en tiennent que N-1 ; la dernière reste
# au pousseur, qui part sans attendre.
: > "$J"
echo 2 > "$POSTE_LOCK_DIR/lourds-max"; touch "$POSTE_LOCK_DIR/place-pousseur"
travail lourd L1 3 &
sleep 0.3
travail lourd L2 1 &
sleep 0.3
POSTE_VOIE=pousseur travail lourd P 1 &
wait
[ -n "$(instant P début)" ] && [ "$(instant P début)" -lt "$(instant L1 fin)" ] && [ "$(instant L2 début)" -ge "$(instant L1 fin)" ]
cas $? "deux places, une réservée : une session en tient une, la seconde attend, le pousseur part à la place réservée"
rm -f "$POSTE_LOCK_DIR/place-pousseur" "$POSTE_LOCK_DIR/lourds-max"
[ -z "$(ls -A "$POSTE_LOCK_DIR/fiches" 2>/dev/null)" ] && [ -z "$(ls -A "$POSTE_LOCK_DIR/file" 2>/dev/null)" ]
cas $? "à la fin, aucune fiche ni aucun billet ne reste"

# ── 8. L'enveloppe CPU : quatre lourds qui saturent ne dépassent pas les cœurs utiles ──
# (14 novembre 2026, luciepc à 33 de charge sur 24 cœurs.) Pour rester léger, le témoin ne laisse que quatre
# cœurs utiles (coeurs-libres = cœurs − 4) et quatre places : un cœur par lourd. Chaque lourd lance huit
# boucles actives pendant 4 s ; on lit le temps CPU de sa portée (cpu.stat) : la somme des quatre ne doit pas
# dépasser 4 cœurs × 4 s, à 15 % près. Les nombres de fils dits aux outils valent 1.
if systemd-run --user --scope -q -- true 2>/dev/null; then
  n=$(nproc --all)
  if [ "$n" -ge 6 ]; then
    echo 4 > "$POSTE_LOCK_DIR/lourds-max"; echo $((n - 4)) > "$POSTE_LOCK_DIR/coeurs-libres"
    sature='g=$(cut -d: -f3 /proc/self/cgroup); echo "$POSTE_FILS $RAYON_NUM_THREADS $CARGO_BUILD_JOBS $RUST_TEST_THREADS" > "$1.fils";
      for i in 1 2 3 4 5 6 7 8; do timeout 4 sh -c "while :; do :; done" & done; wait;
      awk "/^usage_usec/{print \$2}" "/sys/fs/cgroup$g/cpu.stat" > "$1.cpu"'
    for k in 1 2 3 4; do "$POSTE" lourd bash -c "$sature" _ "$ICI/s$k" > /dev/null 2>&1 & done
    wait
    total=0; for k in 1 2 3 4; do total=$(( total + $(cat "$ICI/s$k.cpu" 2>/dev/null || echo 999999999) )); done
    [ "$total" -le $(( 4 * 4 * 1150000 )) ]
    cas $? "quatre lourds saturants : $((total / 1000)) ms de CPU en 4 s, au plus $((4 * 4 * 1150)) (4 cœurs utiles)"
    [ "$(cat "$ICI/s1.fils")" = "1 1 1 1" ]
    cas $? "les fils dits aux outils valent la part d'un lourd (lu : $(cat "$ICI/s1.fils"))"
    rm -f "$POSTE_LOCK_DIR/lourds-max" "$POSTE_LOCK_DIR/coeurs-libres"
  else
    echo "SAUTÉ  l'enveloppe CPU : moins de 6 cœurs ici"
  fi
else
  echo "SAUTÉ  l'enveloppe CPU : systemd-run --user indisponible"
fi

# ── 9. Sous un quota étranger, les parts se calculent sur les cœurs du poste ──────────────
# (14 novembre 2026 : dans un lourd de 500 % à luciepc, `nproc` rendait 5 et le cas 8 sautait.) Un poste lancé
# dans une portée à quota (CPUQuota=200 %, un `systemd-run` qui n'est pas un poste) doit donner à un lourd
# (cœurs du poste − 2) / 4, pas (2 − 2) / 4.
if systemd-run --user --scope -q -- true 2>/dev/null && [ "$(nproc --all)" -ge 10 ]; then
  echo 4 > "$POSTE_LOCK_DIR/lourds-max"; echo 2 > "$POSTE_LOCK_DIR/coeurs-libres"
  attendu=$(( ($(nproc --all) - 2) / 4 ))
  lu=$(systemd-run --user --scope -q -p CPUQuota=200% -E POSTE_LOCK_DIR="$POSTE_LOCK_DIR" -- "$POSTE" lourd sh -c 'echo $POSTE_FILS' 2>/dev/null | tail -1)
  [ "$lu" = "$attendu" ]
  cas $? "sous un quota étranger de 200 %, un lourd reçoit $attendu fils (lu : $lu)"
  rm -f "$POSTE_LOCK_DIR/lourds-max" "$POSTE_LOCK_DIR/coeurs-libres"
else
  echo "SAUTÉ  le quota étranger : systemd-run --user indisponible ou moins de 10 cœurs"
fi

# **Une validation au-dessus du plafond du confineur est refusée avant la file** (15 novembre 2026, ticket d'hotfixs) :
# sous le défaut de 40G, une passe attendait sa place (5 min 23 s à luciepc), puis le confineur la refusait en 0,18 s
# (« aucun plafond effectif <= 32 Gio »). Le défaut ne change pas pour les autres lourds (mot de c7).
V="$ICI/validation"; mkdir -p "$V"
for e in run_e2e.sh passe_epinglee.sh temoins_navigateur.sh; do printf '#!/bin/sh\ntouch "%s/joue-$(basename "$0")"\n' "$V" > "$V/$e"; chmod +x "$V/$e"; done
sortie=$("$POSTE" lourd bash "$V/passe_epinglee.sh" 2>&1); rc=$?
[ $rc = 2 ] && [ ! -e "$V/joue-passe_epinglee.sh" ] && echo "$sortie" | grep -q 'POSTE_MEM_MAX=32G'
cas $? "sous le défaut (40G), une passe épinglée est refusée avant la file, sans jouer, et le refus dit POSTE_MEM_MAX=32G (code $rc)"
sortie=$(POSTE_MEM_MAX=48G "$POSTE" lourd bash -c "cd $V && ./run_e2e.sh --test e2e_x" 2>&1); rc=$?
[ $rc = 2 ] && [ ! -e "$V/joue-run_e2e.sh" ]
cas $? "un run_e2e.sh nommé dans un bash -c, sous 48G, est refusé lui aussi (code $rc)"
POSTE_MEM_MAX=32G "$POSTE" lourd "$V/temoins_navigateur.sh" > /dev/null 2>&1; rc=$?
[ $rc = 0 ] && [ -e "$V/joue-temoins_navigateur.sh" ]
cas $? "sous POSTE_MEM_MAX=32G, la validation joue (code $rc)"
"$POSTE" lourd sh -c "touch $V/joue-autre" > /dev/null 2>&1; rc=$?
[ $rc = 0 ] && [ -e "$V/joue-autre" ]
cas $? "un lourd qui n'est pas une validation garde le défaut de 40G et joue (code $rc)"
# Le confineur se lit à côté du témoin, dans le dépôt, et non à côté du `poste` jugé : la copie du cache n'a pas de
# voisin (16 novembre 2026, ticket de G).
seuil_poste=$(sed -n 's/^VALIDATION_MEMORY_MAX=\([0-9]*\).*/\1/p' "$POSTE")
seuil_confineur=$(python3 -c "import re,sys; m=re.search(r'^MAX_MEMOIRE = (\d+) \* 1024\*\*3', open(sys.argv[1]).read(), re.M); print(int(m.group(1))*1024**3)" "$(dirname "$(realpath "$0")")/confiner_validation.py" 2>/dev/null)
[ -n "$seuil_poste" ] && [ "$seuil_poste" = "$seuil_confineur" ]
cas $? "le seuil de poste est celui du confineur (poste ${seuil_poste:-absent}, confiner_validation.py ${seuil_confineur:-absent})"
# sccache et mold (19 novembre 2026, c7 ; posés par Lucie aux deux postes) : sous `poste`, une commande lie par mold
# quand il est là, et compile par sccache (redevenu le défaut le 20 novembre 2026 : le bac à sable du produit retire
# RUSTC_WRAPPER et SCCACHE_*, lot bac-sans-sccache d'archi) ; POSTE_SCCACHE=0 le coupe ; un choix de l'appelant l'emporte.
R=$(mktemp -d -p "${TMPDIR:-$HOME/.cache}"); mkdir -p "$R/bin"
for b in sccache mold clang; do printf '#!/bin/sh\nexit 0\n' > "$R/bin/$b"; chmod +x "$R/bin/$b"; done
# Un script, pas un `sh -c` : systemd-run développe lui-même les ${…} d'une ligne de commande.
printf '%s\n' '#!/bin/sh' 'echo "${RUSTC_WRAPPER-absent}|${SCCACHE_DIR-absent}|${SCCACHE_CACHE_SIZE-absent}|${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER-absent}|${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS-absent}" > "$1"' > "$R/lire.sh"
env -u RUSTC_WRAPPER -u SCCACHE_DIR -u SCCACHE_CACHE_SIZE -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS -u POSTE_SCCACHE \
  PATH="$R/bin:$PATH" "$POSTE" lourd sh "$R/lire.sh" "$R/env" > /dev/null 2>&1
[ "$(cat "$R/env" 2>/dev/null)" = "sccache|$HOME/.cache/sccache|40G|clang|-C link-arg=-fuse-ld=mold" ]
cas $? "sous poste, sans POSTE_SCCACHE : sccache par défaut (~/.cache/sccache, 40G) et mold par clang (lu : $(cat "$R/env" 2>/dev/null))"
env -u RUSTC_WRAPPER -u SCCACHE_DIR -u SCCACHE_CACHE_SIZE -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS \
  POSTE_SCCACHE=0 PATH="$R/bin:$PATH" "$POSTE" lourd sh "$R/lire.sh" "$R/env0" > /dev/null 2>&1
[ "$(cat "$R/env0" 2>/dev/null)" = "absent|absent|absent|clang|-C link-arg=-fuse-ld=mold" ]
cas $? "sous POSTE_SCCACHE=0 : ni RUSTC_WRAPPER ni SCCACHE_*, mold reste (lu : $(cat "$R/env0" 2>/dev/null))"
env -u RUSTC_WRAPPER -u SCCACHE_DIR -u SCCACHE_CACHE_SIZE -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS \
  POSTE_SCCACHE=1 PATH="$R/bin:$PATH" "$POSTE" lourd sh "$R/lire.sh" "$R/env1" > /dev/null 2>&1
[ "$(cat "$R/env1" 2>/dev/null)" = "sccache|$HOME/.cache/sccache|40G|clang|-C link-arg=-fuse-ld=mold" ]
cas $? "sous POSTE_SCCACHE=1, sccache (~/.cache/sccache, 40G) et mold (lu : $(cat "$R/env1" 2>/dev/null))"
env -u SCCACHE_DIR -u SCCACHE_CACHE_SIZE -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS \
  RUSTC_WRAPPER= POSTE_SCCACHE=1 PATH="$R/bin:$PATH" "$POSTE" lourd sh "$R/lire.sh" "$R/env2" > /dev/null 2>&1
[ "$(cut -d'|' -f1 "$R/env2" 2>/dev/null)" = "" ]
cas $? "RUSTC_WRAPPER posé vide par l'appelant reste vide (lu : $(cat "$R/env2" 2>/dev/null))"
env -u RUSTC_WRAPPER -u SCCACHE_DIR -u SCCACHE_CACHE_SIZE -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER -u CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS \
  PATH="/usr/bin:/bin" POSTE_SANS_OUTILS_TEMOIN=1 "$POSTE" lourd sh "$R/lire.sh" "$R/env3" > /dev/null 2>&1
if command -v sccache > /dev/null 2>&1 && [ "$(command -v sccache)" != "$R/bin/sccache" ]; then cas 0 "(sccache installé ici : le cas sans outils ne se juge pas)"
else [ "$(cut -d'|' -f1 "$R/env3" 2>/dev/null)" = absent ]; cas $? "sans sccache ni mold, rien n'est posé (lu : $(cat "$R/env3" 2>/dev/null))"; fi
rm -rf "$R"
# ── 43. Une mesure refuse à l'arrivée derrière un lourd long (20 novembre 2026, c7 : deux gels de la file en deux heures
# à luciepc, une mesure tenant la porte derrière un lourd de six heures). La fin prévue d'un lourd est le moment où il a
# pris sa place plus le `timeout N` en tête de sa commande ; sans timeout, elle est inconnue, donc longue. Au-delà de
# POSTE_MESURE_ATTENTE_MAX (1200 s par défaut), la mesure refuse tout de suite, code 76, en nommant le lourd.
: > "$J"
travail lourd LONG 4 &
sleep 0.5
debut=$(date +%s); sortie=$(POSTE_MESURE_ATTENTE_MAX=1200 "$POSTE" mesure bash -c "note M début" 2>&1); rc=$?; duree=$(( $(date +%s) - debut ))
wait
[ "$rc" = 76 ] && [ "$duree" -le 2 ] && [ -z "$(instant M début)" ] && echo "$sortie" | grep -q "rejouer sous poste lourd"
cas $? "une mesure derrière un lourd sans timeout refuse tout de suite, code 76 (rc $rc, ${duree} s)"
: > "$J"
"$POSTE" lourd timeout 30 bash -c "note C début; sleep 2; note C fin" > /dev/null 2>&1 &
sleep 0.5
POSTE_MESURE_ATTENTE_MAX=1200 "$POSTE" mesure bash -c "note M début" > /dev/null 2>&1; rc=$?
wait
[ "$rc" = 0 ] && [ -n "$(instant M début)" ] && [ "$(instant M début)" -ge "$(instant C fin)" ]
cas $? "une mesure derrière un lourd court (timeout 30) attend sa fin et joue (rc $rc)"

# ── 44. Une baisse des places porte son heure de retour (20 novembre 2026, c7 : la tablette est restée à une place de
# 07 h 30 à 13 h 35, la fenêtre du matin finie à 08 h 51, six lourds derrière). `poste places <N> jusqu'à HH:MM` écrit
# lourds-max et lourds-max.retour ; la valeur d'avant revient à la première lecture passé l'heure ; --qui le dit tant
# que ça dure ; une baisse sans heure refuse.
echo 2 > "$POSTE_LOCK_DIR/lourds-max"; rm -f "$POSTE_LOCK_DIR/lourds-max.retour"
"$POSTE" places 1 > /dev/null 2>&1; rc=$?
[ "$rc" = 2 ] && [ "$(cat "$POSTE_LOCK_DIR/lourds-max")" = 2 ]
cas $? "une baisse des places sans heure de retour refuse, lourds-max inchangé (rc $rc)"
h=$(date -d '+2 hours' +%H:%M)
"$POSTE" places 1 "jusqu'à" "$h" > /dev/null 2>&1; rc=$?
qui=$("$POSTE" --qui 2>&1)
[ "$rc" = 0 ] && [ "$(cat "$POSTE_LOCK_DIR/lourds-max")" = 1 ] && echo "$qui" | grep -q "places réglées à 1 jusqu'au $(date -d "$h" +%d/%m) $h (puis 2)" && echo "$qui" | grep -q '/1 places'
cas $? "poste places 1 jusqu'à $h baisse à 1, et --qui le dit (rc $rc)"
# L'heure passée : la valeur d'avant revient à la première lecture.
read -r avant _ < "$POSTE_LOCK_DIR/lourds-max.retour"; echo "$avant $(( $(date +%s) - 60 ))" > "$POSTE_LOCK_DIR/lourds-max.retour"
qui=$("$POSTE" --qui 2>&1)
[ "$(cat "$POSTE_LOCK_DIR/lourds-max")" = 2 ] && [ ! -e "$POSTE_LOCK_DIR/lourds-max.retour" ] && ! echo "$qui" | grep -q 'réglées' && echo "$qui" | grep -q '/2 places'
cas $? "passé l'heure, les places reviennent à la valeur d'avant (2) et --qui ne dit plus de baisse"
rm -f "$POSTE_LOCK_DIR/lourds-max" "$POSTE_LOCK_DIR/lourds-max.retour"

# ── 45. Le poste autorise l'iGPU à l'embarqueur par un fichier déclaré (20 novembre 2026, c7 et embarquement : l'APU
# Radeon 8060S de la tablette calcule juste, l'Intel ARL de luciepc non). `embedder-allow-integrated-gpu` dans le dossier
# des verrous, contenu = la référence de la mesure : poste lourd et poste mesure exportent alors
# RAG3WEAVER_EMBEDDER_ALLOW_INTEGRATED_GPU=1 ; sans lui, rien.
unset RAG3WEAVER_EMBEDDER_ALLOW_INTEGRATED_GPU
rm -f "$POSTE_LOCK_DIR/embedder-allow-integrated-gpu"
sans=$("$POSTE" lourd bash -c 'echo "${RAG3WEAVER_EMBEDDER_ALLOW_INTEGRATED_GPU:-absent}"' 2>/dev/null)
echo "embarquement 08979a5b37 : APU Radeon 8060S, cos >= 0,99999991 contre card2" > "$POSTE_LOCK_DIR/embedder-allow-integrated-gpu"
lourd=$("$POSTE" lourd bash -c 'echo "${RAG3WEAVER_EMBEDDER_ALLOW_INTEGRATED_GPU:-absent}"' 2>/dev/null)
mesure=$("$POSTE" mesure bash -c 'echo "${RAG3WEAVER_EMBEDDER_ALLOW_INTEGRATED_GPU:-absent}"' 2>/dev/null)
rm -f "$POSTE_LOCK_DIR/embedder-allow-integrated-gpu"
[ "$sans" = absent ] && [ "$lourd" = 1 ] && [ "$mesure" = 1 ]
cas $? "le fichier embedder-allow-integrated-gpu autorise l'iGPU sous poste lourd et mesure, et seulement lui (sans : $sans, lourd : $lourd, mesure : $mesure)"

# ── 48. Une hausse datée des places (20 novembre 2026, c7 et Lucie : la tablette à trois lourds jusqu'au lundi 8 h, le
# retour posé à la main faute d'outil, et --qui disait « baissées ») : `poste places 3 jusqu'à <date HH:MM>` monte, la
# valeur d'avant revient passé l'heure, et --qui dit « réglées à N jusqu'au jj/mm HH:MM (puis M) ». Une hausse sans heure
# reste permanente.
echo 2 > "$POSTE_LOCK_DIR/lourds-max"; rm -f "$POSTE_LOCK_DIR/lourds-max.retour"
quand=$(date -d '+3 days' '+%Y-%m-%d 08:00')
"$POSTE" places 3 "jusqu'à" "$quand" > /dev/null 2>&1; rc=$?
qui=$("$POSTE" --qui 2>&1)
[ "$rc" = 0 ] && [ "$(cat "$POSTE_LOCK_DIR/lourds-max")" = 3 ] && [ -s "$POSTE_LOCK_DIR/lourds-max.retour" ] \
  && echo "$qui" | grep -q "places réglées à 3 jusqu'au $(date -d "$quand" '+%d/%m %H:%M') (puis 2)"
cas $? "poste places 3 jusqu'à $quand monte à 3 jusqu'à la date, et --qui le dit (rc $rc ; $(echo "$qui" | grep -m1 'places' ))"
"$POSTE" places 4 > /dev/null 2>&1; rc=$?
[ "$rc" = 0 ] && [ "$(cat "$POSTE_LOCK_DIR/lourds-max")" = 4 ] && [ ! -e "$POSTE_LOCK_DIR/lourds-max.retour" ]
cas $? "une hausse sans heure reste permanente et efface un retour en cours (rc $rc)"
rm -f "$POSTE_LOCK_DIR/lourds-max" "$POSTE_LOCK_DIR/lourds-max.retour"

exit $echec
