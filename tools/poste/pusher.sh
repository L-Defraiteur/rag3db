#!/bin/bash
# The pusher (lot 1, docs/optimiseur/18-11-2026/02-le-pousseur.md): sessions deposit their lot in a queue that lives
# outside the repository; the pusher takes one lot at a time, reposes it on the head of the moment in its single
# worktree, replays it by kind, pushes it, and writes « poussé » or « rouge » to its log and to the author's inbox.
# Lot 1 runs trains of one lot; the train, the bisection and the suspension under an old lib come in lots 2 and 3.
#
# Usage, at luciepc:
#   pusher.sh enqueue <id> <branch> <head> <kind> <witness> <witness_suite> <extra_suites> <parent> <author>
#             kind: docs, scripts or rust (cpp is refused: it lands by the transferable path or a window);
#             « - » for an empty field; extra_suites comma-separated
#   pusher.sh dequeue <id>
#   pusher.sh step          one lot from the head of the queue, then return
#   pusher.sh run           step in a loop (PUSHER_IDLE seconds between empty steps, 60 by default)
#
# The queue (queue.tsv), the log (log.tsv, never rewritten), the inboxes (inbox/<author>.txt) and the pause file
# (paused) live in PUSHER_DIR. Everything below can be replaced for the witness (tools/poste/test_pusher.sh).
set -u
CACHE=$HOME/.cache/rag3weaver-build
D=${PUSHER_DIR:-$CACHE/pusher}
WS=${PUSHER_WORKSPACES:-$HOME/git_workspaces}
WTN=${PUSHER_WT_NAME:-rag3db-pusher}
WT=$WS/$WTN
JR=${PUSHER_JOURNALS_ROOT:-$CACHE}
PASS=${PUSHER_PASS:-$CACHE/passe_epinglee.sh}
GUARD=${PUSHER_GUARD:-$CACHE/push_garde.sh}
# Empty: commands run directly (the witness). Otherwise the replay goes through `poste lourd` and the push through
# `poste push pusher`, as a session's would.
POSTE=${PUSHER_POSTE-$CACHE/poste}
HELD=${PUSHER_MASTER_HELD:-$CACHE/master-tenu}
TARGET=${PUSHER_TARGET:-target-pusher}
EMAIL=luciedefraiteur@gmail.com
ENGINE_PATHS='^(src|test|extension/vector|tools/rust_api|tools/shell)/'
Q=$D/queue.tsv
LOG=$D/log.tsv
mkdir -p "$D/inbox"
[ -f "$Q" ] || printf '# id\tbranch\thead\tkind\twitness\twitness_suite\textra_suites\tparent\tauthor\n' > "$Q"

log() { printf '%s\t%s\t%s\t%s\n' "$(date +%FT%T)" "$1" "$2" "${3:-}" >> "$LOG"; }   # <id> <event> [detail]
g() { git -C "$WT" "$@"; }
queue_lines() { grep -v '^#' "$Q"; }
in_queue() { queue_lines | awk -F'\t' -v id="$1" '$1 == id {f=1} END {exit !f}'; }
remove_from_queue() {
  ( flock 9; awk -F'\t' -v id="$1" '/^#/ || $1 != id' "$Q" > "$Q.new" && mv "$Q.new" "$Q" ) 9> "$D/queue.lock"
}
poste_lourd() { if [ -n "$POSTE" ]; then "$POSTE" lourd "$@"; else "$@"; fi; }
poste_push() { if [ -n "$POSTE" ]; then "$POSTE" push pusher -- "$@"; else "$@"; fi; }

# The kind of a lot, from what it changes since master: docs (only .md), cpp (an engine path), rust (what the push
# guard counts as code), scripts (the rest: a poste script, a measuring tool).
kind_of() { # <head>
  local mb files code
  mb=$(g merge-base origin/master "$1") || return 1
  files=$(g diff --name-only "$mb" "$1")
  if [ -z "$(echo "$files" | grep -v '\.md$' | grep .)" ]; then echo docs; return; fi
  if echo "$files" | grep -qE "$ENGINE_PATHS"; then echo cpp; return; fi
  code=$(cd "$WT" && bash "$GUARD" --code-apporte "$mb" "$1")
  if [ -n "$code" ]; then echo rust; else echo scripts; fi
}

