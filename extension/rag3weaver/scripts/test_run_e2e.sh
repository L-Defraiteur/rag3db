#!/bin/bash
# Témoin de run_e2e.sh, sur ce qui se juge sans jouer de suite. Un faux
# `cargo` en tête du PATH note s'il est appelé et échoue ; le target est un
# dossier temporaire (le balayage de sortie n'y trouve rien). Rien n'est bâti,
# rien n'est joué. Rend 0 si tout tient.
#
#   scripts/test_run_e2e.sh            (le run_e2e.sh de ce crate)
#   scripts/test_run_e2e.sh <script>   (un autre, pour prouver le rouge d'avant)
set -u
ICI="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="${1:-$ICI/run_e2e.sh}"
T="$(mktemp -d "${TMPDIR:-/tmp}/test-run-e2e.XXXXXX")"
trap 'rm -rf "$T"' EXIT
echec=0
cas() { if [ "$1" = 0 ]; then echo "OK     $2"; else echo "ÉCHEC  $2"; echec=1; fi; }

mkdir -p "$T/bin" "$T/target"
cat > "$T/bin/cargo" <<EOF
#!/bin/bash
touch "$T/cargo-appele"
exit 1
EOF
chmod +x "$T/bin/cargo"

# Plusieurs filtres positionnels : refusés et dits, avant tout cargo.
PATH="$T/bin:$PATH" CARGO_TARGET_DIR="$T/target" timeout 60 bash "$SCRIPT" e2e_un e2e_deux e2e_trois \
  > "$T/sortie" 2>&1
code=$?
[ "$code" = 2 ]
cas $? "trois filtres positionnels : code 2 (rendu $code)"
grep -q "3 filtres positionnels (e2e_un e2e_deux e2e_trois) : un seul est permis" "$T/sortie" \
  && grep -q -- "--test e2e_un --test e2e_deux --test e2e_trois" "$T/sortie"
cas $? "le refus nomme les trois et donne la forme --test"
[ ! -e "$T/cargo-appele" ]
cas $? "rien n'est bâti ni joué : cargo n'est pas appelé"
[ $echec = 0 ] || { echo "--- sortie"; tail -15 "$T/sortie"; }
exit $echec
