#!/bin/bash
# Witness of pusher.sh (lot 1 of the pusher, docs/optimiseur/18-11-2026/02-le-pousseur.md): a throwaway origin, an
# author clone and the pusher's single worktree; the pass and the push guard are replaced by doubles, the guard's
# --code-apporte is the real one. Each case deposits a lot, runs one step, and reads master, the log and the inbox.
# Usage: bash tools/poste/test_pusher.sh [path of pusher.sh] — exit 0 when every case is green.
set -u
PUSHER=$(realpath "${1:-$(dirname "$0")/pusher.sh}")
HERE=$(dirname "$(realpath "$0")")
[ -f "$PUSHER" ] || { echo "ROUGE  pusher.sh absent : $PUSHER"; exit 1; }
D=$(mktemp -d); trap 'rm -rf "$D"' EXIT
red=0
ok() { echo "vert   $1"; }
ko() { echo "ROUGE  $1"; red=1; }

# The origin, seeded with a master that has docs, a poste script and its witness, Rust and engine code.
git init -q --bare -b master "$D/origin.git"
git clone -q "$D/origin.git" "$D/author" 2>/dev/null
cd "$D/author" && git config user.email luciedefraiteur@gmail.com && git config user.name t
mkdir -p docs tools/poste extension/rag3weaver/src src
echo '# notes' > docs/notes.md
printf '#!/bin/bash\necho ok\n' > tools/poste/foo.sh
printf '#!/bin/bash\ngrep -q "echo ok" "$(dirname "$0")/foo.sh"\n' > tools/poste/test_foo.sh
cp "$HERE/push_garde.sh" tools/poste/push_garde.sh
echo 'fn main() {}' > extension/rag3weaver/src/lib.rs
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
# The pass double: one outcome per call, read from $D/outcomes (VERT, ROUGE <step> or 124).
cat > "$D/pass.sh" <<'P'
#!/bin/bash
echo "$*" >> "$PASS_CALLS"
J=$PUSHER_JOURNALS_ROOT/$2; mkdir -p "$J"
echo "commit $(git -C "$PUSHER_WORKSPACES/$1" rev-parse HEAD)" > "$J/resume.log"
o=$(head -1 "$OUTCOMES"); sed -i 1d "$OUTCOMES"
case "$o" in
  VERT) echo VERT >> "$J/resume.log" ;;
  124) echo "ROUGE e2e_code (124)" >> "$J/resume.log"; exit 124 ;;
  *) echo "$o" >> "$J/resume.log"; exit 1 ;;
esac
P
# The guard double: the real --code-apporte; otherwise the push of the worktree, as the guard ends.
cat > "$D/guard.sh" <<'G'
#!/bin/bash
if [ "$1" = --code-apporte ]; then exec bash "$REAL_GUARD" "$@"; fi
echo "$*" >> "$GUARD_CALLS"
cd "$PUSHER_WORKSPACES/$1" || exit 1
tail -1 "$PUSHER_JOURNALS_ROOT/$2/resume.log" | grep -qx VERT || { echo "ARRÊT : la passe n'est pas VERTE"; exit 1; }
git push -q origin HEAD:master || { echo "ARRÊT : push refusé"; exit 1; }
echo POUSSE
G
export PUSHER_DIR=$D/pusher PUSHER_WORKSPACES=$D/ws PUSHER_WT_NAME=wt PUSHER_JOURNALS_ROOT=$D/journals \
  PUSHER_PASS=$D/pass.sh PUSHER_GUARD=$D/guard.sh PUSHER_POSTE= PUSHER_MASTER_HELD=$D/master-tenu \
  REAL_GUARD=$HERE/push_garde.sh OUTCOMES=$D/outcomes PASS_CALLS=$D/pass_calls GUARD_CALLS=$D/guard_calls
: > "$D/outcomes"; : > "$D/pass_calls"; : > "$D/guard_calls"
P() { bash "$PUSHER" "$@"; }
master() { git -C "$D/author" fetch -q origin && git -C "$D/author" rev-parse origin/master; }
on_master() { git -C "$D/author" fetch -q origin && git -C "$D/author" log --format=%s origin/master | grep -qx "$1"; }
calls() { grep -c . "$1"; }

# 1. Deposits refused.
h=$(lot docs-1 docs/notes.md '# notes, more')
P enqueue docs-1 docs-1 "$h" docs - - - - author-a > /dev/null && ok "dépôt d'un lot de docs accepté" || ko "dépôt d'un lot de docs refusé"
P enqueue docs-1 docs-1 "$h" docs - - - - author-a > /dev/null 2>&1 && ko "id en double accepté" || ok "id en double refusé"
P enqueue docs-x docs-1 0123456789 docs - - - - author-a > /dev/null 2>&1 && ko "tête autre que celle d'origin acceptée" || ok "tête autre que celle d'origin refusée"
P enqueue docs-y docs-1 "$h" rust e2e_code e2e - - author-a > /dev/null 2>&1 && ko "kind faux (rust pour des docs) accepté" || ok "kind faux refusé"
hc=$(lot cpp-1 src/engine.cpp 'int main() { return 1; }')
P enqueue cpp-1 cpp-1 "$hc" cpp - - - - author-a > /dev/null 2>&1 && ko "lot C++ accepté" || ok "lot C++ refusé (hors du pousseur)"
P enqueue docs-z docs-1 "$h" docs - - - absent author-a > /dev/null 2>&1 && ko "parent absent de la file accepté" || ok "parent absent de la file refusé"

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