enqueue() {
  [ $# = 9 ] || { echo "enqueue : neuf champs attendus (id branch head kind witness witness_suite extra_suites parent author)" >&2; return 2; }
  local id=$1 branch=$2 head=$3 kind=$4 parent=$8 remote full computed
  case "$id$branch$9" in *[[:space:]]*) echo "refusé : un champ porte une espace" >&2; return 1 ;; esac
  case "$kind" in docs|scripts|rust) ;; cpp) echo "refusé : un lot C++ passe par la voie transférable ou une fenêtre, pas par le pousseur" >&2; return 1 ;;
    *) echo "refusé : kind inconnu « $kind » (docs, scripts, rust)" >&2; return 1 ;; esac
  [ "$kind" = rust ] && [ "$5" = - ] && { echo "refusé : un lot rust nomme son témoin (le filtre de run_e2e.sh)" >&2; return 1; }
  in_queue "$id" && { echo "refusé : l'id $id est déjà dans la file" >&2; return 1; }
  [ "$parent" = - ] || in_queue "$parent" || { echo "refusé : le parent $parent n'est pas dans la file" >&2; return 1; }
  g fetch -q origin master "+refs/heads/$branch:refs/remotes/origin/$branch" || { echo "refusé : la branche $branch n'est pas sur origin" >&2; return 1; }
  remote=$(g rev-parse "origin/$branch")
  full=$(g rev-parse -q --verify "$head^{commit}")
  [ "$full" = "$remote" ] || { echo "refusé : la tête de $branch sur origin ($(g rev-parse --short "$remote")) n'est pas $head" >&2; return 1; }
  computed=$(kind_of "$remote") || { echo "refusé : le lot ne se lit pas contre master" >&2; return 1; }
  [ "$computed" = cpp ] && { echo "refusé : le lot touche le moteur (cpp), il passe par la voie transférable ou une fenêtre" >&2; return 1; }
  [ "$computed" = "$kind" ] || { echo "refusé : le lot est $computed, pas $kind" >&2; return 1; }
  ( flock 9; printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$id" "$branch" "$full" "$kind" "$5" "$6" "$7" "$parent" "$9" >> "$Q" ) 9> "$D/queue.lock"
  log "$id" déposé "$branch $(g rev-parse --short "$full") $kind par $9"
  echo "déposé : $id ($kind), $(queue_lines | grep -c .) lot(s) dans la file"
}

dequeue() {
  in_queue "$1" || { echo "absent de la file : $1" >&2; return 1; }
  remove_from_queue "$1"; log "$1" retiré; echo "retiré : $1"
}

# A red: out of the queue, in the log and in the author's inbox.
red() { # <id> <author> <reason> <detail>
  remove_from_queue "$1"
  log "$1" rouge "$3 : $4"
  printf 'rouge\t%s\t%s\t%s\n' "$1" "$3" "$4" >> "$D/inbox/$2.txt"
  echo "rouge $1 ($3) : $4"
}
pushed() { # <id> <author> <branch>
  local h; h=$(g rev-parse --short HEAD)
  remove_from_queue "$1"
  log "$1" poussé "$h"
  printf 'poussé\t%s\t%s\t%s\n' "$1" "$h" "$3" >> "$D/inbox/$2.txt"
  echo "poussé $1 : $h"
}

# The witnesses of a scripts lot: for each script it changes, test_<name>.sh or test_<name>.py next to it, or the
# script itself when it is a witness. A script without a witness counts as the push guard counts it: as docs.
replay_scripts() { # <id> → 0 green, 1 red (the failing witness on stdout)
  local f dir n t played=""
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    case "$f" in *.md) continue ;; esac
    dir=$(dirname "$f"); n=$(basename "$f"); n=${n%.*}
    for t in "$dir/test_$n.sh" "$dir/test_$n.py" "$f"; do
      case "$(basename "$t")" in test_*) ;; *) continue ;; esac
      [ -f "$WT/$t" ] || continue
      case " $played " in *" $t "*) continue ;; esac
      played="$played $t"
      if (cd "$WT" && case "$t" in *.py) python3 "$t" ;; *) bash "$t" ;; esac) > "$D/last-witness.log" 2>&1; then
        log "$1" témoin "$t vert"
      else
        log "$1" témoin "$t rouge"; echo "$t"; return 1
      fi
    done
    [ -n "$played" ] || log "$1" témoin "$f sans témoin (compté comme doc, comme la garde)"
  done < <(g diff --name-only "$BASE" HEAD)
  return 0
}

