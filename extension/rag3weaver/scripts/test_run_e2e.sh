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
# (RAG3WEAVER_VERROU_TENU : la garde ne bâtit rien, elle n'attend pas le verrou d'une passe en cours —
# sans lui, le témoin tombait sur son délai derrière un lourd du poste, le 7 nov.)
# (7 novembre 2026 : un test sous openai-llm sortait du compte sans un mot).
rm -f "$T/cargo-appele"
RAG3WEAVER_VERROU_TENU=1 RAG3WEAVER_E2E_FEATURES=rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres \
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
RAG3WEAVER_VERROU_TENU=1 RAG3WEAVER_E2E_VERIFIER_SEULEMENT=1 \
  PATH="$T/bin:$PATH" CARGO_TARGET_DIR="$T/target" RAG3WEAVER_MOTEUR_ANCIEN=1 timeout 120 bash "$SCRIPT" --test e2e_memoire_modele \
  > "$T/sortie-defaut" 2>&1
code=$?
[ "$code" = 0 ] && grep -q "chaque suite demandée a les siennes" "$T/sortie-defaut" && grep -q "openai-llm,anthropic-llm" "$T/sortie-defaut"
cas $? "le jeu par défaut porte openai-llm et anthropic-llm, et la suite passe la garde (rendu $code)"
[ ! -e "$T/cargo-appele" ]
cas $? "la garde seule n'appelle pas cargo"
# Le compte des suites (8 novembre 2026) : `--test e2e e2e_lectures` jouait la
# suite et disait « 0 sur 1 » — il cherchait `tests/e2e.rs`. Le filtre désigne
# la suite, `--test e2e` est le binaire ; la ligne du script (`Running
# tests/e2e_lectures.rs (binaire e2e unique, …)`) ou celle de cargo
# (`Running tests/e2e/main.rs`) la comptent jouée.
printf '     Running tests/e2e_lectures.rs (binaire e2e unique, filtre e2e_lectures::)\ntest result: ok. 18 passed; 0 failed;\n' > "$T/journal-script"
printf '     Running tests/e2e/main.rs (target/debug/deps/e2e-0123)\ntest result: ok. 18 passed; 0 failed;\n' > "$T/journal-cargo"
printf 'rien\n' > "$T/journal-vide"
compte() { # <journal> <args…>
  local j="$1"; shift
  rm -f "$T/cargo-appele"
  RAG3WEAVER_VERROU_TENU=1 RAG3WEAVER_E2E_COMPTER_JOURNAL="$j" \
    PATH="$T/bin:$PATH" CARGO_TARGET_DIR="$T/target" RAG3WEAVER_MOTEUR_ANCIEN=1 timeout 120 bash "$SCRIPT" "$@" 2>&1
}
compte "$T/journal-script" --test e2e e2e_lectures > "$T/sortie-compte" ; grep -q "Suites jouées : 1 sur 1 demandées." "$T/sortie-compte"
cas $? "--test e2e e2e_lectures, journal du script : 1 sur 1"
compte "$T/journal-cargo" --test e2e e2e_lectures > "$T/sortie-compte2" ; grep -q "Suites jouées : 1 sur 1 demandées." "$T/sortie-compte2"
cas $? "--test e2e e2e_lectures, journal de cargo (tests/e2e/main.rs) : 1 sur 1"
compte "$T/journal-script" --test e2e_lectures > "$T/sortie-compte3" ; grep -q "Suites jouées : 1 sur 1 demandées." "$T/sortie-compte3"
cas $? "--test e2e_lectures sans --test e2e : 1 sur 1"
compte "$T/journal-script" --test e2e_lectures --test e2e_tty > "$T/sortie-compte4" ; grep -q "Suites jouées : 1 sur 2 demandées." "$T/sortie-compte4" && grep -q "manquantes : e2e_tty" "$T/sortie-compte4"
cas $? "une suite demandée sans sa ligne reste manquante : 1 sur 2, e2e_tty nommée"
compte "$T/journal-vide" --test e2e e2e_lectures > "$T/sortie-compte5" ; grep -q "Suites jouées : 0 sur 1 demandées." "$T/sortie-compte5" && grep -q "manquantes : e2e_lectures" "$T/sortie-compte5"
cas $? "--test e2e e2e_lectures sans aucune ligne : 0 sur 1, la suite nommée"
[ ! -e "$T/cargo-appele" ]
cas $? "le compte seul n'appelle pas cargo"
# Les bancs à services (9 novembre 2026) : jamais dans l'e2e entier, joués seuls
# avec leur voie, refusés rouges avant tout bâti sans elle.
banc() { # <sortie> <env…> -- <args…>
  local s="$1"; shift; local env=(); while [ "$1" != -- ]; do env+=("$1"); shift; done; shift
  rm -f "$T/cargo-appele"
  env -u RAG3WEAVER_BANC_VERTEX -u RAG3WEAVER_BANC_DECIDE RAG3WEAVER_VERROU_TENU=1 RAG3WEAVER_MOTEUR_ANCIEN=1 "${env[@]}" \
    PATH="$T/bin:$PATH" CARGO_TARGET_DIR="$T/target" timeout 120 bash "$SCRIPT" "$@" > "$T/$s" 2>&1
}
banc sortie-banc1 -- --test e2e_banc_fiches_classees; code=$?
[ "$code" = 1 ] && grep -q "e2e_banc_fiches_classees est un banc à services : il se joue seul, avec sa voie" "$T/sortie-banc1" && [ ! -e "$T/cargo-appele" ]
cas $? "un banc nommé sans sa voie : rouge avant tout cargo, il dit quoi poser (rendu $code)"
banc sortie-banc2 RAG3WEAVER_BANC_VERTEX=1 RAG3WEAVER_E2E_FEATURES=rag3db-native,code,daemon -- --test e2e_banc_fiches_classees; code=$?
[ "$code" = 1 ] && grep -q "RAG3WEAVER_BANC_VERTEX (feature openai-llm)" "$T/sortie-banc2" && [ ! -e "$T/cargo-appele" ]
cas $? "une voie posée sans sa feature : rouge, la variable et la feature nommées (rendu $code)"
banc sortie-banc3 RAG3WEAVER_BANC_VERTEX=1 RAG3WEAVER_E2E_VERIFIER_SEULEMENT=1 -- --test e2e_banc_fiches_classees; code=$?
[ "$code" = 0 ] && grep -q "chaque suite demandée a les siennes" "$T/sortie-banc3"
cas $? "le banc nommé avec sa voie et sa feature passe la garde (rendu $code)"
banc sortie-banc4 RAG3WEAVER_BANC_VERTEX=1 RAG3WEAVER_E2E_VERIFIER_SEULEMENT=1 --; code=$?
grep -q "e2e_banc_fiches_classees NON JOUÉE (banc à services, il se joue seul" "$T/sortie-banc4"
cas $? "l'e2e entier écarte le banc même sa variable posée, en le disant (rendu $code)"
[ ! -e "$T/cargo-appele" ]
cas $? "les gardes des bancs n'appellent pas cargo"
[ $echec = 0 ] || { echo "--- sortie"; tail -15 "$T/sortie"; tail -15 "$T/sortie-features" 2>/dev/null; tail -15 "$T/sortie-defaut" 2>/dev/null; tail -5 "$T"/sortie-compte* "$T"/sortie-banc* 2>/dev/null; }
exit $echec
