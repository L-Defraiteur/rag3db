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
#   pusher.sh open-prs <stamp> <journals|-> <final head>
#                           one pull request per lot of the last repose (called through PRE_PUSH by the guard)
#   pusher.sh pr-catchup <id> <master before> <tip> <author> <subject>
#                           the pull request of a lot that landed without it, closed with where it landed
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
# Where the cache copies of the poste's scripts live, and the other postes that hold the same copies.
CACHE_DIR=${PUSHER_CACHE:-$CACHE}
PEERS=${PUSHER_PEERS-lucie-tablette}
# What the pinned lib is built from: a change here on master after the lib's commit makes the lib late.
LIB_PATHS=${PUSHER_LIB_PATHS:-src/ extension/vector/}
# The pass that pins the lib: once recopied, lib-des-passes says the lib it pins (tour.sh reads that file first).
PIN_SCRIPT=${PUSHER_PIN_SCRIPT:-passe_epinglee.sh}
# One pull request per lot (19 Nov., c7 and G): gh under the token of PR_ACCOUNT, never the active account.
GH=${PUSHER_GH:-gh}
PR_REPO=${PUSHER_PR_REPO:-L-Defraiteur/rag3db}
PR_ACCOUNT=L-Defraiteur
SELF=$(realpath "$0")
EMAIL=luciedefraiteur@gmail.com
ENGINE_PATHS='^(src|test|extension/vector|tools/rust_api|tools/shell)/'
Q=$D/queue.tsv
LOG=$D/log.tsv
mkdir -p "$D/inbox"
printf 'docs/nouveautes.md merge=union\n' > "$D/attributes"
[ -f "$Q" ] || printf '# id\tbranch\thead\tkind\twitness\twitness_suite\textra_suites\tparent\tauthor\n' > "$Q"

