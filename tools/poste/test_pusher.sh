#!/bin/bash
# Witness of pusher.sh (lot 1 of the pusher, docs/optimiseur/18-11-2026/02-le-pousseur.md): a throwaway origin, an
# author clone and the pusher's single worktree; the pass and the push guard are replaced by doubles, the guard's
# --code-apporte is the real one. Each case deposits a lot, runs one step, and reads master, the log and the inbox.
# Usage: bash tools/poste/test_pusher.sh [path of pusher.sh] — exit 0 when every case is green.
set -u
PUSHER=$(realpath "${1:-$(dirname "$0")/pusher.sh}")
HERE=$(dirname "$(realpath "$0")")
[ -f "$PUSHER" ] || { echo "ROUGE  pusher.sh absent : $PUSHER"; exit 1; }
# The witness sets its own PUSHER_*: none comes from whoever runs it (19 Nov.: played by the pusher's unit, it took its
# PUSHER_TRAIN_MAX=1, and every train case went red).
unset $(compgen -e | grep '^PUSHER_')
D=$(mktemp -d); trap 'rm -rf "$D"' EXIT
# Git speaks French at luciepc (19 Nov.: « CONFLIT », and a filter on « CONFLICT » named no conflict).
export LANG=fr_FR.UTF-8 LC_ALL=fr_FR.UTF-8
# The submodule case clones a file:// submodule, which git refuses by default.
export GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=protocol.file.allow GIT_CONFIG_VALUE_0=always
red=0
ok() { echo "vert   $1"; }
ko() { echo "ROUGE  $1"; red=1; }

# The origin, seeded with a master that has docs, a poste script and its witness, Rust and engine code.
git init -q --bare -b master "$D/origin.git"
git clone -q "$D/origin.git" "$D/author" 2>/dev/null
cd "$D/author" && git config user.email luciedefraiteur@gmail.com && git config user.name t
mkdir -p docs tools/poste extension/rag3weaver/src src
echo '# notes' > docs/notes.md
printf '# Nouveautés\n\n- le début\n' > docs/nouveautes.md
printf '#!/bin/bash\necho ok\n' > tools/poste/foo.sh
printf '#!/bin/bash\ngrep -q "echo ok" "${1:-$(dirname "$0")/foo.sh}"\n' > tools/poste/test_foo.sh
printf 'tools/poste/foo.sh foo.sh tools/poste/test_foo.sh\ntools/poste/bar.sh bar.sh tools/poste/test_bar.sh\ntools/poste/pin.sh pin.sh tools/poste/test_pin.sh\n' > tools/poste/cache_copies.txt
# pin.sh stands for passe_epinglee.sh: the lib it pins, under the same line the real one has.
printf '#!/bin/bash\nexport RAG3DB_LIB=rag3db-build-%040d-Release\n' 1 > tools/poste/pin.sh
printf '#!/bin/bash\ngrep -q RAG3DB_LIB= "${1:-$(dirname "$0")/pin.sh}"\n' > tools/poste/test_pin.sh
# mv.sh's witness pushes a commit to master while it runs (MOVE_MASTER=docs or code): master moves during a step.
printf '#!/bin/bash\necho mv\n' > tools/poste/mv.sh
cat > tools/poste/test_mv.sh <<'MV'
#!/bin/bash
[ -n "${MOVE_MASTER:-}" ] || exit 0
c=$(mktemp -d); git clone -q "$ORIGIN" "$c/x" 2>/dev/null && cd "$c/x" && git config user.email t@t && git config user.name t
if [ "$MOVE_MASTER" = docs ]; then echo "# moved $(date +%s%N)" > docs/moved-$RANDOM.md; else echo "fn m() {}" > extension/rag3weaver/src/moved$RANDOM.rs; fi
git add -A && git commit -qm "moved by another session" && git push -q origin HEAD:master
MV
# bar.sh's witness goes red against a cache copy when $BREAK_COPY exists: a copy that behaves otherwise at the cache.
printf '#!/bin/bash\necho bar\n' > tools/poste/bar.sh
printf '#!/bin/bash\n[ -n "${1:-}" ] && [ -e "${BREAK_COPY:-/nonexistent}" ] && exit 1\nexit 0\n' > tools/poste/test_bar.sh
cp "$HERE/push_garde.sh" tools/poste/push_garde.sh
echo 'fn main() {}' > extension/rag3weaver/src/lib.rs
mkdir -p extension/rag3weaver/tests && echo '#[test] fn t() {}' > extension/rag3weaver/tests/e2e_code.rs && cp extension/rag3weaver/tests/e2e_code.rs extension/rag3weaver/tests/e2e_code_sync.rs
echo 'int main() {}' > src/engine.cpp
git add -A && git commit -qm base && git push -q origin HEAD:master
BASE0=$(git rev-parse HEAD)
# A lot: lot <branch> <file> <content> — a branch from origin/master (or from $FROM) with one commit, pushed; echoes its head.
lot() {
  git -C "$D/author" fetch -q origin && git -C "$D/author" checkout -q -B "$1" "${FROM:-origin/master}"
  mkdir -p "$D/author/$(dirname "$2")" && printf '%s\n' "$3" > "$D/author/$2"
  git -C "$D/author" add -A && git -C "$D/author" commit -qm "$1" && git -C "$D/author" push -q origin "HEAD:$1"
  git -C "$D/author" rev-parse HEAD
}

# The pusher's single worktree, its folder, and the doubles.
mkdir -p "$D/ws" "$D/journals" "$D/pusher"
git clone -q "$D/origin.git" "$D/ws/wt" 2>/dev/null && git -C "$D/ws/wt" config user.email luciedefraiteur@gmail.com && git -C "$D/ws/wt" config user.name t
git -C "$D/ws/wt" worktree add -q --detach "$D/ws/wt2" 2>/dev/null; mkdir -p "$D/outcomes.d"
# The pass double: one outcome per call, read from $D/outcomes (VERT, ROUGE <step> or 124).
cat > "$D/pass.sh" <<'P'
#!/bin/bash
echo "$*" >> "$PASS_CALLS"
echo "${SUITES_EN_PLUS:-}" >> "$PASS_ENV"
# The codeparsers it plays with, when the worktree has the submodule (19 Nov.: the pass played the old pointer).
[ -e "$PUSHER_WORKSPACES/$1/extension/rag3weaver/codeparsers/.git" ] && echo "$2 $(git -C "$PUSHER_WORKSPACES/$1/extension/rag3weaver/codeparsers" rev-parse HEAD)" >> "$PASS_SUB"
J=$PUSHER_JOURNALS_ROOT/$2; mkdir -p "$J"
echo "commit $(git -C "$PUSHER_WORKSPACES/$1" rev-parse HEAD) ; lib 0000000001" > "$J/resume.log"
echo "RAG3DB_LIB=rag3db-build-$(printf '%040d' 1)-Release" > "$J/lib.log"
# An outcome named for a lot (outcomes.d/<id>) wins over the queue of outcomes: two passes at once have no order.
o=""
for f in "$OUTCOMES_DIR"/*; do
  [ -f "$f" ] || continue
  case "$2" in *-"${f##*/}"|*-"${f##*/}"+*|*-"${f##*/}"-2) o=$(cat "$f"); rm -f "$f"; break ;; esac
done
if [ -z "$o" ]; then o=$(head -1 "$OUTCOMES"); sed -i 1d "$OUTCOMES"; fi
echo "$1 $2 start $(date +%s%N)" >> "$PASS_TIMES"; sleep 1; echo "$1 $2 end $(date +%s%N)" >> "$PASS_TIMES"
case "$o" in
  VERT) echo VERT >> "$J/resume.log" ;;
  124) echo "ROUGE e2e_code (124)" >> "$J/resume.log"; exit 124 ;;
  *) echo "$o" >> "$J/resume.log"; exit 1 ;;