# A Rust lot: one pinned pass under `poste lourd`, the peak of its memory in the log. An environment red (code 124,
# poste held, a service that refuses) is replayed once; anything else is a code red.
replay_rust() { # <id> <journals> <witness> <witness_suite> <extra_suites> → 0 green, 1 code red, 2 environment red
  local id=$1 J=$2 witness=$3 suite=$4 extra=$5 rc peak last
  [ "$suite" = - ] && suite=e2e
  [ "$extra" = - ] && extra=
  poste_lourd env CIBLE="$TARGET" SUITE_DU_TEMOIN="$suite" SUITES_EN_PLUS="${extra//,/ }" POSTE_MEM_MAX=32G \
    PEAK_FILE="$JR/$J.peak" bash -c '"$@"; rc=$?; cat "/sys/fs/cgroup$(sed -n "s/^0:://p" /proc/self/cgroup)/memory.peak" > "$PEAK_FILE" 2>/dev/null; exit $rc' \
    _ bash "$PASS" "$WTN" "$J" "$witness" - > "$JR/$J.out" 2>&1
  rc=$?
  peak=$(cat "$JR/$J.peak" 2>/dev/null); peak=${peak:-?}
  last=$(tail -1 "$JR/$J/resume.log" 2>/dev/null)
  log "$id" rejoué "$J : $last (rc $rc) peak=$peak"
  [ "$last" = VERT ] && return 0
  if [ "$rc" = 124 ] || grep -qE '\(124\)|poste tenu|Connection refused|refuse la pose' "$JR/$J/resume.log" 2>/dev/null; then return 2; fi
  return 1
}

# Master, re-read after the replay: docs and scripts are pushed by the pusher itself (the guard is for a pass), under
# `poste push`, and only if master has not moved since the repose; otherwise the lot stays in the queue, reposed next step.
push_plain() { # <base> → 0 pushed, 1 master moved, 2 refused
  g fetch -q origin master || return 2
  [ "$(g rev-parse FETCH_HEAD)" = "$1" ] || return 1
  poste_push git -C "$WT" push -q origin HEAD:master || return 2
  g fetch -q origin master
  [ -z "$(g status --porcelain)" ] && [ ! -d "$(g rev-parse --git-dir)/rebase-merge" ] && [ "$(g rev-parse FETCH_HEAD)" = "$(g rev-parse HEAD)" ]
}

# ── The train (lot 2) ────────────────────────────────────────────────────────────────────────────────────────────
# A step takes up to PUSHER_TRAIN_MAX lots from the head of the queue (4 by default), reposes them in order on
# master, replays the train once and pushes it all when green. When red, the bisection: the first half is reposed
# and replayed alone, down to one lot; a green prefix is pushed, and what follows it stays at the head of the queue
# for the next step. A red lot leaves the queue with its children (a lot written on it).
TRAIN_MAX=${PUSHER_TRAIN_MAX:-4}
f_() { printf '%s\n' "$1" | cut -f"$2"; }   # <queue line> <field number>
kind_rank() { case "$1" in docs) echo 0 ;; scripts) echo 1 ;; *) echo 2 ;; esac; }

# A red lot and, through the queue, its children and theirs.
red_family() { # <id> <author> <reason> <detail>
  local child
  red "$1" "$2" "$3" "$4"
  queue_lines | awk -F'\t' -v p="$1" '$8 == p {print $1 "\t" $9}' | while IFS=$'\t' read -r child author; do
    red_family "$child" "$author" "parent rouge" "son parent $1 est rouge"
  done
}

# Repose the given lines on BASE, in the pusher's worktree, under the branch RB. Prints the id of the lot whose
# cherry-pick conflicts and returns 1; returns 0 when every lot is reposed.
repose() { # <line>…
  local line id head commits
  g checkout -q -f -B "$RB" "$BASE" && g clean -q -fd
  for line in "$@"; do
    id=$(f_ "$line" 1); head=$(f_ "$line" 3)
    commits=$(g rev-list --reverse --no-merges "HEAD..$head")
    [ -n "$commits" ] || continue
    if ! g cherry-pick --empty=drop $commits > "$D/last-repose.log" 2>&1; then
      g cherry-pick --abort 2>/dev/null
      echo "$id"; return 1
    fi
  done
  return 0
}