log() { printf '%s\t%s\t%s\t%s\n' "$(date +%FT%T)" "$1" "$2" "${3:-}" >> "$LOG"; }   # <id> <event> [detail]
# Git in English: the pusher reads its messages (19 Nov.: git spoke French at luciepc, and a conflict had no name).
g() { LC_ALL=C git -C "$WT" "$@"; }
# **The fetch, under a file lock shared with the passes' scripts**, retried on a ref lock (19 Nov., relevé d'archi: a
# « cannot lock ref » when two fetches of the shared repository of luciepc crossed). Three tries, three seconds apart.
FETCH_LOCK=${PUSHER_FETCH_LOCK:-$CACHE/git-fetch.lock}
gfetch() {
  local i out
  for i in 1 2 3; do
    out=$(flock "$FETCH_LOCK" env LC_ALL=C git -C "$WT" fetch "$@" 2>&1) && return 0
    if echo "$out" | grep -qE 'cannot lock ref|Unable to create .*\.lock'; then
      log fetch "verrou de ref" "essai $i : $(echo "$out" | grep -m1 -E 'cannot lock ref|\.lock')"; sleep 3
    else
      echo "$out" >&2; return 1
    fi
  done
  return 1
}
queue_lines() { grep -v '^#' "$Q"; }
# The original head of a lot: from the queue, or from landed.tsv once pushed.
origin_head() { # <id>
  local h
  h=$(queue_lines | awk -F'\t' -v id="$1" '$1 == id {print $3}' | tail -1)
  [ -n "$h" ] || h=$(awk -F'\t' -v id="$1" '$1 == id {print $2}' "$D/landed.tsv" 2>/dev/null | tail -1)
  [ -n "$h" ] && echo "$h"
}
in_queue() { queue_lines | awk -F'\t' -v id="$1" '$1 == id {f=1} END {exit !f}'; }
remove_from_queue() {
  ( flock 9; awk -F'\t' -v id="$1" '/^#/ || $1 != id' "$Q" > "$Q.new" && mv "$Q.new" "$Q" ) 9> "$D/queue.lock"
}
# The ceiling is poste's own (19 Nov.: given to the pass only, poste refused every validation under its 40G default).
poste_lourd() { if [ -n "$POSTE" ]; then POSTE_MEM_MAX=32G "$POSTE" lourd "$@"; else "$@"; fi; }
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
  [ "$parent" = - ] || in_queue "$parent" || origin_head "$parent" > /dev/null || { echo "refusé : le parent $parent n'est ni dans la file ni poussé (landed.tsv)" >&2; return 1; }
  gfetch -q origin master "+refs/heads/$branch:refs/remotes/origin/$branch" || { echo "refusé : la branche $branch n'est pas sur origin" >&2; return 1; }
  remote=$(g rev-parse "origin/$branch")
  full=$(g rev-parse -q --verify "$head^{commit}")
  [ "$full" = "$remote" ] || { echo "refusé : la tête de $branch sur origin ($(g rev-parse --short "$remote")) n'est pas $head" >&2; return 1; }
  computed=$(kind_of "$remote") || { echo "refusé : le lot ne se lit pas contre master" >&2; return 1; }
  # The witness's suite is SUITE_DU_TEMOIN: « lib », or the name of an e2e suite of the lot's tree (19 Nov., relevé d'ED).
  if [ "$kind" = rust ] && [ "$6" != lib ] && ! g cat-file -e "$remote:extension/rag3weaver/tests/$6.rs" 2>/dev/null; then
    echo "refusé : la suite du témoin « $6 » n'est ni lib ni une suite e2e du lot (extension/rag3weaver/tests/$6.rs)" >&2; return 1
  fi
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
  # The line is kept: a red of the pusher's own making is deposited again as it was (19 Nov.).
  queue_lines | awk -F'\t' -v id="$1" '$1 == id' >> "$D/returned.tsv"
  remove_from_queue "$1"
  log "$1" rouge "$3 : $4"
  printf 'rouge\t%s\t%s\t%s\n' "$1" "$3" "$4" >> "$D/inbox/$2.txt"
  echo "rouge $1 ($3) : $4"
}
pushed() { # <id> <author> <branch>
  local h pr="" tok; h=$(g rev-parse --short HEAD)
  pr=$(awk -F'\t' -v id="$1" '$1 == id {n=$2} END {print n}' "$D/step-prs.tsv" 2>/dev/null)
  # The lot's original head, kept: a child deposited on it later reposes on its own commits only (19 Nov., c7).
  queue_lines | awk -F'\t' -v id="$1" -v h="$(g rev-parse HEAD)" '$1 == id {print $1 "\t" $3 "\t" h}' >> "$D/landed.tsv"
  remove_from_queue "$1"
  log "$1" poussé "$h${pr:+ PR $pr}"
  printf 'poussé\t%s\t%s\t%s%s\n' "$1" "$h" "$3" "${pr:+	PR $pr}" >> "$D/inbox/$2.txt"
  # The three controls held (push_plain, or the guard's POUSSE): said on the pull request, which GitHub has marked
  # merged since master holds its head. A failed comment is said, never blocking.
  if [ -n "$pr" ]; then
    tok=$(gh_token) && GH_TOKEN=$tok "$GH" pr comment "$pr" --repo "$PR_REPO" --body \
      "Atterri sur master à $(g rev-parse HEAD). Les trois contrôles : status propre, aucun rebase en cours, FETCH_HEAD = HEAD." > /dev/null 2>&1 \
      || log "$1" "PR $pr : commentaire absent"
  fi
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
  # No summary: the pass never ran (poste refused it, the worktree was missing) — the environment, not the lot.
  [ -s "$JR/$J/resume.log" ] || return 2
  if [ "$rc" = 124 ] || grep -qE '\(124\)|poste tenu|Connection refused|refuse la pose' "$JR/$J/resume.log" 2>/dev/null; then return 2; fi
  return 1
}

# Master, re-read after the replay: docs and scripts are pushed by the pusher itself (the guard is for a pass), under
# `poste push`, and only if master has not moved since the repose; otherwise the lot stays in the queue, reposed next step.
push_plain() { # <base> → 0 pushed, 1 master moved, 2 refused
  gfetch -q origin master || return 2
  [ "$(g rev-parse FETCH_HEAD)" = "$1" ] || return 1
  open_prs "$STAMP" - "$(g rev-parse HEAD)"
  poste_push git -C "$WT" push -q origin HEAD:master || { close_step_prs "le push vers master a été refusé"; return 2; }
  gfetch -q origin master
  [ -z "$(g status --porcelain)" ] && [ ! -d "$(g rev-parse --git-dir)/rebase-merge" ] && [ "$(g rev-parse FETCH_HEAD)" = "$(g rev-parse HEAD)" ]
}