esac
P
echo "# RAG3DB_LIB=rag3db-build-$BASE0-Release" >> "$D/pass.sh"
# The guard double: the real --code-apporte; otherwise the push of the worktree, as the guard ends.
cat > "$D/guard.sh" <<'G'
#!/bin/bash
if [ "$1" = --code-apporte ]; then exec bash "$REAL_GUARD" "$@"; fi
echo "$*" >> "$GUARD_CALLS"
cd "$PUSHER_WORKSPACES/$1" || exit 1
tail -1 "$PUSHER_JOURNALS_ROOT/$2/resume.log" | grep -qx VERT || { echo "ARRÊT : la passe n'est pas VERTE"; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "ARRÊT : arbre sale"; exit 1; }
if [ -n "${PRE_PUSH:-}" ]; then ${PRE_PUSH} "$(git rev-parse HEAD)" || echo "PR absente : le crochet PRE_PUSH a rendu $?"; fi
[ -e "${GUARD_REFUSE:-/nonexistent}" ] && { echo "ARRÊT : push refusé"; exit 1; }
git push -q origin HEAD:master || { echo "ARRÊT : push refusé"; exit 1; }
echo POUSSE
G
mkdir -p "$D/cache" && cp "$D/author/tools/poste/foo.sh" "$D/author/tools/poste/bar.sh" "$D/cache/"
export BREAK_COPY=$D/break-copy
cat > "$D/poste.sh" <<'Q'
#!/bin/bash
# The poste double: `lourd <cmd>` refuses a validation unless POSTE_MEM_MAX=32G is in poste's own environment (the
# real one refuses before its queue, 19 Nov.); FAKE_POSTE_DOWN refuses everything. `push <session> -- <cmd>` runs it.
[ -n "${FAKE_POSTE_DOWN:-}" ] && { echo "poste: refusé (doublure)"; exit 2; }
case "$1" in
  lourd) shift; [ "${POSTE_MEM_MAX:-}" = 32G ] || { echo "poste: refusé avant la file : une validation sous un plafond de 40G"; exit 2; }
         echo "${POSTE_VOIE:-}" >> "$POSTE_VOIES"; exec "$@" ;;
  push) shift 3; exec "$@" ;;
esac
exit 2
Q
chmod +x "$D/poste.sh"
export ORIGIN=$D/origin.git PUSHER_PIN_SCRIPT=pin.sh PUSHER_FETCH_LOCK=$D/fetch.lock PUSHER_CACHE=$D/cache PUSHER_PEERS= PUSHER_DIR=$D/pusher PUSHER_WORKSPACES=$D/ws PUSHER_WT_NAME=wt PUSHER_JOURNALS_ROOT=$D/journals \
  PUSHER_PASS=$D/pass.sh PUSHER_GUARD=$D/guard.sh PUSHER_POSTE=$D/poste.sh PUSHER_MASTER_HELD=$D/master-tenu \
  REAL_GUARD=$HERE/push_garde.sh OUTCOMES=$D/outcomes PASS_CALLS=$D/pass_calls GUARD_CALLS=$D/guard_calls PASS_ENV=$D/pass_env OUTCOMES_DIR=$D/outcomes.d PASS_TIMES=$D/pass_times POSTE_VOIES=$D/poste_voies PASS_SUB=$D/pass_sub
: > "$D/outcomes"; : > "$D/pass_calls"; : > "$D/guard_calls"
# The gh double (one pull request per lot, 19 Nov.): notes each call, and each --body apart; $GH_DOWN existing makes
# every call fail. A pull request it creates is numbered from 41.
cat > "$D/gh.sh" <<'H'
#!/bin/bash
printf '%s\n' "$*" >> "$GH_CALLS"
prev=""; for a in "$@"; do [ "$prev" = --body ] && printf '%s\n----\n' "$a" >> "$GH_BODIES"; prev=$a; done
[ -e "${GH_DOWN:-/nonexistent}" ] && { echo "gh: down (doublure)" >&2; exit 1; }
case "$1 $2" in
  "auth token") echo tok-L ;;
  "api user") [ "${GH_TOKEN:-}" = tok-L ] && echo L-Defraiteur || echo someone-else ;;
  "pr create") echo "https://github.com/L-Defraiteur/rag3db/pull/$(( 40 + $(grep -c '^pr create' "$GH_CALLS") ))" ;;
  "pr comment"|"pr close") ;;
  *) exit 2 ;;
esac
H
chmod +x "$D/gh.sh"
export PUSHER_GH=$D/gh.sh GH_CALLS=$D/gh_calls GH_BODIES=$D/gh_bodies GH_DOWN=$D/gh-down GUARD_REFUSE=$D/guard-refuse
: > "$D/gh_calls"; : > "$D/gh_bodies"
P() { bash "$PUSHER" "$@"; }
master() { git -C "$D/author" fetch -q origin && git -C "$D/author" rev-parse origin/master; }
on_master() { git -C "$D/author" fetch -q origin && git -C "$D/author" log --format=%s origin/master | grep -qx "$1"; }
calls() { grep -c . "$1"; }

# 1. Deposits refused.
h=$(lot docs-1 docs/notes.md '# notes, more')
P enqueue docs-1 docs-1 "$h" docs - - - - author-a > /dev/null && ok "dépôt d'un lot de docs accepté" || ko "dépôt d'un lot de docs refusé"
P enqueue docs-1 docs-1 "$h" docs - - - - author-a > /dev/null 2>&1 && ko "id en double accepté" || ok "id en double refusé"
P enqueue docs-x docs-1 0123456789 docs - - - - author-a > /dev/null 2>&1 && ko "tête autre que celle d'origin acceptée" || ok "tête autre que celle d'origin refusée"
P enqueue docs-y docs-1 "$h" rust some_witness e2e_code - - author-a > /dev/null 2>&1 && ko "kind faux (rust pour des docs) accepté" || ok "kind faux refusé"
hc=$(lot cpp-1 src/engine.cpp 'int main() { return 1; }')
P enqueue cpp-1 cpp-1 "$hc" cpp - - - - author-a > /dev/null 2>&1 && ko "lot C++ accepté" || ok "lot C++ refusé (hors du pousseur)"
P enqueue docs-z docs-1 "$h" docs - - - absent author-a > /dev/null 2>&1 && ko "parent absent de la file accepté" || ok "parent absent de la file refusé"
hw=$(lot rust-w extension/rag3weaver/src/w.rs 'fn w() {}')
out=$(P enqueue rust-w rust-w "$hw" rust some_witness e2e - - author-c 2>&1) && ko "suite du témoin « e2e » acceptée" || { echo "$out" | grep -q 'e2e' && ok "suite du témoin ni lib ni e2e existante : refusée, nommée" || ko "suite du témoin refusée sans la nommer ($out)"; }

hr=$(lot rust-0 extension/rag3weaver/src/lib.rs 'fn main() { let _ = 0; }')
P enqueue rust-0 rust-0 "$hr" rust - e2e - - author-c > /dev/null 2>&1 && ko "lot rust sans témoin accepté" || ok "lot rust sans témoin refusé"

# 2. Paused, then master held: nothing taken.
echo "essai" > "$D/pusher/paused"; m0=$(master)
P step > /dev/null 2>&1; [ "$(master)" = "$m0" ] && ok "file en pause : rien de poussé" || ko "file en pause : master a bougé"
rm "$D/pusher/paused"; echo "fenêtre" > "$D/master-tenu"
P step > /dev/null 2>&1; [ "$(master)" = "$m0" ] && ok "master tenu : rien de poussé" || ko "master tenu : master a bougé"
rm "$D/master-tenu"