# Replay the reposed train by its heaviest kind: 0 green, 1 code red, 2 environment red. JNAME receives the journals
# name of a Rust pass, for the guard.
replay_train() { # <line>…
  local line rank=0 r first="" suite=e2e extra="" w s x
  for line in "$@"; do r=$(kind_rank "$(f_ "$line" 4)"); [ "$r" -gt "$rank" ] && rank=$r; done
  case $rank in
    0) return 0 ;;
    1) replay_scripts "$TRAIN_LABEL" > "$D/last-red.txt" && return 0; return 1 ;;
  esac
  # A Rust train: one pass. Its witness is the first Rust lot's; the other Rust lots' witnesses (e2e suites) and
  # every lot's extra suites join SUITES_EN_PLUS; a lib witness is covered by the whole lib that the pass plays.
  for line in "$@"; do
    [ "$(f_ "$line" 4)" = rust ] || continue
    w=$(f_ "$line" 5); s=$(f_ "$line" 6); x=$(f_ "$line" 7)
    if [ -z "$first" ]; then first=$w; [ "$s" = - ] || suite=$s
    elif [ "$s" != lib ]; then extra="$extra,$w"; fi
    [ "$x" = - ] || extra="$extra,$x"
  done
  extra=$(echo "${extra#,}" | tr ',' '\n' | awk 'NF && !seen[$0]++' | paste -sd,)
  JNAME=pusher-$STAMP-$TRAIN_LABEL
  replay_rust "$TRAIN_LABEL" "$JNAME" "$first" "$suite" "${extra:--}"
  case $? in
    0) return 0 ;;
    1) tail -1 "$JR/$JNAME/resume.log" > "$D/last-red.txt" 2>/dev/null; return 1 ;;
  esac
  log "$TRAIN_LABEL" "rouge d'environnement, rejeu"
  JNAME=$JNAME-2
  replay_rust "$TRAIN_LABEL" "$JNAME" "$first" "$suite" "${extra:--}"
  case $? in
    0) return 0 ;;
    1) tail -1 "$JR/$JNAME/resume.log" > "$D/last-red.txt" 2>/dev/null; return 1 ;;
    *) return 2 ;;
  esac
}

# Push the reposed train: a Rust train through the guard, the others by the pusher itself. 0 pushed, 1 master moved
# (the lots stay at the head of the queue), 2 refused (the message in $D/last-red.txt).
push_train() { # <line>…
  local line rank=0 r out
  for line in "$@"; do r=$(kind_rank "$(f_ "$line" 4)"); [ "$r" -gt "$rank" ] && rank=$r; done
  if [ "$rank" = 2 ]; then
    out=$(poste_push bash "$GUARD" "$WTN" "$JNAME" 2>&1)
    log "$TRAIN_LABEL" garde "$(echo "$out" | tail -1)"
    [ "$(echo "$out" | tail -1)" = POUSSE ] && return 0
    echo "$out" | grep -qE 'master a apporté du code|push refusé' && return 1
    echo "$out" | grep -m1 'ARRÊT' > "$D/last-red.txt"; return 2
  fi
  push_plain "$BASE"; r=$?
  [ $r = 2 ] && echo "push refusé ou contrôles faux après le push" > "$D/last-red.txt"
  return $r
}