# ── One pull request per lot (19 Nov., c7 and G) ──────────────────────────────────────────────────────────────────
# Before the push, each lot of the train gets a branch pusher/<STAMP>-<id>-final at its own tip on the final head, and
# a pull request from it to master. The push is a fast-forward to the final head, so GitHub marks every one of them
# merged. A pull request that cannot be opened (gh down, no token, no tip) is « PR absente » in the log and never
# stops the push; pr-catchup opens it later against a branch at the master of before. No ref is ever deleted.

# The token of PR_ACCOUNT, checked: its login must be PR_ACCOUNT. The active account is never touched.
gh_token() {
  local tok
  tok=$("$GH" auth token --user "$PR_ACCOUNT" 2>/dev/null) && [ -n "$tok" ] || return 1
  [ "$(GH_TOKEN=$tok "$GH" api user --jq .login 2>/dev/null)" = "$PR_ACCOUNT" ] || return 1
  printf '%s' "$tok"
}

# A pull request's body: what was replayed, under which lib; no attribution line.
pr_body() { # <id> <branch> <author> <kind> <witness> <suite> <extra> <tip> <final> <journals>
  local lib="aucune : pas de passe Rust"
  # The lib in its long form, from every journal of the pass, as the guard reads it (the summary's first line carries
  # only the short form, 19 Nov., G).
  if [ "${10}" != - ]; then
    lib=$(grep -ho 'rag3db-build-[0-9a-f]\{40\}' "$JR/${10}"/*.log 2>/dev/null | sort -u | paste -sd' ')
    [ -n "$lib" ] || lib="non lue"
  fi
  printf 'Lot %s de la session %s, branche %s (%s), reposé et poussé par le pousseur.\n\n' "$1" "$3" "$2" "$4"
  printf -- '- Lib : %s\n- Témoin : %s (%s) ; suites en plus : %s\n- Rejeu : %s\n' "$lib" "$5" "$6" "$7" "${10/#-/sans passe}"
  printf -- '- Pointe du lot sur la tête finale : %s ; tête finale : %s\n' "$8" "$9"
  printf -- '- Les trois contrôles suivent le push, en commentaire de cette PR.\n'
}

# Open one pull request per lot of the last repose, on the final head: never fails. Called by push_plain before its
# push, and by the guard through PRE_PUSH (the final head appended after the guard's own rebase).
open_prs() { # <stamp> <journals|-> <final head>
  local stamp=$1 jn=$2 final=$3 tok master id branch author kind w s x pids subj tip last n url err
  : > "$D/step-prs.tsv"
  [ -s "$D/step-lots.tsv" ] || return 0
  if ! tok=$(gh_token); then
    while IFS=$'\t' read -r id _; do log "$id" "PR absente : pas de jeton gh de $PR_ACCOUNT"; done < "$D/step-lots.tsv"
    return 0
  fi
  gfetch -q origin master 2>/dev/null; master=$(g rev-parse FETCH_HEAD)
  # The commits of the final head over master, in order, by patch id: each lot's tip is its last own commit; the last
  # lot's tip is the final head itself (a commit the guard adds, the copies of closed tickets, goes with it).
  g log -p --no-merges --reverse --format='commit %H' "$master..$final" | g patch-id --stable > "$D/final-patchids"
  last=$(tail -1 "$D/step-lots.tsv" | cut -f1)
  while IFS=$'\t' read -r id branch author kind w s x pids subj; do
    tip=$(awk -v ids=" $pids " 'index(ids, " " $1 " ") {t=$2} END {print t}' "$D/final-patchids")
    [ "$id" = "$last" ] && [ -n "$tip" ] && tip=$final
    [ -n "$tip" ] || { log "$id" "PR absente : aucun commit du lot sur la tête finale"; continue; }
    g push -q origin "$tip:refs/heads/pusher/$stamp-$id-final" 2>/dev/null || { log "$id" "PR absente : push de pusher/$stamp-$id-final refusé"; continue; }
    if url=$(GH_TOKEN=$tok "$GH" pr create --repo "$PR_REPO" --base master --head "pusher/$stamp-$id-final" \
        --title "$id : $subj" --body "$(pr_body "$id" "$branch" "$author" "$kind" "$w" "$s" "$x" "$tip" "$final" "$jn")" 2>"$D/last-gh.err"); then
      n=${url##*/}
      log "$id" "PR $n ouverte" "$url"
      printf '%s\t%s\n' "$id" "$n" >> "$D/step-prs.tsv"
    else
      err=$(head -1 "$D/last-gh.err"); log "$id" "PR absente : gh pr create a échoué" "$err"
    fi
  done < "$D/step-lots.tsv"
  return 0
}

