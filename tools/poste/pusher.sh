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
  done < <(g diff --name-only origin/master HEAD)
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

step() {
  [ -e "$D/paused" ] && { echo "file en pause : $(cat "$D/paused")"; return 0; }
  [ -e "$HELD" ] && { echo "master tenu : $(cat "$HELD")"; return 0; }
  [ "$(g config user.email)" = "$EMAIL" ] || { echo "identité git du worktree : $(g config user.email)" > "$D/paused"; echo "pause : identité git"; return 1; }
  local line id branch head kind witness suite extra parent author base commits rb n J out
  line=$(queue_lines | head -1)
  [ -n "$line" ] || { echo "file vide"; return 0; }
  IFS=$'\t' read -r id branch head kind witness suite extra parent author <<< "$line"
  log "$id" pris "$branch $kind"
  g fetch -q origin master "+refs/heads/$branch:refs/remotes/origin/$branch" || { red "$id" "$author" environnement "fetch de $branch"; return 0; }
  [ "$(g rev-parse "origin/$branch")" = "$head" ] || { red "$id" "$author" "tête bougée" "$branch est sur $(g rev-parse --short "origin/$branch"), déposé à $(g rev-parse --short "$head") : redéposer"; return 0; }
  # The repose: the lot's commits, cherry-picked on master under a new name. A conflict is the author's to resolve.
  base=$(g rev-parse origin/master)
  n=$(date +%Y%m%d-%H%M%S)
  rb="pusher/$n-$id"
  g checkout -q -f -B "$rb" "$base" && g clean -q -fd
  commits=$(g rev-list --reverse --no-merges "$base..$head")
  if [ -n "$commits" ] && ! g cherry-pick --empty=drop $commits > "$D/last-repose.log" 2>&1; then
    g cherry-pick --abort 2>/dev/null
    red "$id" "$author" conflit "la repose sur master $(g rev-parse --short "$base") bute : $(grep -m1 -E 'CONFLICT|error' "$D/last-repose.log")"
    return 0
  fi
  [ "$(g rev-parse HEAD)" = "$base" ] && { log "$id" "déjà sur master"; pushed "$id" "$author" "$branch"; return 0; }
  g push -q origin "HEAD:refs/heads/$rb" || { red "$id" "$author" environnement "push de la repose $rb"; return 0; }
  log "$id" reposé "$rb $(g rev-parse --short HEAD) sur $(g rev-parse --short "$base")"
  case "$kind" in
    docs) ;;
    scripts)
      out=$(replay_scripts "$id") || { red "$id" "$author" code "témoin rouge : $out ($D/last-witness.log)"; return 0; } ;;
    rust)
      J=pusher-$n-$id
      replay_rust "$id" "$J" "$witness" "$suite" "$extra"
      case $? in
        0) ;;
        1) red "$id" "$author" code "passe rouge : $(tail -1 "$JR/$J/resume.log" 2>/dev/null) ($JR/$J)"; return 0 ;;
        2) log "$id" "rouge d'environnement, rejeu"
           J=$J-2
           replay_rust "$id" "$J" "$witness" "$suite" "$extra"
           case $? in
             0) ;;
             1) red "$id" "$author" code "passe rouge au rejeu : $(tail -1 "$JR/$J/resume.log" 2>/dev/null) ($JR/$J)"; return 0 ;;
             *) red "$id" "$author" environnement "deux rouges d'environnement ($JR/$J)"; return 0 ;;
           esac ;;
      esac
      out=$(poste_push bash "$GUARD" "$WTN" "$J" 2>&1)
      log "$id" garde "$(echo "$out" | tail -1)"
      if [ "$(echo "$out" | tail -1)" = POUSSE ]; then pushed "$id" "$author" "$branch"; return 0; fi
      if echo "$out" | grep -qE 'master a apporté du code|push refusé'; then log "$id" "master a bougé : reposé au pas suivant"; return 0; fi
      red "$id" "$author" garde "$(echo "$out" | grep -m1 'ARRÊT')"; return 0 ;;
  esac
  push_plain "$base"
  case $? in
    0) pushed "$id" "$author" "$branch" ;;
    1) log "$id" "master a bougé : reposé au pas suivant" ;;
    *) red "$id" "$author" environnement "push refusé ou contrôles faux après le push" ;;
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