step() {
  [ -e "$D/paused" ] && { echo "file en pause : $(cat "$D/paused")"; return 0; }
  [ -e "$HELD" ] && { echo "master tenu : $(cat "$HELD")"; return 0; }
  [ "$(g config user.email)" = "$EMAIL" ] || { echo "identité git du worktree : $(g config user.email)" > "$D/paused"; echo "pause : identité git"; return 1; }
  local -a train=() kept=() part=()
  local line id branch author refs="" n half reposed=0
  while IFS= read -r line; do [ ${#train[@]} -lt "$TRAIN_MAX" ] && train+=("$line"); done < <(queue_lines)
  [ ${#train[@]} -gt 0 ] || { echo "file vide"; return 0; }
  for line in "${train[@]}"; do
    branch=$(f_ "$line" 2); refs="$refs +refs/heads/$branch:refs/remotes/origin/$branch"
    log "$(f_ "$line" 1)" pris "$branch $(f_ "$line" 4), train de ${#train[@]}"
  done
  # shellcheck disable=SC2086
  g fetch -q origin master $refs || { echo "fetch du train" > "$D/last-red.txt"; log train "fetch refusé, rejoué au pas suivant"; return 0; }
  # A branch that moved since its deposit: returned, with its children.
  for line in "${train[@]}"; do
    id=$(f_ "$line" 1); branch=$(f_ "$line" 2); author=$(f_ "$line" 9)
    in_queue "$id" || continue
    if [ "$(g rev-parse "origin/$branch")" != "$(f_ "$line" 3)" ]; then
      red_family "$id" "$author" "tête bougée" "$branch est sur $(g rev-parse --short "origin/$branch"), déposé à $(g rev-parse --short "$(f_ "$line" 3)") : redéposer"
    else kept+=("$line"); fi
  done
  BASE=$(g rev-parse origin/master)
  STAMP=$(date +%Y%m%d-%H%M%S)
  # The repose: a conflicting lot leaves the train (and the queue, with its children), the rest is reposed again.
  while :; do
    train=(); for line in "${kept[@]}"; do in_queue "$(f_ "$line" 1)" && train+=("$line"); done
    [ ${#train[@]} -gt 0 ] || return 0
    TRAIN_LABEL=$(f_ "${train[0]}" 1); [ ${#train[@]} -gt 1 ] && TRAIN_LABEL="$TRAIN_LABEL+$(( ${#train[@]} - 1 ))"
    RB="pusher/$STAMP-$TRAIN_LABEL"
    id=$(repose "${train[@]}") && break
    for line in "${train[@]}"; do [ "$(f_ "$line" 1)" = "$id" ] && author=$(f_ "$line" 9); done
    red_family "$id" "$author" conflit "la repose sur master $(g rev-parse --short "$BASE") bute : $(grep -m1 -E 'CONFLICT|error' "$D/last-repose.log")"
    kept=("${train[@]}")
  done
  # The replay, with the bisection: part is the prefix under test.
  part=("${train[@]}")
  while :; do
    if [ "$reposed" = 1 ]; then
      TRAIN_LABEL=$(f_ "${part[0]}" 1); [ ${#part[@]} -gt 1 ] && TRAIN_LABEL="$TRAIN_LABEL+$(( ${#part[@]} - 1 ))"
      RB="pusher/$STAMP-$TRAIN_LABEL"
      repose "${part[@]}" > /dev/null || { log "$TRAIN_LABEL" "repose d'une moitié refusée"; return 0; }
    fi
    reposed=1
    # Every commit of the part already on master (pushed by another way): nothing to replay or push.
    if [ "$(g rev-parse HEAD)" = "$BASE" ]; then
      for line in "${part[@]}"; do log "$(f_ "$line" 1)" "déjà sur master"; pushed "$(f_ "$line" 1)" "$(f_ "$line" 9)" "$(f_ "$line" 2)"; done
      return 0
    fi
    g push -q origin "HEAD:refs/heads/$RB" || { log "$TRAIN_LABEL" "push de la repose $RB refusé, rejoué au pas suivant"; return 0; }
    log "$TRAIN_LABEL" reposé "$RB $(g rev-parse --short HEAD) sur $(g rev-parse --short "$BASE")"
    replay_train "${part[@]}"
    case $? in
      0) break ;;
      2) for line in "${part[@]}"; do red_family "$(f_ "$line" 1)" "$(f_ "$line" 9)" environnement "deux rouges d'environnement pour le train $TRAIN_LABEL : appeler c7"; done
         return 0 ;;
    esac
    n=${#part[@]}
    if [ "$n" = 1 ]; then
      red_family "$(f_ "${part[0]}" 1)" "$(f_ "${part[0]}" 9)" code "$(cat "$D/last-red.txt" 2>/dev/null)"
      return 0
    fi
    half=$(( n / 2 ))
    log "$TRAIN_LABEL" bissection "rouge sur $n lots, rejoue les $half premiers"
    part=("${part[@]:0:$half}")
  done
  push_train "${part[@]}"
  case $? in
    0) for line in "${part[@]}"; do pushed "$(f_ "$line" 1)" "$(f_ "$line" 9)" "$(f_ "$line" 2)"; done ;;
    1) log "$TRAIN_LABEL" "master a bougé : reposé au pas suivant" ;;
    *) for line in "${part[@]}"; do red_family "$(f_ "$line" 1)" "$(f_ "$line" 9)" garde "$(cat "$D/last-red.txt" 2>/dev/null)"; done ;;
  esac
  return 0
}

case "${1:-}" in
  enqueue) shift; enqueue "$@" ;;
  dequeue) dequeue "$2" ;;
  step) step ;;
  run) while :; do out=$(step); echo "$out"; case "$out" in "file vide"|"file en pause"*|"master tenu"*) sleep "${PUSHER_IDLE:-60}" ;; "pause : "*) exit 1 ;; esac; done ;;
  *) sed -n '2,20p' "$0"; exit 2 ;;
esac