# The pull requests of this step that will not land: the push was refused after they opened (master moved, the guard
# stopped). Each is closed with the reason, « PR <n> fermée » in the log; a lot reposed later gets a new one (19 Nov., G).
close_step_prs() { # <reason>
  local tok id n
  [ -s "$D/step-prs.tsv" ] || return 0
  tok=$(gh_token) || { while IFS=$'\t' read -r id n; do log "$id" "PR $n : fermeture impossible, pas de jeton" "$1"; done < "$D/step-prs.tsv"; : > "$D/step-prs.tsv"; return 0; }
  while IFS=$'\t' read -r id n; do
    if GH_TOKEN=$tok "$GH" pr close "$n" --repo "$PR_REPO" --comment "fermée sans atterrir : $1" > /dev/null 2>&1; then
      log "$id" "PR $n fermée" "$1"
    else
      log "$id" "PR $n : fermeture refusée" "$1"
    fi
  done < "$D/step-prs.tsv"
  : > "$D/step-prs.tsv"
}

# A lot that landed without its pull request: a branch -base at the master it landed on, -final at its tip, the pull
# request from one to the other, closed with where it landed. No ref is deleted.
pr_catchup() { # <id> <master before> <tip> <author> <subject>
  local tok stamp url n
  tok=$(gh_token) || { log "$1" "PR absente : pas de jeton gh de $PR_ACCOUNT (rattrapage)"; return 1; }
  stamp=$(date +%Y%m%d-%H%M%S)
  g push -q origin "$2:refs/heads/pusher/$stamp-$1-base" "$3:refs/heads/pusher/$stamp-$1-final" || { log "$1" "PR absente : push des branches du rattrapage refusé"; return 1; }
  url=$(GH_TOKEN=$tok "$GH" pr create --repo "$PR_REPO" --base "pusher/$stamp-$1-base" --head "pusher/$stamp-$1-final" \
    --title "$1 : $5" --body "Lot $1 de la session $4, poussé par le pousseur sans sa PR ; rattrapage. Atterri sur master à $3.") \
    || { log "$1" "PR absente : gh pr create a échoué (rattrapage)"; return 1; }
  n=${url##*/}
  log "$1" "PR $n ouverte" "$url (rattrapage)"
  GH_TOKEN=$tok "$GH" pr close "$n" --repo "$PR_REPO" --comment "atterri sur master à $3" > /dev/null 2>&1 \
    && log "$1" "PR $n fermée" "atterri sur master à $3" || log "$1" "PR $n : fermeture refusée"
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
  local line id head commits par pex before
  g checkout -q -f -B "$RB" "$BASE" && g clean -q -fd
  # The lots of this repose, for their pull requests (open_prs): one line each, written after its cherry-pick.
  : > "$D/step-lots.tsv"
  for line in "$@"; do
    id=$(f_ "$line" 1); head=$(f_ "$line" 3); par=$(f_ "$line" 8); pex=""
    # A child leaves out its parent's original commits by the parent's original head (c7's rule: rebase --onto it);
    # the patch filter stays under it, for commits already on master by another way.
    [ "$par" = - ] || pex=$(origin_head "$par")
    # By patch: a commit whose change is already on the reposed head (a parent that landed reposed, under new hashes)
    # is left out (19 Nov.: picking it again conflicted with the parent's later commits).
    commits=$(g rev-list --reverse --no-merges --cherry-pick --right-only "HEAD...$head" ${pex:+"^$pex"})
    [ -n "$commits" ] || continue
    # docs/nouveautes.md takes every lot's line at its head: the two sides are kept (union), as a session resolves
    # it by hand. Any other collision stays a conflict for the author.
    # --allow-empty keeps a commit empty from its origin (a marker); --empty=drop still drops one that became empty.
    before=$(g rev-parse HEAD)
    if ! g -c core.attributesFile="$D/attributes" cherry-pick --allow-empty --empty=drop $commits > "$D/last-repose.log" 2>&1; then
      # The files in conflict, named for the author (19 Nov., c7: « bute : » said nothing).
      g diff --name-only --diff-filter=U | paste -sd' ' > "$D/last-conflict.txt"
      g cherry-pick --abort 2>/dev/null
      echo "$id"; return 1
    fi
    # id, branch, author, kind, witness, witness suite, extra suites, the patch ids of its reposed commits, subject.
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$id" "$(f_ "$line" 2)" "$(f_ "$line" 9)" "$(f_ "$line" 4)" \
      "$(f_ "$line" 5)" "$(f_ "$line" 6)" "$(f_ "$line" 7)" \
      "$(g log -p --no-merges --format='commit %H' "$before..HEAD" | g patch-id --stable | awk '{print $1}' | paste -sd' ')" \
      "$(g log -1 --format=%s HEAD)" >> "$D/step-lots.tsv"
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
    # A later lot adds its suite: with « e2e » (or « - ») its witness names an e2e suite; with a named suite its
    # witness is a test of it, played by playing the suite (19 Nov.: a test's name was given as a suite).
    if [ -z "$first" ]; then first=$w; [ "$s" = - ] || suite=$s
    else case "$s" in lib) ;; e2e|-) extra="$extra,$w" ;; *) extra="$extra,$s" ;; esac; fi
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
    # The guard calls PRE_PUSH with the final head, after its own rebase: the pull requests open on what lands.
    out=$(PRE_PUSH="bash $SELF open-prs $STAMP $JNAME" poste_push bash "$GUARD" "$WTN" "$JNAME" 2>&1)
    log "$TRAIN_LABEL" garde "$(echo "$out" | tail -1)"
    [ "$(echo "$out" | tail -1)" = POUSSE ] && return 0
    # The guard did not push: the pull requests it opened through PRE_PUSH will not land.
    close_step_prs "la garde n'a pas poussé : $(echo "$out" | grep -m1 'ARRÊT' || echo "$out" | tail -1)"
    echo "$out" | grep -qE 'master a apporté du code|push refusé' && return 1
    echo "$out" | grep -m1 'ARRÊT' > "$D/last-red.txt"; return 2
  fi
  push_plain "$BASE"; r=$?
  [ $r = 2 ] && echo "push refusé ou contrôles faux après le push" > "$D/last-red.txt"
  return $r
}