# 3. A docs lot: pushed without a pass.
P step > /dev/null 2>&1
on_master docs-1 && [ "$(calls "$D/pass_calls")" = 0 ] && ok "docs poussés sans passe" || ko "docs : pas poussés, ou une passe jouée"
grep -q "^poussé	docs-1	" "$D/pusher/inbox/author-a.txt" 2>/dev/null && ok "boîte de l'auteur : poussé" || ko "boîte de l'auteur sans « poussé docs-1 »"
[ -z "$(grep -v '^#' "$D/pusher/queue.tsv")" ] && ok "file vide après le push" || ko "le lot est resté dans la file"
# The docs lot's pull request (19 Nov.): opened before the push, from pusher/<stamp>-docs-1-final to master, its title
# « <id> : <subject> », a body with no attribution; GitHub marks it merged by the fast-forward; « poussé » says its number.
pr=$(grep -m1 '^pr create' "$D/gh_calls")
fin=$(git -C "$D/author" ls-remote origin 'refs/heads/pusher/*-docs-1-final' | cut -f1)
[[ "$pr" == *"--base master"* ]] && [[ "$pr" == *"--head pusher/"*"-docs-1-final"* ]] && [[ "$pr" == *"--title docs-1 : docs-1"* ]] \
  && [ -n "$fin" ] && [ "$fin" = "$(master)" ] && ok "PR du lot de docs : pusher/…-docs-1-final à sa pointe, vers master, titre « id : sujet »" \
  || ko "PR du lot de docs absente ou fausse (« $pr », branche $fin)"
! grep -qiE 'claude|co-authored|generated' "$D/gh_bodies" && grep -q 'Lib : ' "$D/gh_bodies" && ok "corps de PR : lib, témoin, sans ligne d'attribution" || ko "corps de PR : attribution, ou lib absente"
grep -q "	docs-1	PR 41 ouverte	" "$D/pusher/log.tsv" && grep -q "^poussé	docs-1	.*	PR 41$" "$D/pusher/inbox/author-a.txt" \
  && grep -q '^pr comment 41 ' "$D/gh_calls" && ok "« PR 41 ouverte » au journal, son numéro dans « poussé », les trois contrôles en commentaire" || ko "PR 41 : journal, boîte ou commentaire manquant"

