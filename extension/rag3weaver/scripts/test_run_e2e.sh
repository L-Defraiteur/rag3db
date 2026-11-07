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
# Une suite qui exige une feature absente du jeu : rouge, nommée, avant tout cargo
# (7 novembre 2026 : un test sous openai-llm sortait du compte sans un mot).
rm -f "$T/cargo-appele"
RAG3WEAVER_E2E_FEATURES=rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres \
  PATH="$T/bin:$PATH" CARGO_TARGET_DIR="$T/target" RAG3WEAVER_MOTEUR_ANCIEN=1 timeout 120 bash "$SCRIPT" --test e2e_memoire_modele \
  > "$T/sortie-features" 2>&1
code=$?
[ "$code" = 1 ]
cas $? "une suite sous une feature absente : code 1 (rendu $code)"
grep -q "e2e_memoire_modele (openai-llm)" "$T/sortie-features" && grep -q "ne joueraient pas tous leurs tests" "$T/sortie-features"
cas $? "le refus nomme la suite et la feature"
[ ! -e "$T/cargo-appele" ]
cas $? "rien n'est bâti ni joué : cargo n'est pas appelé"
# Le jeu de la procédure, par défaut : la même suite passe la garde.
RAG3WEAVER_E2E_VERIFIER_SEULEMENT=1 \
  PATH="$T/bin:$PATH" CARGO_TARGET_DIR="$T/target" RAG3WEAVER_MOTEUR_ANCIEN=1 timeout 120 bash "$SCRIPT" --test e2e_memoire_modele \
  > "$T/sortie-defaut" 2>&1
code=$?
[ "$code" = 0 ] && grep -q "chaque suite demandée a les siennes" "$T/sortie-defaut" && grep -q "openai-llm,anthropic-llm" "$T/sortie-defaut"
cas $? "le jeu par défaut porte openai-llm et anthropic-llm, et la suite passe la garde (rendu $code)"
[ ! -e "$T/cargo-appele" ]
cas $? "la garde seule n'appelle pas cargo"
[ $echec = 0 ] || { echo "--- sortie"; tail -15 "$T/sortie"; tail -15 "$T/sortie-features" 2>/dev/null; tail -15 "$T/sortie-defaut" 2>/dev/null; }
exit $echec