# ── The lib and the cache (lot 3) ─────────────────────────────────────────────────────────────────────────────
# The pinned lib is the one the pass names (RAG3DB_LIB=rag3db-build-<commit>-…). It is late when master has changed
# what it is built from since its commit: the Rust lots wait, c7 is told (lib-late), the bascule stays a window.
# Prints the reason when late, nothing when the lib is up to date.
lib_late() {
  local lc changed
  lc=$(grep -om1 'rag3db-build-[0-9a-f]\{40\}' "$PASS" | sed 's/rag3db-build-//')
  [ -n "$lc" ] || { echo "la passe ($PASS) ne nomme aucune lib"; return; }
  g cat-file -e "$lc^{commit}" 2>/dev/null || { echo "la lib épinglée $lc n'est pas un commit connu du worktree"; return; }
  # shellcheck disable=SC2086
  changed=$(g diff --name-only "$lc" origin/master -- $LIB_PATHS | head -3 | paste -sd' ')
  [ -z "$changed" ] || echo "lib épinglée sur ${lc:0:10}, master a changé le moteur depuis ($changed…) : bascule à faire"
}

# After a pushed train: the cache copies of what it changed, by the table tools/poste/cache_copies.txt of the pushed
# tree (« <source> <name in the cache> <witness> »). The old copy is kept as <name>.avant-<label>, the new one
# installed, its witness played against the copy; a red witness puts the old copy back and pauses the queue. The other
# postes get the same bytes, proved by their sha256. Returns 3 when pusher.sh itself was recopied (run re-executes).
recopy() { # <label>
  local label=$1 src name witness sum peer rc=0 table lib
  table=$(g show HEAD:tools/poste/cache_copies.txt 2>/dev/null | grep -vE '^[[:space:]]*(#|$)') || return 0
  while read -r src name witness; do
    g diff --quiet "$BASE" HEAD -- "$src" && continue
    [ -f "$CACHE_DIR/$name" ] && cp -p "$CACHE_DIR/$name" "$CACHE_DIR/$name.avant-$label"
    install -m 755 "$WT/$src" "$CACHE_DIR/$name.neuf" && mv "$CACHE_DIR/$name.neuf" "$CACHE_DIR/$name"
    sum=$(sha256sum "$CACHE_DIR/$name" | cut -c1-16)
    if (cd "$WT" && bash "$witness" "$CACHE_DIR/$name") > "$D/last-recopy.log" 2>&1; then
      log "$label" recopié "$name $sum, témoin vert"
    else
      [ -f "$CACHE_DIR/$name.avant-$label" ] && mv "$CACHE_DIR/$name.avant-$label" "$CACHE_DIR/$name"
      log "$label" "recopie refusée" "$name : témoin rouge contre la copie ($D/last-recopy.log), ancienne copie remise"
      echo "recopie de $name refusée : témoin rouge contre la copie (lot $label) ; appeler c7" > "$D/paused"
      return 1
    fi
    for peer in $PEERS; do
      if scp -q "$CACHE_DIR/$name" "$peer:.cache/rag3weaver-build/$name.neuf" && \
         ssh -o BatchMode=yes "$peer" bash -s "$name" "$label" "$sum" > "$D/last-peer.log" 2>&1 <<'PEER'
cd ~/.cache/rag3weaver-build || exit 1
[ -f "$1" ] && cp -p "$1" "$1.avant-$2"
chmod 755 "$1.neuf" && mv "$1.neuf" "$1" && [ "$(sha256sum "$1" | cut -c1-16)" = "$3" ]
PEER
      then log "$label" "recopié chez $peer" "$name $sum"
      else log "$label" "recopie refusée chez $peer" "$name ($D/last-peer.log)"; echo "recopie de $name chez $peer refusée (lot $label) ; appeler c7" > "$D/paused"; return 1
      fi
    done
    [ "$name" = pusher.sh ] && rc=3
    if [ "$name" = "$PIN_SCRIPT" ]; then
      lib=$(grep -om1 'RAG3DB_LIB=rag3db-build-[0-9a-f]\{40\}[^ ]*' "$CACHE_DIR/$name" | cut -d= -f2)
      if [ -n "$lib" ] && [ "$(cat "$CACHE_DIR/lib-des-passes" 2>/dev/null)" != "$lib" ]; then
        [ -f "$CACHE_DIR/lib-des-passes" ] && cp -p "$CACHE_DIR/lib-des-passes" "$CACHE_DIR/lib-des-passes.avant-$label"
        echo "$lib" > "$CACHE_DIR/lib-des-passes.neuf" && mv "$CACHE_DIR/lib-des-passes.neuf" "$CACHE_DIR/lib-des-passes"
        log "$label" "lib-des-passes" "$lib"
      fi
    fi
  done <<< "$table"
  return $rc
}