# 4. A scripts lot, witness green; then one whose witness is red.
h=$(lot scripts-1 tools/poste/foo.sh '#!/bin/bash
echo ok; echo more')
P enqueue scripts-1 scripts-1 "$h" scripts - - - - author-b > /dev/null && P step > /dev/null 2>&1
on_master scripts-1 && ok "scripts : témoin vert, poussé" || ko "scripts au témoin vert : pas poussé"
grep -q 'echo more' "$D/cache/foo.sh" && [ -f "$D/cache/foo.sh.avant-scripts-1" ] && grep -q "	scripts-1	recopié	foo.sh .*témoin vert" "$D/pusher/log.tsv" \
  && ok "recopie au cache : copie neuve, ancienne gardée, témoin joué contre la copie" || ko "recopie au cache : absente ou incomplète"
h=$(lot bar-1 tools/poste/bar.sh '#!/bin/bash
echo bar, new')
touch "$D/break-copy"
P enqueue bar-1 bar-1 "$h" scripts - - - - author-b > /dev/null && P step > /dev/null 2>&1
on_master bar-1 && ! grep -q 'bar, new' "$D/cache/bar.sh" && grep -q 'recopie de bar.sh' "$D/pusher/paused" 2>/dev/null \
  && ok "témoin rouge contre la copie : ancienne copie remise, file en pause" || ko "témoin rouge contre la copie : copie neuve gardée, ou file pas en pause"
rm -f "$D/break-copy" "$D/pusher/paused"
h=$(lot scripts-2 tools/poste/foo.sh '#!/bin/bash
echo broken')
P enqueue scripts-2 scripts-2 "$h" scripts - - - - author-b > /dev/null && P step > /dev/null 2>&1
! on_master scripts-2 && grep -q "^rouge	scripts-2	code	" "$D/pusher/inbox/author-b.txt" && ok "scripts : témoin rouge, rendu à l'auteur" || ko "scripts au témoin rouge : poussé, ou pas rendu"
[ -z "$(grep -v '^#' "$D/pusher/queue.tsv")" ] && ok "rouge sorti de la file" || ko "rouge resté dans la file"

# 5. A Rust lot: one pass, then the guard.
h=$(lot rust-1 extension/rag3weaver/src/lib.rs 'fn main() { println!(); }')
echo VERT >> "$D/outcomes"
P enqueue rust-1 rust-1 "$h" rust some_witness e2e_code - - author-c > /dev/null && P step > /dev/null 2>&1
on_master rust-1 && [ "$(calls "$D/pass_calls")" = 1 ] && [ "$(calls "$D/guard_calls")" = 1 ] && ok "rust : une passe, la garde, poussé" || ko "rust : pas poussé, ou passe ou garde manquante"
grep -q "	rust-1	.*peak=" "$D/pusher/log.tsv" && ok "pic mémoire du rejeu au journal" || ko "pas de pic mémoire au journal"
grep -qx pousseur "$D/poste_voies" 2>/dev/null && ! grep -vqx pousseur "$D/poste_voies" && ok "la passe du pousseur prend la voie du pousseur (POSTE_VOIE)" || ko "la passe du pousseur ne demande pas sa voie ($(sort -u "$D/poste_voies" 2>/dev/null | tr '\n' ' '))"

# 6. A Rust lot red on code: returned, no guard.
h=$(lot rust-2 extension/rag3weaver/src/lib.rs 'fn main() { panic!(); }')
echo "ROUGE e2e_code" >> "$D/outcomes"
P enqueue rust-2 rust-2 "$h" rust some_witness e2e_code - - author-c > /dev/null && P step > /dev/null 2>&1
! on_master rust-2 && [ "$(calls "$D/guard_calls")" = 1 ] && grep -q "^rouge	rust-2	code	" "$D/pusher/inbox/author-c.txt" && ok "rust rouge de code : rendu, sans garde" || ko "rust rouge de code : mal traité"

# 7. Environment red: replayed once, then green; twice: returned as environment.
h=$(lot rust-3 extension/rag3weaver/src/lib.rs 'fn main() { let _ = 3; }')
printf '124\nVERT\n' >> "$D/outcomes"
P enqueue rust-3 rust-3 "$h" rust some_witness e2e_code - - author-c > /dev/null && P step > /dev/null 2>&1
on_master rust-3 && [ "$(calls "$D/pass_calls")" = 4 ] && ok "rouge d'environnement rejoué une fois, puis poussé" || ko "rouge d'environnement : pas rejoué, ou pas poussé"
h=$(lot rust-4 extension/rag3weaver/src/lib.rs 'fn main() { let _ = 4; }')
printf '124\n124\n' >> "$D/outcomes"
P enqueue rust-4 rust-4 "$h" rust some_witness e2e_code - - author-c > /dev/null && P step > /dev/null 2>&1
! on_master rust-4 && grep -q "^rouge	rust-4	environnement	" "$D/pusher/inbox/author-c.txt" && ok "deux rouges d'environnement : rendu comme environnement" || ko "deux rouges d'environnement : mal traités"

# 8. A conflict at the repose: returned, master unchanged.
h=$(FROM=$BASE0 lot docs-2 docs/notes.md '# notes, the other way')
P enqueue docs-2 docs-2 "$h" docs - - - - author-a > /dev/null; m0=$(master)
P step > /dev/null 2>&1
[ "$(master)" = "$m0" ] && grep -q "^rouge	docs-2	conflit	.*docs/notes.md" "$D/pusher/inbox/author-a.txt" && ok "conflit à la repose : rendu à l'auteur, nommé" || ko "conflit à la repose : mal traité, ou sans son nom"
grep -q "^docs-2	" "$D/pusher/returned.tsv" 2>/dev/null && ok "la ligne rendue est gardée (returned.tsv)" || ko "la ligne rendue est perdue"

# 9. A branch moved after its deposit: returned.
h=$(lot docs-3 docs/other.md '# other')
P enqueue docs-3 docs-3 "$h" docs - - - - author-a > /dev/null
FROM=docs-3 lot docs-3 docs/other.md '# other, moved' > /dev/null
P step > /dev/null 2>&1
! on_master docs-3 && grep -q "^rouge	docs-3	tête bougée	" "$D/pusher/inbox/author-a.txt" && ok "branche bougée après le dépôt : rendue" || ko "branche bougée après le dépôt : poussée ou pas rendue"

# 10. The repose is under a new name, pushed to origin; the log has one line per event.
git -C "$D/author" ls-remote -q origin 'refs/heads/pusher/*' | grep -q . && ok "repose poussée sous un nom neuf (pusher/…)" || ko "aucune branche pusher/… sur origin"
grep -q "^.*	docs-1	pris" "$D/pusher/log.tsv" && grep -q "	docs-1	poussé	" "$D/pusher/log.tsv" && ok "journal : pris puis poussé" || ko "journal incomplet"

# Lot 2, the train. 11. Three docs lots deposited before one step: one step pushes all three, no pass.
c0=$(calls "$D/pass_calls")
h1=$(lot tr-a docs/a.md '# a'); h2=$(lot tr-b docs/b.md '# b'); h3=$(lot tr-c docs/c.md '# c')
for x in "tr-a $h1" "tr-b $h2" "tr-c $h3"; do set -- $x; P enqueue "$1" "$1" "$2" docs - - - - author-a > /dev/null; done
P step > /dev/null 2>&1
on_master tr-a && on_master tr-b && on_master tr-c && [ "$(calls "$D/pass_calls")" = "$c0" ] && ok "train de trois lots de docs poussé en un pas" || ko "train de docs : pas tout poussé en un pas"

# 12. A Rust train of four, the third red: the train is red, its first half green and pushed; then C,D red, C alone
# red and returned; then D pushed. Five passes in all, A, B and D on master, C returned.
c0=$(calls "$D/pass_calls")
ha=$(lot bi-a extension/rag3weaver/src/a.rs 'fn a() {}'); hb=$(lot bi-b extension/rag3weaver/src/b.rs 'fn b() {}')
hc=$(lot bi-c extension/rag3weaver/src/c.rs 'fn c() {}'); hd=$(lot bi-d extension/rag3weaver/src/d.rs 'fn d() {}')
for x in "bi-a $ha" "bi-b $hb" "bi-c $hc" "bi-d $hd"; do set -- $x; P enqueue "$1" "$1" "$2" rust some_witness e2e_code - - author-c > /dev/null; done
printf 'ROUGE e2e_code
VERT
ROUGE e2e_code
ROUGE e2e_code
VERT
' >> "$D/outcomes"
P step > /dev/null 2>&1; P step > /dev/null 2>&1; P step > /dev/null 2>&1
on_master bi-a && on_master bi-b && on_master bi-d && ! on_master bi-c && [ "$(( $(calls "$D/pass_calls") - c0 ))" = 5 ] \
  && grep -q "^rouge	bi-c	code	" "$D/pusher/inbox/author-c.txt" && ok "bissection : A, B, D poussés, C rendu, cinq passes" || ko "bissection : mauvais lots poussés ou mauvais nombre de passes ($(( $(calls "$D/pass_calls") - c0 )))"

# 13. A conflict in the middle of a train: the middle lot returned, the two others pushed in the same step.
m0=$(master)
h1=$(lot cf-a docs/notes.md '# notes, by a'); h2=$(FROM=$m0 lot cf-b docs/notes.md '# notes, a third way'); h3=$(lot cf-c docs/cf-c.md '# c')
for x in "cf-a $h1" "cf-b $h2" "cf-c $h3"; do set -- $x; P enqueue "$1" "$1" "$2" docs - - - - author-a > /dev/null; done
P step > /dev/null 2>&1
on_master cf-a && on_master cf-c && ! on_master cf-b && grep -q "^rouge	cf-b	conflit	" "$D/pusher/inbox/author-a.txt" && ok "conflit au milieu du train : rendu, les deux autres poussés" || ko "conflit au milieu du train : mal traité"

# 14. The child of a red lot is returned with it.
hp=$(lot pa-1 extension/rag3weaver/src/p.rs 'fn p() {}')
hk=$(FROM=pa-1 lot pa-2 extension/rag3weaver/src/k.rs 'fn k() {}')
P enqueue pa-1 pa-1 "$hp" rust some_witness e2e_code - - author-c > /dev/null; P enqueue pa-2 pa-2 "$hk" rust some_witness e2e_code - pa-1 author-c > /dev/null
printf 'ROUGE e2e_code\nROUGE e2e_code\n' >> "$D/outcomes"
P step > /dev/null 2>&1
grep -q "^rouge	pa-1	code	" "$D/pusher/inbox/author-c.txt" && grep -q "^rouge	pa-2	parent rouge	" "$D/pusher/inbox/author-c.txt" && [ -z "$(grep -v '^#' "$D/pusher/queue.tsv")" ] \
  && ok "l'enfant d'un lot rouge est rendu avec lui" || ko "l'enfant d'un lot rouge : pas rendu, ou resté dans la file"

# Lot 3. 15. C++ lands on master after the pinned lib: a Rust lot waits in the queue without a pass, a docs lot
# deposited behind it passes; after the bascule (the pass names a lib on the new master), the Rust lot passes.
git -C "$D/author" fetch -q origin && git -C "$D/author" checkout -q -B cpp-lands origin/master
echo 'int main() { return 2; }' > "$D/author/src/engine.cpp"
git -C "$D/author" commit -qam cpp-lands && git -C "$D/author" push -q origin HEAD:master
hr=$(lot late-r extension/rag3weaver/src/late.rs 'fn late() {}'); hd=$(lot late-d docs/late.md '# late')
P enqueue late-r late-r "$hr" rust some_witness e2e_code - - author-c > /dev/null; P enqueue late-d late-d "$hd" docs - - - - author-a > /dev/null
c0=$(calls "$D/pass_calls")
P step > /dev/null 2>&1
on_master late-d && ! on_master late-r && [ "$(calls "$D/pass_calls")" = "$c0" ] && grep -q "^late-r	" "$D/pusher/queue.tsv" && [ -s "$D/pusher/lib-late" ] \
  && ok "lib en retard : lot rust suspendu dans la file, docs poussés, c7 prévenu (lib-late)" || ko "lib en retard : lot rust joué ou sorti, ou docs bloqués"
sed -i "s/rag3db-build-$BASE0-/rag3db-build-$(master)-/" "$D/pass.sh"
echo VERT >> "$D/outcomes"
P step > /dev/null 2>&1
on_master late-r && [ ! -e "$D/pusher/lib-late" ] && ok "après la bascule : le lot rust passe, lib-late retiré" || ko "après la bascule : lot rust pas poussé, ou lib-late resté"

# 16. Two lots each adding their line at the head of docs/nouveautes.md, from the same base: one train, both pushed,
# both lines on master (union, as a session resolves it by hand).
m0=$(master)
h1=$(FROM=$m0 lot nv-a docs/nouveautes.md '# Nouveautés

- la ligne de a
- le début'); h2=$(FROM=$m0 lot nv-b docs/nouveautes.md '# Nouveautés

- la ligne de b
- le début')
P enqueue nv-a nv-a "$h1" docs - - - - author-a > /dev/null; P enqueue nv-b nv-b "$h2" docs - - - - author-a > /dev/null
P step > /dev/null 2>&1
git -C "$D/author" fetch -q origin
on_master nv-a && on_master nv-b && git -C "$D/author" show origin/master:docs/nouveautes.md | grep -q 'la ligne de a' && git -C "$D/author" show origin/master:docs/nouveautes.md | grep -q 'la ligne de b' \
  && ok "nouveautes.md : les deux lignes, un train" || ko "nouveautes.md : un lot rendu, ou une ligne perdue"

# 17. Poste refuses the pass (no summary written): an environment red, never a code red.
h=$(lot env-poste extension/rag3weaver/src/envp.rs 'fn envp() {}')
P enqueue env-poste env-poste "$h" rust some_witness e2e_code - - author-c > /dev/null
c0=$(calls "$D/pass_calls")
FAKE_POSTE_DOWN=1 P step > /dev/null 2>&1
! on_master env-poste && [ "$(calls "$D/pass_calls")" = "$c0" ] && grep -q "^rouge	env-poste	environnement	" "$D/pusher/inbox/author-c.txt" \
  && ok "passe refusée par poste : rouge d'environnement" || ko "passe refusée par poste : comptée rouge de code, ou jouée"

# 18. A Rust train of two: the second lot's witness is a test of the suite e2e_code_sync. The pass gets the suite in
# its extra suites, never the test's name (19 Nov.: a test name given as a suite).
h1=$(lot sx-a extension/rag3weaver/src/sxa.rs 'fn sxa() {}'); h2=$(lot sx-b extension/rag3weaver/src/sxb.rs 'fn sxb() {}')
P enqueue sx-a sx-a "$h1" rust some_lib_test lib - - author-c > /dev/null
P enqueue sx-b sx-b "$h2" rust usages_does_not_show_the_declaration e2e_code_sync e2e_code - author-c > /dev/null
echo VERT >> "$D/outcomes"
P step > /dev/null 2>&1
x=$(tail -1 "$D/pass_env" | tr " " ",")
on_master sx-a && on_master sx-b && [[ ",$x," == *,e2e_code_sync,* ]] && [[ ",$x," == *,e2e_code,* ]] && [[ "$x" != *usages_does_not* ]] \
  && ok "train rust : la suite du second lot en plus, pas son témoin ($x)" || ko "train rust : suites en plus fausses ($x)"

# 19. A lot that changes a cache copy's source closes its train: the docs lot behind it waits for the next step,
# which runs on the new copy.
h1=$(lot cc-a tools/poste/foo.sh '#!/bin/bash
echo ok; echo cut'); h2=$(lot cc-b docs/cc.md '# cc')
P enqueue cc-a cc-a "$h1" scripts - - - - author-b > /dev/null; P enqueue cc-b cc-b "$h2" docs - - - - author-a > /dev/null
P step > /dev/null 2>&1
on_master cc-a && ! on_master cc-b && grep -q 'echo cut' "$D/cache/foo.sh" && ok "un lot qui change une copie du cache ferme son train" || ko "le lot d'après est parti dans le même train"
P step > /dev/null 2>&1
on_master cc-b && ok "le lot d'après passe au pas suivant" || ko "le lot d'après n'est pas passé"

# 20. A fetch that meets a ref lock held a few seconds (another fetch in the shared repository, 19 Nov.): retried, the
# lot passes; never returned.
h=$(lot fl-a docs/fl.md '# fl')
P enqueue fl-a fl-a "$h" docs - - - - author-a > /dev/null
gd=$(git -C "$D/ws/wt" rev-parse --git-common-dir); case "$gd" in /*) ;; *) gd=$D/ws/wt/$gd ;; esac
# Master moves first (another session's push), so the fetch must write the ref and meets the lock.
FROM=origin/master lot fl-other docs/fl-other.md '# other' > /dev/null && git -C "$D/author" push -q origin fl-other:master
mkdir -p "$gd/refs/remotes/origin" && touch "$gd/refs/remotes/origin/master.lock"
( sleep 4; rm -f "$gd/refs/remotes/origin/master.lock" ) &
P step > /dev/null 2>&1; wait
on_master fl-a && ! grep -q "^rouge	fl-a	" "$D/pusher/inbox/author-a.txt" && ok "fetch sur un verrou de ref : retenté dans le même pas, le lot passe" || ko "fetch sur un verrou de ref : lot rendu ou pas passé"
grep -q "	fl-a	.*\|	train	fetch" "$D/pusher/log.tsv" && grep -q "verrou de ref" "$D/pusher/log.tsv" && ok "le journal dit le verrou de ref retenté" || ko "le journal ne dit pas le verrou de ref"

# 21. A child lot written on a parent that landed reposed (new hashes): the parent's first commit adds a line that its
# second commit rewrites; picking the parent's commits again would conflict. Commits are chosen by patch (19 Nov.:
# rangement-lot-2 and memoire-turns returned « conflit » after pile-archi-19 landed).
git -C "$D/author" fetch -q origin && git -C "$D/author" checkout -q -B ch-p origin/master
echo 'step one' > "$D/author/docs/ch.md" && git -C "$D/author" add -A && git -C "$D/author" commit -qm "ch-p 1"
echo 'step two' > "$D/author/docs/ch.md" && git -C "$D/author" commit -qam "ch-p 2" && git -C "$D/author" push -q origin HEAD:ch-p
hp=$(git -C "$D/author" rev-parse HEAD)
hk=$(FROM=ch-p lot ch-k docs/ch-k.md '# child')
FROM=origin/master lot ch-move docs/ch-move.md '# master moves' > /dev/null && git -C "$D/author" push -q origin ch-move:master
P enqueue ch-p ch-p "$hp" docs - - - - author-a > /dev/null; P enqueue ch-k ch-k "$hk" docs - - - ch-p author-a > /dev/null
P step > /dev/null 2>&1; P step > /dev/null 2>&1
on_master "ch-p 2" && on_master ch-k && ok "l'enfant d'un parent reposé passe (commits choisis par patch)" || ko "l'enfant d'un parent reposé bute ($(grep -P '\tch-k\t' "$D/pusher/log.tsv" | tail -1 | cut -f3-))"

# 22. A lot holding a commit empty from its origin (git commit --allow-empty): kept, not a conflict (19 Nov.: pile-arbre-2a3).
git -C "$D/author" fetch -q origin && git -C "$D/author" checkout -q -B em-a origin/master
git -C "$D/author" commit -q --allow-empty -m "em-a marker" && echo '# em' > "$D/author/docs/em.md" && git -C "$D/author" add -A && git -C "$D/author" commit -qm "em-a" && git -C "$D/author" push -q origin HEAD:em-a
P enqueue em-a em-a "$(git -C "$D/author" rev-parse HEAD)" docs - - - - author-a > /dev/null
P step > /dev/null 2>&1
on_master em-a && ok "un commit vide à l'origine passe" || ko "un commit vide à l'origine bute ($(grep -P '\tem-a\t' "$D/pusher/log.tsv" | tail -1 | cut -f3-))"

# 23. A child deposited after its parent landed, the parent's nouveautes line merged by union (so its patch changed): the
# child's repose leaves out the parent's original commits by its original head (landed.tsv), never by patch alone — the
# line would come twice (19 Nov., c7's rule: rebase --onto the parent's original head).
m0=$(master)
FROM=$m0 lot un-m docs/nouveautes.md '# Nouveautés

- la ligne de master
- le début' > /dev/null && git -C "$D/author" push -q origin un-m:master
hp=$(FROM=$m0 lot un-p docs/nouveautes.md '# Nouveautés

- la ligne du parent
- le début')
hk=$(FROM=un-p lot un-k docs/un-k.md '# child of un-p')
P enqueue un-p un-p "$hp" docs - - - - author-a > /dev/null
P step > /dev/null 2>&1
P enqueue un-k un-k "$hk" docs - - - un-p author-a > /dev/null 2>&1 || echo "dépôt de l'enfant d'un parent atterri refusé" >> "$D/un-k.refus"
P step > /dev/null 2>&1
git -C "$D/author" fetch -q origin
n=$(git -C "$D/author" show origin/master:docs/nouveautes.md | grep -c 'la ligne du parent')
[ ! -e "$D/un-k.refus" ] && on_master un-p && on_master un-k && [ "$n" = 1 ] && ok "l'enfant d'un parent atterri se repose sur ses seuls commits (ligne du parent une fois)" || ko "l'enfant d'un parent atterri : refusé, absent, ou la ligne du parent $n fois"

# 24. A lot that changes the pinned lib: once recopied, lib-des-passes says that lib, the old file kept (19 Nov., relevé de
# mémoire: lib-des-passes said 59ac9c86b, two bascules late, and tour.sh reads it first).
cp "$D/author/tools/poste/pin.sh" "$D/cache/pin.sh"; echo rag3db-build-old > "$D/cache/lib-des-passes"
newlib=rag3db-build-$(printf '%040d' 2)-Release
h=$(lot pin-2 tools/poste/pin.sh "#!/bin/bash
export RAG3DB_LIB=$newlib")
P enqueue pin-2 pin-2 "$h" scripts - - - - author-b > /dev/null && P step > /dev/null 2>&1
on_master pin-2 && [ "$(cat "$D/cache/lib-des-passes")" = "$newlib" ] && [ "$(cat "$D/cache/lib-des-passes.avant-pin-2" 2>/dev/null)" = rag3db-build-old ] \
  && ok "après une épingle, lib-des-passes dit la lib épinglée (ancien gardé)" || ko "lib-des-passes : $(cat "$D/cache/lib-des-passes")"
# 25. One pull request per lot in a train: each from its own tip on the final head.
h1=$(lot pt-a docs/pt-a.md '# pt-a'); h2=$(lot pt-b docs/pt-b.md '# pt-b')
P enqueue pt-a pt-a "$h1" docs - - - - author-a > /dev/null; P enqueue pt-b pt-b "$h2" docs - - - - author-a > /dev/null
P step > /dev/null 2>&1
fa=$(git -C "$D/author" ls-remote origin 'refs/heads/pusher/*-pt-a-final' | cut -f1); fb=$(git -C "$D/author" ls-remote origin 'refs/heads/pusher/*-pt-b-final' | cut -f1)
on_master pt-a && on_master pt-b && [ -n "$fa" ] && [ "$fb" = "$(master)" ] && [ "$(git -C "$D/author" log -1 --format=%s "$fa")" = pt-a ] \
  && grep -q "	pt-a	PR [0-9]* ouverte	" "$D/pusher/log.tsv" && grep -q "	pt-b	PR [0-9]* ouverte	" "$D/pusher/log.tsv" \
  && ok "train de deux lots : deux PR, chacune de la pointe de son lot" || ko "train de deux lots : PR absentes ou pointes fausses ($fa / $fb)"

# 26. A Rust lot: the pull request opens through the guard (PRE_PUSH), on the final head.
h=$(lot pr-r extension/rag3weaver/src/prr.rs 'fn prr() {}')
echo VERT >> "$D/outcomes"
P enqueue pr-r pr-r "$h" rust some_witness e2e_code - - author-c > /dev/null && P step > /dev/null 2>&1
fr=$(git -C "$D/author" ls-remote origin 'refs/heads/pusher/*-pr-r-final' | cut -f1)
on_master pr-r && [ "$fr" = "$(master)" ] && grep -q "	pr-r	PR [0-9]* ouverte	" "$D/pusher/log.tsv" && ok "lot rust : la PR s'ouvre par la garde, sur la tête finale" || ko "lot rust : pas de PR par la garde ($fr)"
grep -q "Lot pr-r " "$D/gh_bodies" && awk '/^Lot pr-r /{f=1} f && /^- Lib : /{print; exit}' "$D/gh_bodies" | grep -q "rag3db-build-$(printf '%040d' 1)" \
  && ok "corps de la PR rust : la lib en forme longue, lue dans les journaux de la passe" || ko "corps de la PR rust : lib absente ($(awk '/^Lot pr-r /{f=1} f && /^- Lib : /{print; exit}' "$D/gh_bodies"))"

# 27. gh down: the lot is pushed all the same, « PR absente » in the log.
touch "$D/gh-down"
h=$(lot pr-down docs/pr-down.md '# down')
P enqueue pr-down pr-down "$h" docs - - - - author-a > /dev/null && P step > /dev/null 2>&1
rm -f "$D/gh-down"
on_master pr-down && grep -q "	pr-down	PR absente : " "$D/pusher/log.tsv" && ok "gh en panne : le lot est poussé, « PR absente » au journal" || ko "gh en panne : lot bloqué, ou pas de « PR absente »"

# 27 bis. A push refused after the pull requests opened (G, 19 Nov.): they are closed, said in the log, and nothing of
# theirs stays open. Docs refused by poste: the lot is returned; Rust refused by the guard: it stays, reposed next step.
h=$(lot pr-o docs/pr-o.md '# orphan')
P enqueue pr-o pr-o "$h" docs - - - - author-a > /dev/null && FAKE_POSTE_DOWN=1 P step > /dev/null 2>&1
no=$(grep "	pr-o	PR [0-9]* ouverte	" "$D/pusher/log.tsv" | tail -1 | sed -E 's/.*	PR ([0-9]+) ouverte	.*/\1/')
! on_master pr-o && [ -n "$no" ] && grep -q "^pr close $no .*--comment" "$D/gh_calls" && grep -q "	pr-o	PR $no fermée	" "$D/pusher/log.tsv" \
  && ok "push refusé après l'ouverture : la PR fermée, dite au journal" || ko "push refusé après l'ouverture : PR $no laissée ouverte"
h=$(lot pr-g extension/rag3weaver/src/prg.rs 'fn prg() {}')
printf 'VERT\nVERT\n' >> "$D/outcomes"; touch "$D/guard-refuse"
P enqueue pr-g pr-g "$h" rust some_witness e2e_code - - author-c > /dev/null && P step > /dev/null 2>&1
rm -f "$D/guard-refuse"; P step > /dev/null 2>&1
opened=$(grep -c "	pr-g	PR [0-9]* ouverte	" "$D/pusher/log.tsv"); closed=$(grep -c "	pr-g	PR [0-9]* fermée	" "$D/pusher/log.tsv")
on_master pr-g && [ "$opened" = 2 ] && [ "$closed" = 1 ] && ok "la garde refuse après PRE_PUSH : la PR fermée, une seule neuve au pas suivant" || ko "garde refusée : $opened ouvertes, $closed fermées"

# 28. The catch-up: a -base branch at the master of before, the pull request from the -final to it, closed with where it
# landed; no ref deleted.
before=$(git -C "$D/author" rev-parse "$(master)~1"); tip=$(master)
P pr-catchup pr-down "$before" "$tip" author-a "pr-down" > /dev/null 2>&1
cb=$(git -C "$D/author" ls-remote origin 'refs/heads/pusher/*-pr-down-base' | cut -f1)
[ "$cb" = "$before" ] && grep -q -- "--base pusher/.*-pr-down-base --head pusher/.*-pr-down-final" "$D/gh_calls" \
  && grep -q "^pr close [0-9]* .*--comment atterri sur master à $tip" "$D/gh_calls" && ok "rattrapage : -base au master d'avant, PR de la finale vers elle, fermée « atterri sur master »" || ko "rattrapage faux ($cb)"
grep -vE '^[[:space:]]*#' "$PUSHER" | grep -qE 'push[^|;]*(--delete|[[:space:]]:refs/)' && ko "le pousseur efface une ref" || ok "aucune ref effacée par le pousseur"


# The second slot (19 Nov., c7 and G). 25. Two Rust lots, trains of one, two slots: the second is reposed on the first's
# head in the second worktree and replayed at the same time; both green, both pushed in one step.
h1=$(lot sl-a extension/rag3weaver/src/sla.rs 'fn sla() {}'); h2=$(FROM=sl-a lot sl-b extension/rag3weaver/src/slb.rs 'fn slb() {}')
P enqueue sl-a sl-a "$h1" rust some_witness e2e_code - - author-c > /dev/null; P enqueue sl-b sl-b "$h2" rust some_witness e2e_code - sl-a author-c > /dev/null
echo VERT > "$D/outcomes.d/sl-a"; echo VERT > "$D/outcomes.d/sl-b"
PUSHER_SLOTS=2 PUSHER_TRAIN_MAX=1 PUSHER_WT2_NAME=wt2 P step > /dev/null 2>&1
ea=$(awk '$2 ~ /-sl-a$/ && $3 == "end" {print $4}' "$D/pass_times")
sb=$(awk '$2 ~ /-sl-b/ && $3 == "start" {print $4}' "$D/pass_times"); wb=$(awk '$2 ~ /-sl-b/ && $3 == "start" {print $1}' "$D/pass_times")
on_master sl-a && on_master sl-b && [ "$wb" = wt2 ] && [ -n "$sb" ] && [ -n "$ea" ] && [ "$sb" -lt "$ea" ] \
  && ok "second créneau : B reposé sur A dans wt2, rejoué en même temps, les deux poussés en un pas" || ko "second créneau : A ou B pas poussé, ou B pas rejoué dans wt2 en même temps (wt=$wb)"

# 26. A red, B speculative on A: A returned, B thrown away but kept in the queue (never returned), then replayed alone.
h1=$(lot sr-a extension/rag3weaver/src/sra.rs 'fn sra() {}'); h2=$(lot sr-b extension/rag3weaver/src/srb.rs 'fn srb() {}')
P enqueue sr-a sr-a "$h1" rust some_witness e2e_code - - author-c > /dev/null; P enqueue sr-b sr-b "$h2" rust some_witness e2e_code - - author-c > /dev/null
echo "ROUGE e2e_code" > "$D/outcomes.d/sr-a"; echo VERT > "$D/outcomes.d/sr-b"
PUSHER_SLOTS=2 PUSHER_TRAIN_MAX=1 PUSHER_WT2_NAME=wt2 P step > /dev/null 2>&1
q1=$(grep -c "^sr-b	" "$D/pusher/queue.tsv")
echo VERT > "$D/outcomes.d/sr-b"
PUSHER_SLOTS=2 PUSHER_TRAIN_MAX=1 PUSHER_WT2_NAME=wt2 P step > /dev/null 2>&1
! on_master sr-a && grep -q "^rouge	sr-a	code	" "$D/pusher/inbox/author-c.txt" && [ "$q1" = 1 ] && ! grep -q "^rouge	sr-b	" "$D/pusher/inbox/author-c.txt" && on_master sr-b \
  && ok "A rouge : B spéculatif jeté sans être rendu, puis rejoué seul et poussé" || ko "A rouge : B rendu, perdu ou pas repris (B en file après le pas 1 : $q1)"

# 27. A green, B red: A pushed, B kept in the queue (not returned) for its own replay.
h1=$(lot sg-a extension/rag3weaver/src/sga.rs 'fn sga() {}'); h2=$(lot sg-b extension/rag3weaver/src/sgb.rs 'fn sgb() {}')
P enqueue sg-a sg-a "$h1" rust some_witness e2e_code - - author-c > /dev/null; P enqueue sg-b sg-b "$h2" rust some_witness e2e_code - - author-c > /dev/null
echo VERT > "$D/outcomes.d/sg-a"; echo "ROUGE e2e_code" > "$D/outcomes.d/sg-b"
PUSHER_SLOTS=2 PUSHER_TRAIN_MAX=1 PUSHER_WT2_NAME=wt2 P step > /dev/null 2>&1
on_master sg-a && ! on_master sg-b && grep -q "^sg-b	" "$D/pusher/queue.tsv" && ! grep -q "^rouge	sg-b	" "$D/pusher/inbox/author-c.txt" \
  && ok "A vert, B rouge : A poussé, B gardé en file pour son propre rejeu" || ko "A vert, B rouge : mal traité"
P dequeue sg-b > /dev/null 2>&1

# 28. Master moves by docs while a scripts lot replays (another session pushes a page): the lot lands in the same step,
# reposed on the new master, never sent back to the next step (19 Nov.: pusher-second-slot was taken three times).
h=$(lot mv-d tools/poste/mv.sh '#!/bin/bash
echo mv docs')
P enqueue mv-d mv-d "$h" scripts - - - - author-b > /dev/null
MOVE_MASTER=docs P step > /dev/null 2>&1
on_master mv-d && [ "$(grep -cP '\tmv-d\tpris' "$D/pusher/log.tsv")" = 1 ] && ok "master bouge en docs pendant le pas : le lot atterrit dans le même pas" || ko "master bouge en docs : le lot n'a pas atterri dans le pas ($(grep -P '\tmv-d\t' "$D/pusher/log.tsv" | tail -1 | cut -f3))"

# 29. Master moves by code while a scripts lot replays: the lot stays at the head of the queue, replayed next step.
h=$(lot mv-c tools/poste/mv.sh '#!/bin/bash
echo mv code')
P enqueue mv-c mv-c "$h" scripts - - - - author-b > /dev/null
MOVE_MASTER=code P step > /dev/null 2>&1
! on_master mv-c && grep -q "^mv-c	" "$D/pusher/queue.tsv" && ok "master bouge en code pendant le pas : le lot reste en tête pour un rejeu" || ko "master bouge en code : le lot est parti sans rejeu"
# The next step, in the same second: its repose goes under a name of its own, and the lot lands (19 Nov.: two steps in
# one second reposed under the same name, the push of the second repose was refused, the lot looped).
P step > /dev/null 2>&1
on_master mv-c && ok "le pas suivant, dans la même seconde, repose sous un nom neuf et pousse" || ko "le pas suivant n'a pas poussé ($(grep -P '\tmv-c\t' "$D/pusher/log.tsv" | tail -1 | cut -f3))"

# 30. A lot whose only witness is a browser test (19 Nov., c7): witness suite « navigateur » accepted at the deposit, and
# handed to the pass as SUITE_DU_TEMOIN=navigateur.
h=$(lot nav-1 extension/rag3weaver/src/nav.rs 'fn nav() {}')
P enqueue nav-1 nav-1 "$h" rust la_page_du_chat_nomme navigateur - - author-c > /dev/null 2>&1 && ok "suite du témoin « navigateur » acceptée au dépôt" || ko "suite du témoin « navigateur » refusée au dépôt"
P dequeue nav-1 > /dev/null 2>&1

# 32. A lot of the terminal's crate (19 Nov., tty): witness suite « tty » accepted at the deposit, and handed to the
# pass as SUITE_DU_TEMOIN=tty (judged at the pass's tty step).
h=$(lot tty-1 extension/rag3weaver/tty/src/t.rs 'fn t() {}')
P enqueue tty-1 tty-1 "$h" rust the_view_says_it tty - - author-c > /dev/null 2>&1 && ok "suite du témoin « tty » acceptée au dépôt" || ko "suite du témoin « tty » refusée au dépôt"
P dequeue tty-1 > /dev/null 2>&1

# 31. landed.tsv keeps the declared witness and its suite of each landed lot (19 Nov., c7: for an audit of what proved it).
h=$(lot lw-1 extension/rag3weaver/src/lw.rs 'fn lw() {}')
P enqueue lw-1 lw-1 "$h" rust lw_witness e2e_code - - author-c > /dev/null; echo VERT > "$D/outcomes.d/lw-1"
P step > /dev/null 2>&1
awk -F'\t' '$1 == "lw-1" && $4 == "lw_witness" && $5 == "e2e_code" {f = 1} END {exit !f}' "$D/pusher/landed.tsv" \
  && ok "landed.tsv garde le témoin déclaré et sa suite" || ko "landed.tsv sans le témoin : $(grep '^lw-1' "$D/pusher/landed.tsv")"
# 33. A lot that moves the codeparsers submodule's pointer (19 Nov., parent-cache): the pass plays the lot's pointer, the
# guard sees a clean tree, and the pull request of the previous step's docs lot is never closed by this step.
git init -q -b master "$D/sub" && git -C "$D/sub" config user.email t@t && git -C "$D/sub" config user.name t
echo v1 > "$D/sub/p.rs" && git -C "$D/sub" add -A && git -C "$D/sub" commit -qm c1 && c1=$(git -C "$D/sub" rev-parse HEAD)
echo v2 > "$D/sub/p.rs" && git -C "$D/sub" commit -qam c2 && c2=$(git -C "$D/sub" rev-parse HEAD)
git -C "$D/sub" checkout -q "$c1"
( cd "$D/author" && git fetch -q origin && git checkout -q -B sub-base origin/master && git submodule add -q "$D/sub" extension/rag3weaver/codeparsers \
  && git -C extension/rag3weaver/codeparsers checkout -q "$c1" && git add -A && git commit -qm "the submodule" && git push -q origin HEAD:master )
# The pusher's worktree has the submodule at master's pointer, as at luciepc.
git -C "$D/ws/wt" fetch -q origin && git -C "$D/ws/wt" checkout -q -f origin/master && git -C "$D/ws/wt" submodule update -q --init
h=$(lot sub-docs docs/sub.md '# sub'); P enqueue sub-docs sub-docs "$h" docs - - - - author-a > /dev/null; P step > /dev/null 2>&1
dpr=$(grep -P '\tsub-docs\tPR [0-9]+ ouverte' "$D/pusher/log.tsv" | tail -1 | cut -f3 | awk '{print $2}')
( cd "$D/author" && git fetch -q origin && git checkout -q -B sub-bump origin/master && git submodule update -q --init \
  && git -C extension/rag3weaver/codeparsers fetch -q origin && git -C extension/rag3weaver/codeparsers checkout -q "$c2" \
  && git add extension/rag3weaver/codeparsers && git commit -qm sub-bump && git push -q origin HEAD:sub-bump )
h=$(git -C "$D/author" rev-parse sub-bump)
P enqueue sub-bump sub-bump "$h" rust some_witness e2e_code - - author-c > /dev/null; echo VERT > "$D/outcomes.d/sub-bump"
P step > /dev/null 2>&1
played=$(awk '$1 ~ /-sub-bump/ {print $2}' "$D/pass_sub" | tail -1)
[ "$played" = "$c2" ] && ok "un lot qui bouge le pointeur du sous-module : la passe joue son codeparsers" || ko "la passe a joué le codeparsers $played, pas celui du lot ($c2)"
on_master sub-bump && ok "le lot du sous-module poussé, la garde voit un arbre propre" || ko "le lot du sous-module rendu : $(grep -P '\tsub-bump\t' "$D/pusher/log.tsv" | tail -1 | cut -f3,4)"
[ -n "$dpr" ] && ! grep -q "^pr close $dpr " "$D/gh_calls" && ok "la PR du lot de docs du pas d'avant n'est jamais fermée par ce pas" || ko "la PR $dpr du pas d'avant visée par une fermeture"
# 34. The tickets' headers (19 Nov., c7): master carries check_tickets.py; a docs lot whose ticket says « corrigé » outside
# closed/ is returned with the refusal; with master red at check_tickets, a docs lot that does not mend it waits behind
# the next lot, the lot that mends it lands, and the waiting one lands after.
tk() { printf '# %s\n\n- **État** : %s\n- **Famille** : produit\n- **Gravité** : faible\n- **Atteignable en service** : non\n- **Touche rag3weaver** : non\n' "$1" "$2"; }
( cd "$D/author" && git fetch -q origin && git checkout -q -B tk-base origin/master && mkdir -p extension/rag3weaver/scripts docs/s/tickets docs/tickets/closed && touch docs/tickets/closed/.keep \
  && cp "$HERE/../../extension/rag3weaver/scripts/check_tickets.py" extension/rag3weaver/scripts/ && tk un ouvert > docs/s/tickets/un.md \
  && git add -A && git commit -qm "check_tickets" && git push -q origin HEAD:master )
( cd "$D/author" && python3 extension/rag3weaver/scripts/check_tickets.py . > /dev/null ) && ok "master du témoin vert à check_tickets" || ko "master du témoin déjà rouge à check_tickets"
h=$(lot tk-bad docs/s/tickets/deux.md "$(tk deux corrigé)")
P enqueue tk-bad tk-bad "$h" docs - - - - author-a > /dev/null; P step > /dev/null 2>&1
! on_master tk-bad && grep -q "^rouge	tk-bad	.*check_tickets" "$D/pusher/inbox/author-a.txt" && ok "un ticket « corrigé » hors closed/ : le lot rendu avec le refus" || ko "le lot au ticket faux : $(grep -P '\ttk-bad\t' "$D/pusher/log.tsv" | tail -1 | cut -f3,4)"
# master made red by another way, then a docs lot that does not mend it, and one that does
( cd "$D/author" && git fetch -q origin && git checkout -q -B tk-red origin/master && tk trois corrigé > docs/s/tickets/trois.md && git add -A && git commit -qm "red tickets" && git push -q origin HEAD:master )
h1=$(lot tk-wait docs/other.md '# other'); P enqueue tk-wait tk-wait "$h1" docs - - - - author-a > /dev/null
h2=$(lot tk-mend docs/s/tickets/trois.md "$(tk trois ouvert)"); P enqueue tk-mend tk-mend "$h2" docs - - - - author-a > /dev/null
PUSHER_IDLE=0 PUSHER_TRAIN_MAX=1 P step > /dev/null 2>&1
w1=$(on_master tk-wait && echo poussé || echo attend)
PUSHER_IDLE=0 PUSHER_TRAIN_MAX=1 P step > /dev/null 2>&1; PUSHER_IDLE=0 PUSHER_TRAIN_MAX=1 P step > /dev/null 2>&1
[ "$w1" = attend ] && ! grep -q "^rouge	tk-wait	" "$D/pusher/inbox/author-a.txt" && ok "master rouge à check_tickets : le lot qui ne le répare pas attend, sans être rendu" || ko "master rouge : le lot qui ne répare pas est $w1"
on_master tk-mend && on_master tk-wait && ok "le lot qui répare atterrit, puis celui qui attendait" || ko "après la réparation : tk-mend $(on_master tk-mend && echo poussé || echo absent), tk-wait $(on_master tk-wait && echo poussé || echo absent)"
exit $red