# 4. A scripts lot, witness green; then one whose witness is red.
h=$(lot scripts-1 tools/poste/foo.sh '#!/bin/bash
echo ok; echo more')
P enqueue scripts-1 scripts-1 "$h" scripts - - - - author-b > /dev/null && P step > /dev/null 2>&1
on_master scripts-1 && ok "scripts : témoin vert, poussé" || ko "scripts au témoin vert : pas poussé"
h=$(lot scripts-2 tools/poste/foo.sh '#!/bin/bash
echo broken')
P enqueue scripts-2 scripts-2 "$h" scripts - - - - author-b > /dev/null && P step > /dev/null 2>&1
! on_master scripts-2 && grep -q "^rouge	scripts-2	code	" "$D/pusher/inbox/author-b.txt" && ok "scripts : témoin rouge, rendu à l'auteur" || ko "scripts au témoin rouge : poussé, ou pas rendu"
[ -z "$(grep -v '^#' "$D/pusher/queue.tsv")" ] && ok "rouge sorti de la file" || ko "rouge resté dans la file"

# 5. A Rust lot: one pass, then the guard.
h=$(lot rust-1 extension/rag3weaver/src/lib.rs 'fn main() { println!(); }')
echo VERT >> "$D/outcomes"
P enqueue rust-1 rust-1 "$h" rust e2e_code e2e - - author-c > /dev/null && P step > /dev/null 2>&1
on_master rust-1 && [ "$(calls "$D/pass_calls")" = 1 ] && [ "$(calls "$D/guard_calls")" = 1 ] && ok "rust : une passe, la garde, poussé" || ko "rust : pas poussé, ou passe ou garde manquante"
grep -q "	rust-1	.*peak=" "$D/pusher/log.tsv" && ok "pic mémoire du rejeu au journal" || ko "pas de pic mémoire au journal"

# 6. A Rust lot red on code: returned, no guard.
h=$(lot rust-2 extension/rag3weaver/src/lib.rs 'fn main() { panic!(); }')
echo "ROUGE e2e_code" >> "$D/outcomes"
P enqueue rust-2 rust-2 "$h" rust e2e_code e2e - - author-c > /dev/null && P step > /dev/null 2>&1
! on_master rust-2 && [ "$(calls "$D/guard_calls")" = 1 ] && grep -q "^rouge	rust-2	code	" "$D/pusher/inbox/author-c.txt" && ok "rust rouge de code : rendu, sans garde" || ko "rust rouge de code : mal traité"

# 7. Environment red: replayed once, then green; twice: returned as environment.
h=$(lot rust-3 extension/rag3weaver/src/lib.rs 'fn main() { let _ = 3; }')
printf '124\nVERT\n' >> "$D/outcomes"
P enqueue rust-3 rust-3 "$h" rust e2e_code e2e - - author-c > /dev/null && P step > /dev/null 2>&1
on_master rust-3 && [ "$(calls "$D/pass_calls")" = 4 ] && ok "rouge d'environnement rejoué une fois, puis poussé" || ko "rouge d'environnement : pas rejoué, ou pas poussé"
h=$(lot rust-4 extension/rag3weaver/src/lib.rs 'fn main() { let _ = 4; }')
printf '124\n124\n' >> "$D/outcomes"
P enqueue rust-4 rust-4 "$h" rust e2e_code e2e - - author-c > /dev/null && P step > /dev/null 2>&1
! on_master rust-4 && grep -q "^rouge	rust-4	environnement	" "$D/pusher/inbox/author-c.txt" && ok "deux rouges d'environnement : rendu comme environnement" || ko "deux rouges d'environnement : mal traités"

# 8. A conflict at the repose: returned, master unchanged.
h=$(FROM=$BASE0 lot docs-2 docs/notes.md '# notes, the other way')
P enqueue docs-2 docs-2 "$h" docs - - - - author-a > /dev/null; m0=$(master)
P step > /dev/null 2>&1
[ "$(master)" = "$m0" ] && grep -q "^rouge	docs-2	conflit	" "$D/pusher/inbox/author-a.txt" && ok "conflit à la repose : rendu à l'auteur" || ko "conflit à la repose : mal traité"

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
for x in "bi-a $ha" "bi-b $hb" "bi-c $hc" "bi-d $hd"; do set -- $x; P enqueue "$1" "$1" "$2" rust e2e_code e2e - - author-c > /dev/null; done
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
P enqueue pa-1 pa-1 "$hp" rust e2e_code e2e - - author-c > /dev/null; P enqueue pa-2 pa-2 "$hk" rust e2e_code e2e - pa-1 author-c > /dev/null
printf 'ROUGE e2e_code\nROUGE e2e_code\n' >> "$D/outcomes"
P step > /dev/null 2>&1
grep -q "^rouge	pa-1	code	" "$D/pusher/inbox/author-c.txt" && grep -q "^rouge	pa-2	parent rouge	" "$D/pusher/inbox/author-c.txt" && [ -z "$(grep -v '^#' "$D/pusher/queue.tsv")" ] \
  && ok "l'enfant d'un lot rouge est rendu avec lui" || ko "l'enfant d'un lot rouge : pas rendu, ou resté dans la file"
exit $red