step() {
  [ -e "$D/paused" ] && { echo "file en pause : $(cat "$D/paused")"; return 0; }
  [ -e "$HELD" ] && { echo "master tenu : $(cat "$HELD")"; return 0; }
  [ "$(g config user.email)" = "$EMAIL" ] || { echo "identité git du worktree : $(g config user.email)" > "$D/paused"; echo "pause : identité git"; return 1; }
  local -a train=() kept=() part=()
  local line id branch author refs="" n half reposed=0 late="" skipped=" "
  [ -n "$(queue_lines)" ] || { echo "file vide"; return 0; }
  gfetch -q origin master || { log train "fetch de master refusé, rejoué au pas suivant"; return 0; }
  late=$(lib_late)
  if [ -n "$late" ]; then
    [ -s "$D/lib-late" ] || log lib "en retard : $late"
    echo "$late" > "$D/lib-late"
  else
    [ -e "$D/lib-late" ] && { rm -f -- "$D/lib-late"; log lib "à jour : les lots rust reprennent"; }
  fi
  # The train: up to TRAIN_MAX lots in queue order; under a late lib, the Rust lots and their children stay queued.
  # A lot that changes the source of a cache copy closes its train: the lots after it run on the new copy, recopied
  # once the train is pushed (19 Nov.: a pass changed with the lot that needed it would run the old copy).
  local sources closed=""
  sources=$(g show origin/master:tools/poste/cache_copies.txt 2>/dev/null | grep -vE '^[[:space:]]*(#|$)' | awk '{print $1}')
  while IFS= read -r line; do
    id=$(f_ "$line" 1)
    if { [ -n "$late" ] && [ "$(f_ "$line" 4)" = rust ]; } || [[ "$skipped" == *" $(f_ "$line" 8) "* ]]; then
      skipped="$skipped$id "; continue
    fi
    [ -z "$closed" ] && [ ${#train[@]} -lt "$TRAIN_MAX" ] || continue
    train+=("$line")
    branch=$(f_ "$line" 2)
    gfetch -q origin "+refs/heads/$branch:refs/remotes/origin/$branch" 2>/dev/null
    if [ -n "$sources" ] && g diff --name-only "$(g merge-base origin/master "origin/$branch" 2>/dev/null)" "origin/$branch" 2>/dev/null | grep -qxF "$sources"; then
      closed=$id; log "$id" "train fermé" "le lot change une copie du cache"
    fi
  done < <(queue_lines)
  [ ${#train[@]} -gt 0 ] || { echo "lots rust suspendus : lib en retard ($late)"; return 0; }
  for line in "${train[@]}"; do
    branch=$(f_ "$line" 2); refs="$refs +refs/heads/$branch:refs/remotes/origin/$branch"
    log "$(f_ "$line" 1)" pris "$branch $(f_ "$line" 4), train de ${#train[@]}"
  done
  # shellcheck disable=SC2086
  gfetch -q origin master $refs || { echo "fetch du train" > "$D/last-red.txt"; log train "fetch refusé, rejoué au pas suivant"; return 0; }
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
    red_family "$id" "$author" conflit "la repose sur master $(g rev-parse --short "$BASE") bute sur : $(cat "$D/last-conflict.txt" 2>/dev/null) — $(grep -m1 -E 'CONFLICT|error' "$D/last-repose.log")"
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
    0) for line in "${part[@]}"; do pushed "$(f_ "$line" 1)" "$(f_ "$line" 9)" "$(f_ "$line" 2)"; done
       # A refused recopy has paused the queue already; 3 means pusher.sh itself was recopied.
       recopy "$TRAIN_LABEL"; [ $? = 3 ] && return 3 ;;
    1) log "$TRAIN_LABEL" "master a bougé : reposé au pas suivant" ;;
    *) for line in "${part[@]}"; do red_family "$(f_ "$line" 1)" "$(f_ "$line" 9)" garde "$(cat "$D/last-red.txt" 2>/dev/null)"; done ;;
  esac
  return 0
}

case "${1:-}" in
  enqueue) shift; enqueue "$@" ;;
  dequeue) dequeue "$2" ;;
  step) step ;;
  open-prs) open_prs "$2" "$3" "$4" ;;
  pr-catchup) shift; pr_catchup "$@" ;;
  run) while :; do
         out=$(step); rc=$?; echo "$out"
         # pusher.sh recopied by the train it just pushed: the new copy takes over.
         [ $rc = 3 ] && exec bash "$CACHE_DIR/pusher.sh" run
         case "$out" in "file vide"|"file en pause"*|"master tenu"*|"lots rust suspendus"*) sleep "${PUSHER_IDLE:-60}" ;; "pause : "*) exit 1 ;; esac
       done ;;
  *) sed -n '2,20p' "$0"; exit 2 ;;
esac
