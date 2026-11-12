#!/bin/bash
# Run rag3weaver E2E tests with a dedicated native build.
#
# This build includes all required extensions (vector, geo)
# and is isolated from other builds (WASM, nodejs, etc.).
#
# Usage:
#   ./run_e2e.sh                          # run all e2e_search tests (skip build if exists)
#   ./run_e2e.sh phase0                   # run tests matching "phase0"
#   ./run_e2e.sh --test e2e_phase0b       # run e2e_phase0b tests instead
#   ./run_e2e.sh --build                  # force rebuild rag3db before tests
#   ./run_e2e.sh --build-only             # just build, don't run tests
#   ./run_e2e.sh --no-cuda phase0         # accepté, sans effet (burn/wgpu, pas de CUDA)
#   ./run_e2e.sh --summary                # show only the per-suite summary at the end
#   ./run_e2e.sh --features navigateur    # add features to the default set

set -euo pipefail

# ── Le verrou du poste ──────────────────────────────────────────────────────
#
# Une mesure de durée ou de mémoire ne vaut que seule sur le poste. Les
# annonces croisées entre sessions ont échoué deux fois le 4 octobre 2026 :
# une boucle e2e_code que personne n'avait dans sa liste a faussé une série de
# mesures. Le script prend donc lui-même le verrou du poste
# (`~/.cache/rag3weaver-build/poste.lock`, décision de l'orchestration) :
# - **partagé** par défaut : les passes ordinaires tournent ensemble, elles
#   attendent seulement qu'une mesure ait fini ;
# - **exclusif** pour une mesure (`RAG3WEAVER_MESURE=1`) et pour un rebâti du
#   moteur (`--build`, `--build-only`), que personne ne doit lire en même
#   temps.
# La **porte** (`porte.lock`, le script `~/.cache/rag3weaver-build/poste`)
# donne la priorité à la mesure : une passe partagée la franchit avant
# d'entrer, une passe exclusive la tient pendant qu'elle attend et tourne.
# Dès qu'une mesure attend, plus aucune passe partagée n'entre ; elle n'attend
# que la fin de celles déjà lancées. Sans porte, les partagées enchaînées
# l'affamaient.
# Tout ce qui n'est pas une mesure tourne en **priorité basse** (`nice -n 15
# ionice -c 3`) : l'écran et la personne devant le poste passent avant. Une
# mesure garde la priorité normale, sinon son chiffre ne vaut rien.

# **Un seul filtre positionnel** (relevé par le proto, 12 octobre 2026) :
# trois noms de suites donnés à la suite n'en filtraient qu'un, le dernier,
# et les deux autres étaient ignorés en silence — trois suites crues jouées,
# une seule l'était. Le positionnel est UN filtre sur les noms de tests ; des
# suites se nomment par `--test`, qui s'accumule. Jugé ici, avant le verrou
# du poste : un refus d'arguments n'attend pas une passe en cours.
POSITIONNELS=()
suivant_est_valeur=false
for a in "$@"; do
  if [ "$suivant_est_valeur" = true ]; then suivant_est_valeur=false; continue; fi
  case "$a" in
    --test|--features) suivant_est_valeur=true ;;
    -*) ;;
    *) POSITIONNELS+=("$a") ;;
  esac
done
if [ ${#POSITIONNELS[@]} -gt 1 ]; then
  echo "✗ ${#POSITIONNELS[@]} filtres positionnels (${POSITIONNELS[*]}) : un seul est permis, il filtre les noms de tests." >&2
  printf '  Pour jouer plusieurs suites :'  >&2
  printf ' --test %s' "${POSITIONNELS[@]}" >&2
  printf '\n' >&2
  exit 2
fi

# La primitive commune prend réellement le poste avant le marqueur, atteste
# Max32 et garde ses descendants/bus privé jusqu'au nettoyage. Aucun drapeau
# d'environnement seul ne permet de sauter cette vérification.
CONFINEUR="$(cd "$(dirname "$0")" && pwd)/scripts/confiner_validation.py"
if ! python3 "$CONFINEUR" verifier >/dev/null 2>&1; then
  mode_validation=lourd
  for a in "$@"; do
    case "$a" in --build|--build-only) mode_validation=mesure ;; esac
  done
  [ "${RAG3WEAVER_MESURE:-}" != 1 ] || mode_validation=mesure
  commande_validation=(bash "$0" "$@")
  # Un rebâti ordinaire est exclusif ET discret ; seule une mesure demandée
  # garde la priorité normale. `poste lourd` pose lui-même la priorité basse.
  if [ "$mode_validation" = mesure ] && [ "${RAG3WEAVER_MESURE:-}" != 1 ]; then
    commande_validation=(nice -n 15 ionice -c 3 "${commande_validation[@]}")
  fi
  exec python3 "$CONFINEUR" lancer --mode "$mode_validation" \
    --delai "${RAG3WEAVER_VALIDATION_TIMEOUT:-18000}" -- "${commande_validation[@]}"
fi

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# **La bibliothèque contre laquelle tout est éprouvé.**
#
# C'était `build/native-test`, du 24 août — donc antérieure de dix jours au
# report de Vela sur `storage_manager.cpp`, qui lève l'exclusion
# lecteur/écrivain. Toutes nos suites tournaient contre un cœur qui refusait
# encore un lecteur pendant qu'un écrivain tient la base ; on l'a découvert le
# 3 septembre 2026 en éprouvant la reprise sur refus transitoire, et le test le
# montrait noir sur blanc (`e2e_prise_atomique`, 80 refus contre 80 lectures
# selon la bibliothèque liée).
#
# `build/lecteurs` porte le correctif, avec le même type Release, le même
# `BUILD_SHARED_LIBS`, les mêmes extensions. `RAG3DB_BUILD` permet de revenir
# sur l'ancienne en une variable — utile pour vérifier qu'un test dit vrai des
# deux côtés, ce que plusieurs des nôtres font maintenant exprès.
#
# **Et pour un `cargo` joué à la main, c'est le couple
# `RAG3DB_LIBRARY_DIR` + `RAG3DB_INCLUDE_DIR` qui évite le bâti cmake**, pas
# `RAG3DB_BUILD` ni `RAG3DB_SHARED`. Lu dans `tools/rust_api/build.rs` : si les
# deux sont posées, build.rs ajoute un `rustc-link-search` et un `rpath` et
# s'arrête là ; si l'une manque, il tombe dans `build_bundled_cmake()` et
# rebâtit **tout le moteur**. `RAG3DB_SHARED` ne décide que du mode de liaison
# (`dylib` au lieu de `static`) — seul, il ne protège de rien.
#
# Écrit ici parce que la nuance coûte vingt minutes à qui ne pose qu'une des
# trois, et qu'on ne la voit pas : cargo ne dit pas qu'il part en cmake.
# Vérifié le 10 octobre 2026 sur un worktree neuf de luciepc — la seule
# occurrence de « cmake » dans la trace était la **crate** `cmake`, compilée
# comme dépendance de build ; zéro `Building CXX`.
BUILD="${RAG3DB_BUILD:-$ROOT/build/lecteurs-csv}"
WEAVER="$ROOT/extension/rag3weaver"

# **Le magasin de libs, par défaut** (page docs/11-octobre-2026-recherche/07-le-magasin-de-libs.md,
# décision de l'orchestration du 12 octobre 2026) : la lib, les en-têtes et
# l'extension d'un dossier du magasin, résolu **une fois**, ici. Une
# publication pendant la passe ne la change pas, et plus personne n'a à
# s'arrêter pour un rebâti. `RAG3DB_LIB=bati` reprend le dossier de bâti,
# `RAG3DB_LIB=<identifiant>` un bâti nommé ; `RAG3DB_BUILD` ne dit plus que
# où `--build` bâtit. L'ordre : scripts/lib_du_magasin.sh, `choisir_la_lib`.
. "$WEAVER/scripts/lib_du_magasin.sh"
LIBDIR="$(choisir_la_lib "$BUILD")" || exit 2
MAGASIN=false
[ "$LIBDIR" != "$BUILD/src" ] && MAGASIN=true

# **Un worktree ne lie pas son propre moteur en silence** (10 octobre 2026).
# Sans `RAG3DB_BUILD`, `BUILD` est le `build/lecteurs-csv` **du worktree** :
# s'il en a un (une session qui y a bâti), la passe le lie sans le dire, et
# une mesure compare deux moteurs en croyant n'en voir qu'un (vécu par la
# session recherche pendant sa tenue de mesure). Un worktree nomme donc sa
# lib, ou déclare qu'il éprouve la sienne (`RAG3WEAVER_MOTEUR_DU_WORKTREE=1`,
# le cas du cœur C++).
if [ "$MAGASIN" = false ] && [ -z "${RAG3DB_BUILD:-}" ] && [ "${RAG3WEAVER_MOTEUR_DU_WORKTREE:-0}" != 1 ]; then
  commun="$(git -C "$ROOT" rev-parse --path-format=absolute --git-common-dir 2>/dev/null || true)"
  propre="$(git -C "$ROOT" rev-parse --path-format=absolute --git-dir 2>/dev/null || true)"
  if [ -n "$commun" ] && [ "$commun" != "$propre" ]; then
    echo "✗ worktree sans RAG3DB_BUILD : la passe lierait le moteur du worktree ($BUILD) sans le dire."
    echo "  Nommez la lib commune (RAG3DB_BUILD=<arbre principal>/build/lecteurs-csv, et RAG3DB_ROOT pour"
    echo "  l'extension vector), ou RAG3WEAVER_MOTEUR_DU_WORKTREE=1 pour éprouver le moteur de ce worktree."
    exit 2
  fi
fi

# La portée commune possède déjà MemoryMax et le budget. Une nouvelle scope
# systemd --user ici serait une sœur, pas une limite héritée du superviseur.
confined() { "$@"; }

# ── Tracer la charge, pendant la compilation comme pendant les tests ───────
#
# « Ma machine recommence à galérer » n'est pas une mesure. Un échantillon
# toutes les cinq secondes dans un TSV en est une, et elle survit à la passe :
# on peut y revenir le lendemain pour savoir *quand* ça a basculé.
#
# Ça démarre ici, avant le build, parce que c'est le build C++ qui coûte le
# plus cher — et c'est justement celui qu'on ne voyait pas.
#
# `RAG3WEAVER_CHARGE=0` désactive ; `RAG3WEAVER_CHARGE_INTERVALLE` change le pas.
CHARGE_LOG="${RAG3WEAVER_CHARGE_LOG:-$WEAVER/target/charge-last.tsv}"
CHARGE_PID=""
if [ "${RAG3WEAVER_CHARGE:-1}" != "0" ] && [ -x "$WEAVER/charge.py" ]; then
  mkdir -p "$(dirname "$CHARGE_LOG")"
  : > "$CHARGE_LOG"
  "$WEAVER/charge.py" --sortie "$CHARGE_LOG" \
    --intervalle "${RAG3WEAVER_CHARGE_INTERVALLE:-5}" &
  CHARGE_PID=$!
  echo "▸ Charge tracée dans $CHARGE_LOG (tail -f pour suivre)"
fi

# ── Le démon d'embarquement né pendant la passe s'arrête avec elle ──────────
#
# Le démon survit exprès à la suite qui l'a lancé (`Fin::Laisser`,
# `tests/common/mod.rs`) : le binaire suivant retrouve BGE-M3 déjà chargé, et
# la passe le charge une fois au lieu d'une par binaire. Cette survie ne sert
# plus rien après la passe : le 4 octobre 2026, un démon resté en place tenait
# 2,7 Go de mémoire et 3,6 Go sur la carte qui porte l'écran de Lucie, et
# l'écran a gelé.
#
# Le démon hérite de l'environnement de la suite qui le lance : la marque
# `RAG3WEAVER_PASSE_E2E` dit lequel est né de *cette* passe. On n'arrête que
# ceux-là, jamais celui d'une autre passe partagée qui tourne à côté. Par
# `pidof` et SIGTERM, jamais `pgrep -f` (un motif attrape le shell qui le
# porte) ; `|| true` parce que `pidof` rend 1 quand il ne trouve rien, ce qui
# tuerait la passe en silence sous `set -e`. Une passe interrompue nettoie
# aussi : c'est le piège de sortie.
export RAG3WEAVER_PASSE_E2E="$$-$(date +%s%N)"
preparer_journaux_des_demons() {
  # Une ressource partagée entre suites appartient à la passe, sur disque.
  # Ticket recherche/tickets/2026-11-11-le-service-d-embarquement-laisse-son-journal-dans-le-tmpdir-d-e2e-search.md.
  export RAG3WEAVER_E2E_JOURNAUX_DEMONS="$E2E_DISQUE_PARENT/demons"
  mkdir -p "$RAG3WEAVER_E2E_JOURNAUX_DEMONS"
}
pids_des_demons() {
  local resultat code
  if resultat=$(pidof rag3weaver-embeddings); then
    printf '%s\n' "$resultat"
  else
    code=$?
    [ "$code" = 1 ] || { echo "✗ inventaire des démons indisponible" >&2; return 1; }
  fi
}
arreter_les_demons_de_la_passe() {
  local pids=() liste
  liste=$(pids_des_demons) || return 1
  read -r -a pids <<< "$liste"
  python3 "$WEAVER/scripts/arret_demons_e2e.py" "$RAG3WEAVER_PASSE_E2E" "${pids[@]}"
}
parent_de_passe_quiescent() {
  local pid="${1##*-}" pids=() liste
  [[ "$pid" =~ ^[0-9]+$ ]] || return 1
  [ ! -d "/proc/$pid" ] || return 1
  liste=$(pids_des_demons) || return 1
  read -r -a pids <<< "$liste"
  python3 "$WEAVER/scripts/arret_demons_e2e.py" --quiescence "$pid-" "${pids[@]}"
}
garder_parent_termine() {
  local parent="$1" garde="$E2E_DISQUE_RACINE/e2e-gardes/parent-${1##*/}-$RAG3WEAVER_PASSE_E2E"
  mkdir -p "$(dirname "$garde")" && mv -- "$parent" "$garde" || return 1
  echo "▸ restes d'une passe quiescente archivés : $garde"
}
archiver_journaux_des_demons() {
  local dossier="${RAG3WEAVER_E2E_JOURNAUX_DEMONS:-}" garde
  [ -n "$dossier" ] && [ -d "$dossier" ] || return 0
  [ -n "$(find "$dossier" -mindepth 1 -print -quit)" ] || return 0
  garde="$E2E_DISQUE_RACINE/e2e-gardes/demons-$RAG3WEAVER_PASSE_E2E"
  mkdir -p "$(dirname "$garde")" && mv -- "$dossier" "$garde" || return 1
  echo "▸ journaux des démons terminés conservés : $garde"
}

# **Les vieux binaires de test s'effacent** (10 octobre 2026, disque plein) :
# le hachage d'un binaire change avec le jeu de features, le profil, et le
# **chemin du paquet** — chaque worktree qui partage un target y laisse les
# siens. Un target de session en portait 3 par suite. À la sortie, on garde
# le plus récent de chaque `e2e_*` et on efface les autres, avec leurs `.d`
# et leurs `.dwo`. Un binaire effacé se rebâtit s'il resservait.
balayer_les_vieux_binaires() {
  local deps="${CARGO_TARGET_DIR:-$WEAVER/target}/debug/deps" nom garde f h efface=0
  [ -d "$deps" ] || return 0
  # **Les binaires d'avant le binaire unique** (`e2e_<suite>-<empreinte>`, un
  # par suite) ne sont plus jamais produits depuis le 11 octobre 2026 : en
  # garder « le plus récent de chaque » laisserait ~270 Mo morts par suite.
  # Ils partent tous.
  for f in $(find "$deps" -maxdepth 1 -type f -perm -u+x -name 'e2e_*-????????????????' -printf '%f\n'); do
    h="${f##*-}"; nom="${f%-*}"
    rm -f -- "$deps/$nom-$h" "$deps/$nom-$h.d" "$deps/$nom-$h".*.dwo
    efface=$((efface + 1))
  done
  # Le binaire unique : le plus récent est gardé, les empreintes d'avant partent.
  for nom in $(find "$deps" -maxdepth 1 -type f -perm -u+x -name 'e2e-????????????????' -printf '%f\n' | sed 's/-[0-9a-f]*$//' | sort -u); do
    garde="$(ls -t "$deps/$nom"-???????????????? 2>/dev/null | head -1)"
    for f in "$deps/$nom"-????????????????; do
      [ -f "$f" ] && [ "$f" != "$garde" ] || continue
      h="${f##*-}"
      rm -f -- "$deps/$nom-$h" "$deps/$nom-$h.d" "$deps/$nom-$h".*.dwo
      efface=$((efface + 1))
    done
  done
  [ "$efface" -gt 0 ] && echo "▸ $efface vieux binaires de test effacés (le plus récent de chaque suite est gardé)"
  return 0
}
a_la_sortie() {
  [ -n "$CHARGE_PID" ] && kill "$CHARGE_PID" 2>/dev/null || true
  # Sans quiescence ni archive, conserver les parents : aucun effacement sous un écrivain.
  if ! arreter_les_demons_de_la_passe || ! archiver_journaux_des_demons; then
    echo "✗ fin de passe incomplète ; dossiers conservés : ${E2E_TMP_PARENT:-} ${E2E_DISQUE_PARENT:-}" >&2
    exit 101
  fi
  balayer_les_vieux_binaires
  # Le parent des dossiers de la passe (les suites ont effacé les leurs) :
  # même une passe interrompue nettoie. `RAG3WEAVER_GARDER=1` le garde.
  local parent
  for parent in "${E2E_TMP_PARENT:-}" "${E2E_DISQUE_PARENT:-}"; do
    [ -n "$parent" ] && [ -d "$parent" ] || continue
    if [ "${RAG3WEAVER_GARDER:-}" = 1 ]; then
      echo "▸ dossiers de la passe gardés (RAG3WEAVER_GARDER=1) : $parent"
    else
      rm -rf -- "$parent"
    fi
  done
}
trap a_la_sortie EXIT

# Parse flags
BUILD_ONLY=false
FORCE_BUILD=false
NO_CUDA=false
SUMMARY=false
TEST_FILES=()
EXTRA_FEATURES=""
TEST_FILTER=""
EXTRA_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --build-only) BUILD_ONLY=true; FORCE_BUILD=true; shift ;;
    --build)      FORCE_BUILD=true; shift ;;
    --no-build)   shift ;;  # kept for compat, now the default
    --no-cuda)    NO_CUDA=true; shift ;;
    --summary)    SUMMARY=true; shift ;;
    # **`--test` s'accumule.** Il écrasait : quatre `--test` n'en lançaient
    # qu'un, et le résumé affichait un TOTAL vert pour un quart du travail
    # demandé — la même famille que le `--no-fail-fast` ci-dessous, un total
    # partiel qui ressemble à un total complet (18 septembre 2026).
    --test)       shift; TEST_FILES+=("$1"); shift ;;
    --features)   shift; EXTRA_FEATURES="$1"; shift ;;
    -*)           EXTRA_ARGS+=("$1"); shift ;;
    *)            TEST_FILTER="$1"; shift ;;
  esac
done

# ── Build ──────────────────────────────────────────────────────────────────
# By default, skip build if librag3db.so already exists.
# Use --build to force rebuild (e.g. after changing rag3db C++ or extensions).

NEED_BUILD=false
if [ "$FORCE_BUILD" = true ]; then
  NEED_BUILD=true
elif [ ! -f "$LIBDIR/librag3db.so" ]; then
  # **Pas de bâti du moteur implicite** (10 octobre 2026) : il partait ici
  # sans rien demander — vingt minutes de C++, la mémoire du poste, et un
  # moteur de plus dans un dossier que personne ne nommait.
  echo "✗ pas de lib sous $LIBDIR : rien n'est bâti sans le demander."
  echo "  RAG3DB_BUILD vers une lib existante, ou --build pour bâtir celle-ci."
  exit 2
fi

if [ "$NEED_BUILD" = true ]; then
  # Configure (or reconfigure)
  if [ ! -f "$BUILD/Makefile" ]; then
    echo "▸ Configuring native-test build..."
    mkdir -p "$BUILD"
    cd "$BUILD"
    cmake "$ROOT" \
      -DCMAKE_BUILD_TYPE=Release \
      -DBUILD_EXTENSIONS="vector;geo" \
      -DBUILD_SHELL=FALSE \
      -DBUILD_TESTS=FALSE \
      -DBUILD_EXTENSION_TESTS=FALSE
  fi

  echo "▸ Building rag3db + extensions..."
  # Deux cœurs restent à la machine : c'est la compilation C++ qui fige le
  # poste, pas les tests (voir .cargo/config.toml à la racine).
  JOBS=$(( $(nproc) > 2 ? $(nproc) - 2 : 1 ))
  confined cmake --build "$BUILD" -j"$JOBS"
  echo "▸ Build done."
  # La lib neuve au magasin, si elle porte son identifiant (refus dit sinon) ;
  # `courant` la désigne désormais.
  if python3 "$WEAVER/scripts/rag3db_libs.py" publier "$BUILD" --racine "$ROOT"; then
    if [ "$MAGASIN" = true ] && [ "${RAG3DB_LIB:-courant}" = courant ]; then
      LIBDIR="$(lib_du_magasin courant)" || exit 2
    fi
  else
    echo "▸ lib non publiée au magasin (voir ci-dessus) ; la passe garde $LIBDIR"
  fi
fi

if [ "$BUILD_ONLY" = true ]; then
  echo "✓ Build complete: $BUILD/src/librag3db.so"
  if [ -n "$CHARGE_PID" ]; then
    kill "$CHARGE_PID" 2>/dev/null || true
    CHARGE_PID=""
    echo "CHARGE"
    "$WEAVER/charge.py" --resume "$CHARGE_LOG" || true
  fi
  exit 0
fi

# ── Run tests ──────────────────────────────────────────────────────────────

cd "$WEAVER"

# Build the cargo test filter args
# Chemin produit : burn (wgpu — AMD/NVIDIA/Apple, un seul code). candle n'est
# plus une feature des E2E ; --no-cuda est accepté pour compatibilité et sans effet.
# Tout l'arsenal burn est dans le jeu : une suite qui ne tourne pas n'existe
# pas. burn-embedder (BGE-M3) et burn-ocr (PP-OCRv6 tiny) chargent leurs poids
# depuis ~/.cache/rag3weaver/ — téléchargés au premier passage.
# `--features a,b` ajoute au jeu.
#
# **Plus de `burn-llm`** (28 août 2026) : notre moteur ne fait pas d'inférence
# de LLM. Il fait l'embedding, le rerank, l'OCR — ce pour quoi un graphe burn
# local a un sens. Un LLM vient de llama.cpp ou d'un fournisseur distant.
# `daemon` fait partie du socle depuis le 29 août : `tests/common/mod.rs` choisit
# entre BGE-M3 chargé ici et le démon qui le sert, et son type `Bge` référence
# `DaemonEmbedder` sans condition. Sans la feature, **aucune suite utilisant
# burn ne compile** — l'oubli a survécu parce que les suites qui n'y touchent
# pas (e2e_code) passaient très bien.
# **Un seul jeu de features, quoi qu'il arrive** (10 octobre 2026) : `postgres`
# y est toujours. Il n'y entrait que si le conteneur répondait, et changer de
# jeu change le hachage de **tous** les binaires de test — une passe avec le
# conteneur puis une sans rebâtissaient tout, et gardaient les deux (un
# binaire e2e pèse des centaines de Mo). Ses dépendances sont du Rust pur ;
# seule la suite `e2e_postgres` dépend du conteneur, et elle est écartée sans
# lui (ci-dessous).
# **Le jeu de la procédure, par défaut** (7 novembre 2026) : `openai-llm` et
# `anthropic-llm` y entrent. Sans elles, un test sous `#[cfg(feature =
# "openai-llm")]` dans une suite par ailleurs jouée disparaissait du compte sans
# rien dire : « 3 passed » sur une suite de quatre (le témoin de `models.llm`,
# 662dda2cc, n'a jamais été joué par les passes qui l'ont poussé). Les suites qui
# appellent un vrai modèle restent écartées par leur **nom** (plus bas), plus par
# une feature. `RAG3WEAVER_E2E_FEATURES` remplace le jeu entier, en le disant.
FEATURES_PAR_DEFAUT="rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm"
FEATURES="${RAG3WEAVER_E2E_FEATURES:-$FEATURES_PAR_DEFAUT}${EXTRA_FEATURES:+,$EXTRA_FEATURES}"
if [ -n "${RAG3WEAVER_E2E_FEATURES:-}" ]; then
  echo "▸ ⚠ jeu de features remplacé par RAG3WEAVER_E2E_FEATURES : $FEATURES (celui de la procédure : $FEATURES_PAR_DEFAUT)"
fi

# ── PostgreSQL : dans la passe, ou absent **en le disant** ──────────────────
#
# `e2e_postgres` ne tournait pas ici : sa feature n'était pas dans le jeu, donc
# ses dix-sept tests étaient hors de toute passe complète. On les lançait à la
# main — c'est-à-dire qu'on y pensait, ou pas. Le backend qu'on a passé le
# 3 septembre 2026 à construire n'était couvert par rien d'automatique.
#
# La suite ne se saute jamais en silence : sans base, chacun de ses tests
# échoue en disant comment la démarrer. Faire échouer une passe entière parce
# qu'un conteneur dort serait pourtant excessif. D'où la sonde : si postgres
# répond, la feature entre et la suite tourne ; sinon on l'annonce **une fois,
# en clair**, et le résumé le rappelle. Un saut annoncé n'est pas un silence.
PG_DANS_LA_PASSE=false
PG_RAISON=""
PG_URL="${RAG3WEAVER_PG:-postgres://rag3weaver:rag3weaver@localhost:5433/rag3weaver_test}"
# Les deux formes que lit tokio-postgres : l'URL (`postgres://u:p@hôte:port/base`) et les
# mots-clés (`host=… port=…`, port 5432 par défaut). Une adresse que la sonde ne sait pas lire se
# dit telle, jamais « rien n'écoute » : le 25 octobre 2026, la forme à mots-clés sondait
# « host=…:host=… » et sautait la suite en accusant un conteneur bien vivant.
PG_HOTE=""; PG_PORT=""
case "$PG_URL" in
  postgres://*|postgresql://*)
    PG_HOTE=$(printf '%s' "$PG_URL" | sed -nE 's|^[a-z]+://([^@/]*@)?(\[[^]]+\]\|[^:/?]+).*|\2|p' | tr -d '[]')
    PG_PORT=$(printf '%s' "$PG_URL" | sed -nE 's|^[a-z]+://([^@/]*@)?(\[[^]]+\]\|[^:/?]+):([0-9]+).*|\3|p')
    [ -n "$PG_HOTE" ] && PG_PORT=${PG_PORT:-5432} ;;
  *host=*)
    PG_HOTE=$(printf '%s' " $PG_URL" | sed -nE 's|.*[[:space:]]host=([^[:space:]]+).*|\1|p')
    PG_PORT=$(printf '%s' " $PG_URL" | sed -nE 's|.*[[:space:]]port=([0-9]+).*|\1|p')
    [ -n "$PG_HOTE" ] && PG_PORT=${PG_PORT:-5432} ;;
esac
# `/dev/tcp` de bash plutôt que `nc` : la première version employait `nc`, qui
# n'est **pas installé ici**. La sonde ne pouvait donc jamais répondre oui, et
# aurait écarté postgres pour toujours en croyant l'avoir vérifié — le défaut
# même qu'elle est censée prévenir, dans l'outil qui le prévient.
if [ -z "${RAG3WEAVER_SANS_PG:-}" ] && { [ -z "$PG_HOTE" ] || ! [[ "$PG_PORT" =~ ^[0-9]+$ ]]; }; then
  PG_RAISON="adresse RAG3WEAVER_PG illisible : « $PG_URL » (attendu postgres://u:p@hôte:port/base ou host=… port=…)"
elif [ -z "${RAG3WEAVER_SANS_PG:-}" ]; then
  if timeout 2 bash -c "echo > /dev/tcp/$PG_HOTE/$PG_PORT" 2>/dev/null; then
    PG_DANS_LA_PASSE=true
  else
    PG_RAISON="rien n'écoute sur $PG_HOTE:$PG_PORT — \`docker start rag3weaver-pg\`"
  fi
else
  PG_RAISON="RAG3WEAVER_SANS_PG posée"
fi
if [ "$PG_DANS_LA_PASSE" = false ]; then
  echo "▸ ⚠ e2e_postgres NON JOUÉE : $PG_RAISON"
fi

# ── Les suites qui appellent un vrai modèle ────────────────────────────────
#
# Elles dépensent le quota Vertex à chaque passe : une passe complète les écarte
# **par leur nom**, l'annonce au départ et le rappelle au résumé. Nommées par
# `--test`, ou avec `RAG3WEAVER_SUITES_A_MODELE=1`, elles jouent. Avant le
# 7 novembre 2026, c'est l'absence de la feature `openai-llm` qui les écartait,
# et elle écartait du même coup, sans le dire, tout test sous cette feature
# dans les autres suites.
SUITES_A_MODELE=(e2e_avis_du_modele e2e_cloud_code_agent e2e_cloud_schema_probe e2e_conversation_a_plusieurs e2e_lecture_mermaid e2e_banc_vertex_embedding)
ECARTEES_MODELE=()
est_une_suite_a_modele() {
  local s
  for s in "${SUITES_A_MODELE[@]}"; do [ "$s" = "$1" ] && return 0; done
  return 1
}

# ── Les suites dont l'objet est la carte d'ici ─────────────────────────────
#
# Quatre familles court-circuitent exprès le service d'embarquement distant
# (`SUITES_LOCALES`, `tests/common/mod.rs`) : leur objet est l'embarqueur, le
# démon, ou la vitesse de *ce* poste. Les envoyer au service leur ferait
# mesurer la carte d'un autre poste, un chiffre juste pour une question qu'on
# ne leur pose pas. Aucun régime ne les rend donc légères : elles chargent
# leur modèle sur la carte qui porte l'écran, par le démon ou sur place.
#
# Décision de l'orchestration (4 octobre 2026, après un gel d'écran) : une
# batterie de jour les écarte, `RAG3WEAVER_SANS_CARTE_LOCALE=0` les fait
# entrer (la nuit, ou sur demande de Lucie ; une livraison qui touche
# l'embarqueur, le démon ou le moteur burn les exige avant fusion). Une suite
# nommée par `--test` se joue toujours : on l'a demandée. Une passe qui les
# écarte se dit « complète hors carte locale », jamais « complète ».
SUITES_CARTE_LOCALE=(e2e_burn_ e2e_demon_embeddings e2e_mesure_ingestion_code e2e_banc_bge_m3)
SANS_CARTE_LOCALE="${RAG3WEAVER_SANS_CARTE_LOCALE:-1}"
ECARTEES_CARTE_LOCALE=()
# **Les suites écartées à la demande** (`RAG3WEAVER_E2E_SAUF="<suite> …"`, 9 novembre 2026) : une
# mesure qui tient la carte (e2e_charge_ingestion, près de deux heures) se joue seule, sous
# `poste mesure`, pas à chaque passe entière. Elles sont dites NON JOUÉES, jamais tues.
ECARTEES_SAUF=()
# ── Les bancs à services ────────────────────────────────────────────────────
#
# Un banc dont l'objet est un service que la batterie ne pose pas (un
# llama-server, Vertex) refuse sans lui, et c'est juste : un banc sauté n'est
# pas un banc vert. **Il ne fait jamais partie de l'e2e entier** : une passe
# complète l'écarte en le disant, ses variables posées ou non, nommé au départ
# et au résumé, et la passe se dit « complète hors bancs à services ». Il se
# joue **seul**, nommé par `--test`, avec sa voie (décision de l'orchestration,
# 9 novembre 2026 : posé pour l'e2e entier, il rougissait d'environnement toute
# passe qui l'embarquait sans sa voie, jusqu'au rejeu de la session recherche).
# Nommé sans aucune de ses variables, ou avec une variable dont la feature
# manque au jeu, il est refusé ROUGE avant tout bâti, en disant quoi poser.
# Forme : `suite:VAR1|VAR2` — l'une des variables suffit à la jouer ;
# `VOIES_FEATURES` dit la feature qu'une variable exige (sans elle, la voie
# n'est pas compilée et le banc n'a aucune voie).
BANCS_A_SERVICES=("e2e_banc_fiches_classees:RAG3WEAVER_BANC_DECIDE|RAG3WEAVER_BANC_VERTEX")
VOIES_FEATURES=("RAG3WEAVER_BANC_DECIDE:daemon" "RAG3WEAVER_BANC_VERTEX:openai-llm")
ECARTEES_BANCS=()
banc_sans_service() {
  local entree vars v
  for entree in "${BANCS_A_SERVICES[@]}"; do
    [ "${entree%%:*}" = "$1" ] || continue
    vars="${entree#*:}"
    IFS='|' read -r -a vs <<< "$vars"
    for v in "${vs[@]}"; do
      [ -n "${!v:-}" ] && return 1
    done
    return 0
  done
  return 1
}
est_un_banc_a_services() {
  local entree
  for entree in "${BANCS_A_SERVICES[@]}"; do
    [ "${entree%%:*}" = "$1" ] && return 0
  done
  return 1
}
# Les voies posées d'un banc dont la feature manque au jeu, une par ligne.
voies_sans_feature() {
  local entree vars v f
  for entree in "${BANCS_A_SERVICES[@]}"; do
    [ "${entree%%:*}" = "$1" ] || continue
    IFS='|' read -r -a vs <<< "${entree#*:}"
    for v in "${vs[@]}"; do
      [ -n "${!v:-}" ] || continue
      for f in "${VOIES_FEATURES[@]}"; do
        [ "${f%%:*}" = "$v" ] || continue
        case ",$FEATURES," in *",${f#*:},"*) ;; *) echo "$v (feature ${f#*:})" ;; esac
      done
    done
  done
}
services_du_banc() {
  local entree
  for entree in "${BANCS_A_SERVICES[@]}"; do
    [ "${entree%%:*}" = "$1" ] && { echo "${entree#*:}" | sed 's/|/ ou /g'; return 0; }
  done
}

est_de_la_carte_locale() {
  local motif
  for motif in "${SUITES_CARTE_LOCALE[@]}"; do
    case "$1" in "$motif"*) return 0 ;; esac
  done
  return 1
}

CARGO_ARGS=(
  --features "$FEATURES"
)

if [ ${#TEST_FILES[@]} -gt 0 ]; then
  for nom in "${TEST_FILES[@]}"; do
    if est_un_banc_a_services "$nom"; then
      if banc_sans_service "$nom"; then
        echo "✗ $nom est un banc à services : il se joue seul, avec sa voie — posez $(services_du_banc "$nom") (procédure du push, « Les bancs »)"
        exit 1
      fi
      sans=$(voies_sans_feature "$nom")
      if [ -n "$sans" ]; then
        echo "✗ $nom : une voie posée n'est pas compilée dans le jeu ($FEATURES) : $(echo $sans) — ajoutez la feature (--features …)"
        exit 1
      fi
    fi
    CARGO_ARGS+=(--test "$nom")
  done
else
  # Run ALL e2e test files
  for f in "$WEAVER"/tests/e2e_*.rs; do
    nom="$(basename "${f%.rs}")"
    # Sans la feature, ce binaire ne compile même pas : l'exclure est la seule
    # option, et c'est pour ça que l'absence est annoncée plus haut.
    if [ "$nom" = "e2e_postgres" ] && [ "$PG_DANS_LA_PASSE" = false ]; then
      continue
    fi
    if [[ " ${RAG3WEAVER_E2E_SAUF:-} " == *" $nom "* ]]; then
      ECARTEES_SAUF+=("$nom")
      continue
    fi
    if [ "$SANS_CARTE_LOCALE" != 0 ] && est_de_la_carte_locale "$nom"; then
      ECARTEES_CARTE_LOCALE+=("$nom")
      continue
    fi
    if est_un_banc_a_services "$nom"; then
      ECARTEES_BANCS+=("$nom")
      continue
    fi
    if [ "${RAG3WEAVER_SUITES_A_MODELE:-}" != 1 ] && est_une_suite_a_modele "$nom"; then
      ECARTEES_MODELE+=("$nom")
      continue
    fi
    CARGO_ARGS+=(--test "$nom")
  done
  if [ ${#ECARTEES_MODELE[@]} -gt 0 ]; then
    echo "▸ ⚠ ${#ECARTEES_MODELE[@]} suites NON JOUÉES (elles appellent un vrai modèle ; --test <nom> ou RAG3WEAVER_SUITES_A_MODELE=1 pour les jouer) :"
    echo "    ${ECARTEES_MODELE[*]}"
  fi
  if [ ${#ECARTEES_CARTE_LOCALE[@]} -gt 0 ]; then
    echo "▸ ⚠ ${#ECARTEES_CARTE_LOCALE[@]} suites NON JOUÉES (carte locale, RAG3WEAVER_SANS_CARTE_LOCALE=0 pour les jouer) :"
    echo "    ${ECARTEES_CARTE_LOCALE[*]}"
  fi
  if [ ${#ECARTEES_SAUF[@]} -gt 0 ]; then
    echo "▸ ⚠ ${#ECARTEES_SAUF[@]} suites NON JOUÉES (écartées par RAG3WEAVER_E2E_SAUF) :"
    echo "    ${ECARTEES_SAUF[*]}"
  fi
  for nom in "${ECARTEES_BANCS[@]}"; do
    echo "▸ ⚠ $nom NON JOUÉE (banc à services, il se joue seul : --test $nom avec $(services_du_banc "$nom"))"
  done
fi

# ── Une suite ne se joue pas à moitié ──────────────────────────────────────
#
# Chaque suite demandée est lue avant tout bâti : une feature qu'elle exige
# (`#![cfg(...)]` du fichier, ou `#[cfg(...)]` d'un test) et que le jeu n'a pas
# la rendrait muette en tout ou en partie, avec un total vert. C'est **rouge**,
# en nommant la suite et la feature. On ne lit que les `cfg` positifs : un
# `not(...)` ou un `any(...)` ne se juge pas ici.
ABSENTES=()
for ((i = 0; i < ${#CARGO_ARGS[@]}; i++)); do
  [ "${CARGO_ARGS[$i]}" = "--test" ] || continue
  nom="${CARGO_ARGS[$((i + 1))]}"
  fichier="$WEAVER/tests/$nom.rs"
  [ -f "$fichier" ] || continue
  for x in $(grep -E '^[[:space:]]*#!?\[cfg\(' "$fichier" | grep -v -e 'not(' -e 'any(' | grep -oP 'feature = "\K[A-Za-z0-9_-]+' | sort -u); do
    case ",$FEATURES," in
      *",$x,"*) ;;
      *) ABSENTES+=("$nom ($x)") ;;
    esac
  done
done
if [ ${#ABSENTES[@]} -gt 0 ]; then
  echo "✗ ${#ABSENTES[@]} suite(s) ne joueraient pas tous leurs tests : une feature qu'elles exigent manque au jeu ($FEATURES)"
  for a in "${ABSENTES[@]}"; do echo "    · $a"; done
  echo "  Ajoutez-la (--features <nom>), ou ne demandez pas la suite."
  exit 1
fi
if [ "${RAG3WEAVER_E2E_VERIFIER_SEULEMENT:-}" = 1 ]; then
  echo "▸ features : chaque suite demandée a les siennes ($FEATURES) — rien n'est bâti ni joué"
  exit 0
fi

# Une suite en échec n'arrête pas les autres : sans ça, cargo s'arrête au
# premier binaire de test qui échoue, et le résumé affiche un total partiel
# qui ressemble à un total complet (25 août 2026 : « 89 passed » pour 17
# suites sur 28).
CARGO_ARGS+=(--no-fail-fast)
# **Tous les tests de la suite**, ignorés ou non : `--ignored` seul écartait
# ceux sans `#[ignore]`, et une suite qui n'en a pas rendait « 0 passed »
# sans un mot (`usages_rendu`, 3 octobre 2026).
CARGO_ARGS+=(-- --include-ignored --nocapture)

if [ -n "$TEST_FILTER" ]; then
  CARGO_ARGS+=("$TEST_FILTER")
fi

# **Les suites demandées, pour les comptes** (8 novembre 2026). `--test e2e`
# nomme le binaire unique, pas une suite : cargo y écrit `Running
# tests/e2e/main.rs`, et le compte qui cherchait `tests/e2e.rs` disait « 0 sur
# 1 » d'une passe jouée. Avec `--test e2e`, c'est le filtre qui désigne la
# suite (`e2e_lectures`, `e2e_lectures::un_test`) ; sans filtre de suite, le
# binaire compte pour une. Un `--test <autre>` garde la règle d'avant : sa
# ligne `Running tests/<autre>.rs`. `SUITES_BINAIRE[i]` est le binaire dont la
# ligne vaut aussi pour `SUITES_DEMANDEES[i]` (vide sinon).
SUITES_DEMANDEES=(); SUITES_BINAIRE=()
for ((i = 0; i < ${#CARGO_ARGS[@]}; i++)); do
  [ "${CARGO_ARGS[$i]}" = "--" ] && break
  [ "${CARGO_ARGS[$i]}" = "--test" ] || continue
  nom="${CARGO_ARGS[$((i + 1))]}"
  if [ "$nom" = "e2e" ]; then
    suite="${TEST_FILTER%%::*}"
    if [[ "$suite" =~ ^e2e_[A-Za-z0-9_]+$ ]]; then
      SUITES_DEMANDEES+=("$suite")
    else
      SUITES_DEMANDEES+=("e2e")
    fi
    SUITES_BINAIRE+=("e2e")
  else
    SUITES_DEMANDEES+=("$nom"); SUITES_BINAIRE+=("")
  fi
done
compter_les_suites() {
  ATTENDUES=0; JOUEES=0; MANQUANTES=()
  local i nom binaire
  for ((i = 0; i < ${#SUITES_DEMANDEES[@]}; i++)); do
    nom="${SUITES_DEMANDEES[$i]}"; binaire="${SUITES_BINAIRE[$i]}"
    ATTENDUES=$((ATTENDUES + 1))
    if grep -qF "Running tests/$nom.rs" "$1" \
       || { [ -n "$binaire" ] && grep -qF "Running tests/$binaire/main.rs" "$1"; }; then
      JOUEES=$((JOUEES + 1))
    else
      MANQUANTES+=("$nom")
    fi
  done
  echo "Suites jouées : $JOUEES sur $ATTENDUES demandées."
}
# Pour le témoin (scripts/test_run_e2e.sh) : le compte seul, sur un journal
# donné, sans rien bâtir ni jouer.
if [ -n "${RAG3WEAVER_E2E_COMPTER_JOURNAL:-}" ]; then
  compter_les_suites "$RAG3WEAVER_E2E_COMPTER_JOURNAL"
  [ ${#MANQUANTES[@]} -eq 0 ] || echo "manquantes : ${MANQUANTES[*]}"
  exit 0
fi

CARGO_ARGS+=("${EXTRA_ARGS[@]}")

# Espace d'adressage : une base en mémoire réserve 1 TiB (Rag3dbConnection::
# IN_MEMORY_MAX_DB_SIZE), pas les 8 TiB de kuzu — sinon 24 tests parallèles
# dans un même processus dépassent les 128 TiB adressables et `in_memory()`
# échoue au hasard. Le script ne force rien : c'est le défaut de la
# bibliothèque qui est testé ici. RAG3DB_MAX_DB_SIZE reste surchargeable.

# **Contre quoi la passe tourne.** Un résultat qui ne dit pas contre quoi il a
# été obtenu n'est pas un résultat (3 octobre 2026 : un SIGSEGV qui avait
# l'air d'une régression venait d'une bibliothèque bâtie 1 h 38 avant le
# correctif qu'on vérifiait). En tête de passe : la date et l'âge de la
# bibliothèque et de l'extension liées ; et un refus si elles sont plus
# vieilles que le dernier commit des sources du moteur (le commit résiste à
# un `touch`). `RAG3WEAVER_MOTEUR_ANCIEN=1` passe outre, en le disant.
# **L'extension vector** : bâtie dans l'arbre principal, pas dans un worktree.
# `RAG3DB_ROOT` désigne l'arbre où elle est, comme pour les tests ; sinon
# l'arbre du script.
EXTENSION_VECTOR="${RAG3DB_ROOT:-$ROOT}/extension/vector/build/libvector.rag3db_extension"
# **Sous le magasin, la paire vient du même bâti** : le backend et les suites
# (common::vector_extension, lot 3c) cherchent l'extension à côté de la lib.
# Une extension de l'arbre d'un autre bâti n'est plus chargée par elles ; elle
# est seulement dite, pour une suite qui serait écrite à l'ancienne.
if [ "$MAGASIN" = true ]; then
  ID_MAGASIN="$(cat "$LIBDIR/BUILD_ID" 2>/dev/null)"
  if [ -f "$EXTENSION_VECTOR" ] && ! grep -qaF "$ID_MAGASIN" "$EXTENSION_VECTOR"; then
    echo "▸ note : l'extension de l'arbre ($EXTENSION_VECTOR) n'est pas du bâti du magasin ($ID_MAGASIN) ;"
    echo "  les suites prennent celle du magasin. Une suite qui la chargerait par RAG3DB_ROOT serait refusée par le moteur."
  fi
  EXTENSION_VECTOR="$LIBDIR/libvector.rag3db_extension"
fi

verifier_le_moteur() {
  local lib="$LIBDIR/librag3db.so" ext="$EXTENSION_VECTOR"
  local sources_ct sources_de vieux=0 f ct chemins
  # Chaque bibliothèque se compare aux sources qui la bâtissent : le moteur à
  # `src/`, l'extension vecteur à ses propres sources et aux en-têtes du
  # moteur. Un commit du seul `src/storage` ne relinke pas l'extension, et la
  # comparer à lui faisait refuser une extension à jour (4 octobre 2026).
  for f in "$lib" "$ext"; do
    if [ "$f" = "$lib" ]; then chemins="src"; else chemins="extension/vector/src src/include"; fi
    # shellcheck disable=SC2086
    sources_ct=$(git -C "$ROOT" log -1 --format=%ct -- $chemins 2>/dev/null)
    # shellcheck disable=SC2086
    sources_de=$(git -C "$ROOT" log -1 --format='%h du %cd' --date=format:'%d/%m %H:%M' -- $chemins 2>/dev/null)
    if [ ! -f "$f" ]; then
      echo "⚠ introuvable : $f — depuis un worktree, RAG3DB_ROOT désigne l'arbre où le moteur est bâti"
      continue
    fi
    ct=$(stat -c %Y "$f")
    echo "▸ moteur lié : $f — bâti le $(date -d "@$ct" '+%d/%m %H:%M'), il y a $(( ($(date +%s) - ct) / 60 )) min"
    if [ -n "$sources_ct" ] && [ "$ct" -lt "$sources_ct" ]; then
      echo "✗ $(basename "$f") est plus vieux que le dernier commit de ses sources ($chemins : $sources_de)"
      vieux=1
    fi
  done
  if [ "$vieux" = 1 ]; then
    if [ "${RAG3WEAVER_MOTEUR_ANCIEN:-}" = 1 ]; then
      echo "⚠ RAG3WEAVER_MOTEUR_ANCIEN=1 : la passe tourne contre un moteur d'avant ces sources — à dire avec son résultat"
    else
      echo "✗ Rebâtir : cmake --build $BUILD -j 8 (ou ./run_e2e.sh --build-only), ou RAG3WEAVER_MOTEUR_ANCIEN=1 en le sachant."
      exit 1
    fi
  fi
}
verifier_le_moteur

# **Et contre quoi elle a tourné.** La ligne d'en-tête dit contre quoi la passe
# a *commencé*. Un rebâti en cours de route est invisible : les binaires lancés
# avant tiennent l'ancien moteur, ceux d'après le neuf (4 octobre 2026, une
# batterie jetée). On relève date et taille au début et à la fin ; si elles ont
# changé, la passe ne conclut pas. Une somme du contenu, ni la date (un
# `touch` sans rebâti ferait refuser à tort) ni la taille (deux rebâtis de ce
# soir avaient la même, à l'octet).
empreinte_du_moteur() {
  local f
  for f in "$LIBDIR/librag3db.so" "$EXTENSION_VECTOR"; do
    # Un `[ -f ] && …` en dernière ligne rendait 1 quand le fichier manquait,
    # et `set -e` arrêtait le script sans un mot (4 octobre 2026, depuis un
    # worktree). La fonction ne doit jamais échouer : elle relève ce qui est.
    if [ -f "$f" ]; then cksum "$f"; else echo "absent $f"; fi
  done
  return 0
}
MOTEUR_AU_DEBUT="$(empreinte_du_moteur)"
moteur_inchange() {
  if [ "$(empreinte_du_moteur)" != "$MOTEUR_AU_DEBUT" ]; then
    echo "✗ le moteur a été remplacé pendant la passe : ce résultat ne vaut rien, rejoue"
    return 1
  fi
  return 0
}

# **Zéro test n'est pas une réussite** — dans les deux modes. Sans filtre,
# chaque suite demandée doit jouer au moins un test ; avec un filtre, c'est la
# passe entière (un filtre qui ne désigne rien est une faute de frappe, pas
# un succès). Rend 1 et le dit si la règle est enfreinte.
suites_vides() {
  local journal="$1" vides total
  vides=$(grep -c '^test result: .* 0 passed; 0 failed;' "$journal" || true)
  total=0
  # Les tests joués, réussis ou non : un filtre qui désigne un test en échec
  # désigne bien quelque chose.
  for n in $(grep '^test result:' "$journal" | grep -oP '\d+(?= (passed|failed);)'); do total=$((total + n)); done
  if [ -z "$TEST_FILTER" ] && [ "${vides:-0}" -gt 0 ]; then
    echo "✗ ${vides} suite(s) n'ont joué aucun test : feature manquante, ou fichier sans test ?"
    return 1
  fi
  if [ -n "$TEST_FILTER" ] && [ "${total:-0}" -eq 0 ]; then
    echo "✗ le filtre « $TEST_FILTER » n'a désigné aucun test"
    return 1
  fi
  return 0
}

# ── Le binaire unique des e2e, lancé une fois PAR SUITE ─────────────────────
#
# Demande de Lucie (11 octobre 2026) : « bâtis une seule fois avec le même
# cœur, pas instanciés 50 fois ». Chaque tests/e2e_*.rs était son propre
# binaire de test : 97 éditions de liens du crate entier par passe, et un
# target de plusieurs centaines de Go. Les suites sont maintenant les
# modules d'UN binaire (tests/e2e/main.rs), bâti et relié une fois.
#
# **Lancé une fois par suite**, filtré sur `<suite>::` : chaque suite garde
# son processus — ses `set_var`, ses `OnceLock`, ses statics, son démon —
# exactement comme quand elle était son propre binaire. Le gain est au bâti,
# pas à l'exécution. Les lignes « Running tests/<suite> » et « test result »
# sont celles que cargo écrivait : le résumé plus bas les lit sans changer.
#
# **Chaque suite écrit dans son dossier** : `TMPDIR` pointe sur
# `$E2E_TMP_PARENT/<suite>/`, que `std::env::temp_dir()`, `tempfile` et le
# cache des catalogues respectent tous. Le dossier part à la fin de la suite,
# le parent à la fin de la passe ; `RAG3WEAVER_GARDER=1` garde tout pour
# l'examen. Avant : 11 Go de restes d'e2e dans /tmp du poste principal —
# de la RAM — et 10,9 Go sur luciepc, mesurés et effacés le 11 octobre.
# L'état durable du produit a une durée de vie de fixture ici. Seul le chemin
# créé par mktemp appartient au lanceur ; ne jamais effacer un chemin reçu de
# RAG3WEAVER_STATE_DIR. Le marqueur vérifie cette propriété avant nettoyage.
creer_etat_suite_e2e() {
  local etat
  etat=$(mktemp -d "${1:?}/etat-temoins.XXXXXX") || return 1
  printf '%s\n' "$etat" > "$etat/.possede-par-le-lanceur-e2e" || return 1
  printf '%s\n' "$etat"
}

terminer_etat_suite_e2e() {
  local etat="${1:?}" statut="${2:?}" garder="${3:-}" archive="${4:?}"
  # Échec/expiration : l'état original reste avec les preuves de la suite.
  [ "$statut" -eq 0 ] || return 0
  if [ ! -d "$etat" ] || [ -L "$etat" ] \
     || [ "$(cat "$etat/.possede-par-le-lanceur-e2e" 2>/dev/null)" != "$etat" ]; then
    echo "✗ état de suite non possédé : nettoyage refusé ($etat)" >&2
    return 1
  fi
  if [ "$garder" = 1 ]; then
    # Conserver les octets demandés sur le disque propre à la suite, sans
    # les compter comme un résidu TMPDIR arbitraire ni changer le garde.
    if [ -e "$archive" ] || [ -L "$archive" ] || ! mv -T -- "$etat" "$archive"; then
      echo "✗ conservation de l'état de suite impossible : $etat" >&2
      return 1
    fi
    echo "▸ état possédé de la suite conservé : $archive"
  elif ! rm -rf -- "$etat"; then
    echo "✗ nettoyage de l'état possédé impossible : $etat" >&2
    return 1
  fi
}

jouer_les_suites() {
  local e2e=() autres=() i nom code=0 bin statut etat_suite
  for ((i = 0; i < ${#CARGO_ARGS[@]}; i++)); do
    [ "${CARGO_ARGS[$i]}" = "--test" ] || continue
    nom="${CARGO_ARGS[$((i + 1))]}"
    case "$nom" in e2e_*) e2e+=("$nom") ;; *) autres+=("$nom") ;; esac
  done

  # 1. Bâtir une fois : le binaire e2e, et les autres tests nommés.
  local cibles=()
  [ ${#e2e[@]} -gt 0 ] && cibles+=(--test e2e)
  for nom in "${autres[@]}"; do cibles+=(--test "$nom"); done
  echo "▸ Bâti : cargo test --no-run --features $FEATURES ${cibles[*]}"
  local json
  json="$(mktemp)"
  confined cargo test --no-run --features "$FEATURES" "${cibles[@]}" \
    --message-format=json-render-diagnostics > "$json"
  statut=$?
  bin="$(python3 -c '
import json, sys
for ligne in open(sys.argv[1]):
    try:
        m = json.loads(ligne)
    except ValueError:
        continue
    if m.get("reason") == "compiler-artifact" and m.get("target", {}).get("name") == "e2e" and m.get("executable"):
        print(m["executable"])
' "$json")"
  rm -f "$json"
  if [ "$statut" -ne 0 ]; then
    echo "✗ le bâti a échoué (code $statut)"
    return 101
  fi
  if [ ${#e2e[@]} -gt 0 ] && [ ! -x "$bin" ]; then
    # Sans ce refus, chaque suite échouerait sur une commande vide.
    echo "✗ binaire e2e introuvable dans la sortie de cargo (cible « e2e », tests/e2e/main.rs)"
    return 101
  fi
  if [ -n "$bin" ]; then
    local mo ref
    mo=$(( $(stat -c %s "$bin") / 1024 / 1024 ))
    echo "▸ binaire e2e : $bin (${mo} Mo)"
    # La référence commitée : une dérive se voit à la passe qui la crée.
    ref=$(sed -n 's/^binaire_e2e_mo=//p' "$WEAVER/tests/e2e/poids-reference.txt" 2>/dev/null)
    if [ -n "$ref" ] && [ "$mo" -gt $(( ref * 3 / 2 )) ]; then
      echo "▸ ⚠ le binaire e2e pèse ${mo} Mo, plus de 1,5 fois sa référence (${ref} Mo,"
      echo "    tests/e2e/poids-reference.txt) : une dérive à expliquer, ou une référence à remesurer"
    fi
  fi

  # 2. Les filets du binaire lui-même (une suite montée, aucune relance par un
  # nom court) : à sa racine, hors de tout `<suite>::`, aucun filtre de suite
  # ne les désigne — ils jouent donc ici, en tête, à chaque passe.
  if [ ${#e2e[@]} -gt 0 ]; then
    echo "     Running tests/e2e_filets.rs (binaire e2e unique, ses propres filets)"
    CARGO_MANIFEST_DIR="$WEAVER" "$bin" --exact chaque_suite_est_montee \
      aucune_suite_ne_se_relance_par_son_nom_court --include-ignored || code=101
  fi

  # 3. Chaque suite e2e : son processus, son dossier.
  for nom in "${e2e[@]}"; do
    mkdir -p "$E2E_TMP_PARENT/$nom" "$E2E_DISQUE_PARENT/$nom/checkpoints"
    export RAG3WEAVER_E2E_DOSSIER="$E2E_DISQUE_PARENT/$nom"
    export RAG3WEAVER_CHECKPOINT_DIR="$E2E_DISQUE_PARENT/$nom/checkpoints"
    # Les caches d'index de la suite (catalog::index_cache_dir) : sur le disque, à elle, effacés
    # avec elle ; ni dans son TMPDIR, ni dans le vrai cache de la personne.
    export RAG3WEAVER_CACHE_DIR="$E2E_DISQUE_PARENT/$nom/cache"
    # L'heure de départ de chaque suite : un pic du journal de charge
    # (échange, carte) s'attribue à la suite qui tournait.
    echo "     [départ $(date +%H:%M:%S)] $nom"
    echo "     Running tests/$nom.rs (binaire e2e unique, filtre $nom::)"
    etat_suite=$(creer_etat_suite_e2e "$E2E_TMP_PARENT/$nom") || return 101
    if [ -n "$TEST_FILTER" ]; then
      local noms=()
      mapfile -t noms < <("$bin" --list --format terse 2>/dev/null \
        | sed -n 's/: test$//p' | grep "^$nom::" | grep -F -- "$TEST_FILTER")
      if [ ${#noms[@]} -eq 0 ]; then
        # La ligne que cargo écrit quand le filtre n'y désigne rien.
        printf '\nrunning 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n'
        rm -rf -- "${E2E_TMP_PARENT:?}/$nom" "${E2E_DISQUE_PARENT:?}/$nom"
        continue
      fi
      TMPDIR="$E2E_TMP_PARENT/$nom" RAG3WEAVER_STATE_DIR="$etat_suite" CARGO_MANIFEST_DIR="$WEAVER" \
        confined "$bin" --exact "${noms[@]}" --include-ignored --nocapture "${EXTRA_ARGS[@]}"
    else
      TMPDIR="$E2E_TMP_PARENT/$nom" RAG3WEAVER_STATE_DIR="$etat_suite" CARGO_MANIFEST_DIR="$WEAVER" \
        confined "$bin" "$nom::" --include-ignored --nocapture "${EXTRA_ARGS[@]}"
    fi
    statut=$?
    terminer_etat_suite_e2e "$etat_suite" "$statut" "${RAG3WEAVER_GARDER:-}" \
      "$E2E_DISQUE_PARENT/$nom/etat-temoins" || statut=101
    [ "$statut" -ne 0 ] && code=101
    # **Une suite rend son TMPDIR vide** (ticket « les tests ne nettoient pas leurs
    # dossiers temporaires », 8 novembre 2026) : /tmp est de la mémoire vive, et ce
    # qu'une suite y laisse hors de run_e2e.sh y reste. Ce qu'elle laisse ici se dit,
    # chemin et taille ; hors de la liste des suites connues pour en laisser, la suite
    # est rouge. Une suite de la liste qui finit propre (passe entière, sans filtre)
    # est rouge aussi : elle sort de la liste dans le commit qui la rend propre.
    residus_de_la_suite "$nom" || { statut=101; code=101; }
    # **Une suite rouge garde son dossier disque** (relecture du banc, 11
    # octobre 2026) : un rouge d'arrêt brutal tombe une fois sur vingt ou
    # trente, et sa base est ce qu'on vient chercher — `GARDER=1` demanderait
    # de prévoir le rouge avant de lancer. Déplacé (mv, sans copie) hors du
    # parent de la passe, sous e2e-gardes/ que le balayage des passes mortes
    # (e2e-<pid>) ne touche pas ; les cinq derniers par suite seulement. Une
    # suite verte reste effacée.
    if [ "$statut" -ne 0 ] && [ "${RAG3WEAVER_GARDER:-}" != 1 ] \
       && [ -d "$E2E_DISQUE_PARENT/$nom" ]; then
      local gardes="$E2E_DISQUE_RACINE/e2e-gardes" garde
      garde="$gardes/$nom-$(date +%Y%m%d-%H%M%S)-$$"
      if mkdir -p "$gardes" && mv -- "$E2E_DISQUE_PARENT/$nom" "$garde" \
         && mv -- "$E2E_TMP_PARENT/$nom" "$garde/tmpdir"; then
        echo "▸ $nom a rougi : ses dossiers disque et TMPDIR sont gardés dans $garde"
      else
        echo "✗ archive de $nom incomplète ; parents conservés" >&2
        export RAG3WEAVER_GARDER=1
        code=101
      fi
      ls -1dt "$gardes/$nom"-[0-9]* 2>/dev/null | tail -n +6 | while read -r vieux; do
        rm -rf -- "$vieux"
      done
    fi
    if [ "${RAG3WEAVER_GARDER:-}" != 1 ]; then
      rm -rf -- "${E2E_TMP_PARENT:?}/$nom" "${E2E_DISQUE_PARENT:?}/$nom"
    fi
  done
  unset RAG3WEAVER_E2E_DOSSIER RAG3WEAVER_CHECKPOINT_DIR RAG3WEAVER_CACHE_DIR

  # 4. Les autres tests nommés (sondes, rendus) : par cargo, comme avant.
  if [ ${#autres[@]} -gt 0 ]; then
    local args=(--features "$FEATURES" --no-fail-fast)
    for nom in "${autres[@]}"; do args+=(--test "$nom"); done
    args+=(-- --include-ignored --nocapture)
    [ -n "$TEST_FILTER" ] && args+=("$TEST_FILTER")
    args+=("${EXTRA_ARGS[@]}")
    confined cargo test "${args[@]}" || code=101
  fi
  return "$code"
}

# **Une suite non montée ne serait plus bâtie du tout** (autotests = false) :
# refuser de partir plutôt que de la sauter en silence.
# Seules les suites SUIVIES par git comptent (index compris : `git add`
# suffit) : un brouillon non suivi d'une autre session, dans un arbre partagé,
# n'est qu'annoncé — il ne bloque personne (11 octobre 2026).
NON_MONTEES=()
SUIVIES="$(git -C "$WEAVER" ls-files 'tests/e2e_*.rs' 2>/dev/null)"
for f in "$WEAVER"/tests/e2e_*.rs; do
  b="$(basename "$f")"
  grep -qF "#[path = \"../$b\"]" "$WEAVER/tests/e2e/main.rs" && continue
  if [ -z "$SUIVIES" ] || printf '%s\n' "$SUIVIES" | grep -qx "tests/$b"; then
    NON_MONTEES+=("$b")
  else
    echo "▸ suite non suivie et non montée, ignorée : $b (git add, puis python3 tests/e2e/generer.py)"
  fi
done
if [ ${#NON_MONTEES[@]} -gt 0 ]; then
  echo "✗ suites non montées dans tests/e2e/main.rs : ${NON_MONTEES[*]}"
  echo "  rejouer : python3 tests/e2e/generer.py"
  exit 2
fi

# **Les suites connues pour laisser des résidus** dans leur TMPDIR : la liste du
# ticket (les lignes # sont des commentaires). Une ligne `<suite>` seule : elle en laisse
# à chaque passe, et propre, elle est rouge. Une ligne `<suite> <chemin du ticket>` : un
# rouge connu du témoin, un résidu qui va et vient avec une cause en ticket ; il se dit en
# toutes lettres, propre reste propre.
LISTE_DES_RESIDUS="$WEAVER/tests/e2e/suites_qui_laissent_des_residus.txt"
residus_de_la_suite() {
  local nom="$1" dossier="$E2E_TMP_PARENT/$1" connue=0 ticket=""
  local entree
  entree=$(grep -v '^#' "$LISTE_DES_RESIDUS" 2>/dev/null | sed 's/[[:space:]]*$//' | awk -v s="$nom" '$1 == s { print; exit }')
  if [ -n "$entree" ]; then
    connue=1
    ticket=$(echo "$entree" | awk '{ $1 = ""; sub(/^ +/, ""); print }')
  fi
  if [ -n "$(find "$dossier" -mindepth 1 -print -quit 2>/dev/null)" ]; then
    local n
    n=$(find "$dossier" -mindepth 1 -maxdepth 1 | wc -l)
    echo "▸ RÉSIDUS $nom : $n entrée(s) dans son TMPDIR ($(du -sh "$dossier" 2>/dev/null | cut -f1))"
    find "$dossier" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort | head -20 | while read -r e; do
      echo "    $(du -sh -- "$dossier/$e" 2>/dev/null | cut -f1)	$e"
      # Ce qu'un dossier laissé contient : des fichiers vides (un `.readers`) ne pèsent rien.
      [ -d "$dossier/$e" ] && find "$dossier/$e" -mindepth 1 -maxdepth 3 -printf '      %s o\t%P\n' 2>/dev/null | head -5
    done
    if [ "$connue" = 1 ] && [ -n "$ticket" ]; then
      echo "  résidu connu : $nom, $ticket, $(find "$dossier" -mindepth 1 -maxdepth 3 -printf '%P ' 2>/dev/null | cut -c1-300)"
      return 0
    fi
    if [ "$connue" = 1 ]; then
      echo "  (connue : $nom est dans tests/e2e/suites_qui_laissent_des_residus.txt)"
      return 0
    fi
    echo "✗ $nom laisse des résidus dans son TMPDIR et n'est pas dans tests/e2e/suites_qui_laissent_des_residus.txt : effacez-les derrière vous"
    return 1
  fi
  if [ "$connue" = 1 ] && [ -z "$ticket" ] && [ -z "$TEST_FILTER" ]; then
    echo "✗ $nom finit propre mais reste dans tests/e2e/suites_qui_laissent_des_residus.txt : retirez-la de la liste (et du ticket)"
    return 1
  fi
  return 0
}

# **Le parent des dossiers de la passe**, et les restes des passes mortes :
# Un PID de passe disparu ne suffit pas : ses descendants portent encore sa
# marque. Conserver tant qu'ils vivent (ou si doute), archiver après quiescence.
E2E_DISQUE_RACINE="${HOME}/.cache/rag3weaver-build"
E2E_TMP_RACINE="${TMPDIR:-/tmp}"
E2E_TMP_PARENT="$E2E_TMP_RACINE/rag3weaver-e2e-$$"
for d in "$E2E_TMP_RACINE"/rag3weaver-e2e-*; do
  [ -d "$d" ] || continue
  if parent_de_passe_quiescent "$d"; then
    garder_parent_termine "$d"
  fi
done
mkdir -p "$E2E_TMP_PARENT"

# **Et un parent sur le vrai disque**, pour les suites qui ne veulent pas la
# RAM de /tmp (arrêts brutaux, transactions, plein texte en fichiers) et pour
# les points de reprise des catalogues de test : `RAG3WEAVER_E2E_DOSSIER`
# (lu par `common::dossier_de_suite`) et `RAG3WEAVER_CHECKPOINT_DIR` pointent
# dans `<parent>/<suite>/`, effacé derrière la suite comme son TMPDIR. Avant :
# des bases gardées à chaque passe (arret-brutal, +4 par passe), et 955 Mo de
# points de reprise de tests dans le vrai cache (11 octobre 2026).
E2E_DISQUE_PARENT="$E2E_DISQUE_RACINE/e2e-$$"
for d in "$E2E_DISQUE_RACINE"/e2e-[0-9]*; do
  [ -d "$d" ] || continue
  if parent_de_passe_quiescent "$d"; then
    garder_parent_termine "$d"
  fi
done
mkdir -p "$E2E_DISQUE_PARENT"
preparer_journaux_des_demons

# **Le target a un plafond.** Au-delà, la passe refuse de partir en le
# disant : un target qui grossit sans fin est ce qui a rempli le disque le
# 10 octobre (307 Go). Un bâti propre pèse 5,5 Go (mesuré le 11 octobre,
# tests/e2e/poids-reference.txt). **32 Go depuis le 7 novembre 2026** : une
# passe complète sous le jeu large (check_features et ses cinq jeux, la lib
# entière, l'étape navigateur, les e2e) pèse 19 à 26 Go à luciepc, sans
# accumulation — une rlib de rag3weaver (~0,5 Go) et un binaire de tests
# (~0,43 Go) par jeu de features, ~8,5 Go d'exécutables dans deps. Les 20 Go
# du 11 octobre dataient d'avant anthropic-llm, burn, postgres et navigateur :
# ils ont arrêté trois passes juste avant leurs e2e ce soir-là (23, 21 et
# 26 Go), rejouées de zéro. `RAG3WEAVER_TARGET_MAX_GO` le change.
TARGET_DE_LA_PASSE="${CARGO_TARGET_DIR:-$WEAVER/target}"
PLAFOND_TARGET_GO="${RAG3WEAVER_TARGET_MAX_GO:-32}"
if [ -d "$TARGET_DE_LA_PASSE" ]; then
  POIDS_TARGET_GO=$(du -xs --block-size=1G "$TARGET_DE_LA_PASSE" 2>/dev/null | cut -f1)
  if [ "${POIDS_TARGET_GO:-0}" -gt "$PLAFOND_TARGET_GO" ]; then
    echo "✗ le target $TARGET_DE_LA_PASSE pèse ${POIDS_TARGET_GO} Go, au-delà du plafond de ${PLAFOND_TARGET_GO} Go"
    echo "  (RAG3WEAVER_TARGET_MAX_GO ; une passe complète sous le jeu large pèse 19 à 26 Go). Pour repartir d'un target propre (5,5 Go au rebâti) :"
    echo "    rm -rf $TARGET_DE_LA_PASSE"
    exit 2
  fi
  echo "▸ target : ${POIDS_TARGET_GO} Go (plafond ${PLAFOND_TARGET_GO} Go)"
fi
# Un target ne sert qu'un arbre à la fois (scripts/garde_target.sh).
. "$WEAVER/scripts/garde_target.sh"
garder_le_target "$TARGET_DE_LA_PASSE" "$WEAVER" || exit 2

echo "▸ Running: suites ${CARGO_ARGS[*]}"

export PATH="/usr/local/cuda/bin:$PATH"
export LD_LIBRARY_PATH="$LIBDIR:/usr/local/cuda/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export CUDA_ROOT="/usr/local/cuda"

# ── Le régime, et pourquoi c'est `confort` par défaut ───────────────────────
#
# Une passe complète charge BGE-M3, MiniLM, deux rerankers et l'OCR. Rien dans
# ce script ne disait sur quelle carte : ils prenaient donc celle du système,
# c'est-à-dire la carte principale de Lucie, pendant qu'elle s'en sert. Le
# 4 septembre 2026 : « ça fait encore travailler mon gpu principal ».
#
# `confort` existe pour ça et n'avait jamais été branché ici. Il pose les trois
# rôles burn sur la carte la moins chargée, le rapport cyclique à 60 % et la
# rafale à 2 048 — plus lent, et le poste reste utilisable. C'est le bon défaut
# pour une suite qui tourne en fond ; `RAG3WEAVER_REGIME=plein` reprend la main
# quand on veut la vitesse et qu'on n'utilise pas la machine.
export RAG3WEAVER_REGIME="${RAG3WEAVER_REGIME:-confort}"
echo "▸ régime = $RAG3WEAVER_REGIME (RAG3WEAVER_REGIME=plein pour la vitesse)"

export RAG3DB_SHARED=1
export RAG3DB_LIBRARY_DIR="$LIBDIR"
export RAG3DB_INCLUDE_DIR="$LIBDIR"
# Depuis un worktree, `RAG3DB_ROOT` peut viser le dépôt principal, où
# l'extension vecteur est construite (extension/vector/build/) : posé, on le
# garde ; sinon la racine du script.
export RAG3DB_ROOT="${RAG3DB_ROOT:-$ROOT}"

# **Toute passe laisse son journal**, résumé ou non.
#
# Il vivait dans un `mktemp` effacé à la sortie, et seulement dans la branche
# `--summary` : une suite qui cassait ne laissait que des compteurs, et savoir
# *quel* test avait cassé demandait de tout relancer — une demi-heure pour
# retrouver une ligne qu'on avait déjà eue sous les yeux.
#
# Écrire d'abord, filtrer ensuite. Un appelant qui réduit la sortie à trois
# lignes (`| tail | grep`) ne détruit plus rien : le texte entier est ici.
E2E_LOG="${RAG3WEAVER_E2E_LOG:-$WEAVER/target/e2e-last.log}"
mkdir -p "$(dirname "$E2E_LOG")"

# **L'incrémental ne sert à rien ici, et il coûte cher.**
#
# Il paie sur le *deuxième* build de la *même* cible : on change une ligne, on
# rebuild, il réutilise. Une passe construit 34 binaires de test, chacun une
# fois — elle **écrit** le cache et ne le relit pratiquement jamais. C'est
# pourquoi `CARGO_INCREMENTAL=0` est le réglage de toute CI, et une passe E2E
# est de la CI. La boucle d'édition, elle, le garde : c'est là qu'il rapporte.
#
# Mesuré le 27 août 2026 : 124 Go d'incrémental accumulés en trois jours.
export CARGO_INCREMENTAL=0

# **Une pile à la première occasion, pas à la relance.**
#
# Sans ça, un panic obscur ne donne que son fichier:ligne — celui de la macro
# `panic!`, pas le chemin qui y a mené — et il faut relancer la passe pour
# apprendre ce qu'on aurait pu lire du premier coup. Une demi-heure pour une
# information qui était disponible gratuitement.
#
# `full` ne s'impose pas : la pile courte suffit et reste lisible.
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

# **Le ménage passe avant, pas « de temps en temps ».**
#
# Cargo suffixe chaque artefact d'une empreinte et ne supprime jamais les
# anciennes : à trois passes par jour, une centaine de gigas quotidiens. Ici,
# les périmés ne vivent jamais plus d'une passe. `RAG3WEAVER_NO_GC=1` l'évite.
if [ -z "${RAG3WEAVER_NO_GC:-}" ] && [ -x "$WEAVER/menage_target.py" ]; then
  "$WEAVER/menage_target.py" "$WEAVER/target" || true
fi

if [ "$SUMMARY" = true ]; then
  # Capture output, show summary at the end.
  #
  # **Le journal survit à la passe.** Il vivait dans un `mktemp` effacé à la
  # sortie : quand une suite échouait, il ne restait que des compteurs, et
  # savoir *quel* test avait cassé demandait de tout relancer — une demi-heure
  # pour retrouver une ligne qu'on avait déjà eue sous les yeux.
  #
  # Une passe qui échoue doit laisser de quoi regarder. C'est la même règle
  # que partout ici : rendre visible ce dont l'absence ne se voit pas.
  TMPLOG="$E2E_LOG"

  # **Le résumé va au journal, lui aussi.**
  #
  # `tee` ne capture que la sortie de cargo ; le bloc SUMMARY est écrit après,
  # directement à l'écran. Le journal s'arrêtait donc pile avant la partie qui
  # dit si la passe est verte — l'endroit exact où on regarde en premier.
  #
  # Deux `printf` plutôt qu'un pipe : pas de sous-shell, donc les compteurs
  # restent ceux du script, et pas de course à la fermeture du tube.
  say() { printf "$@"; printf "$@" >> "$E2E_LOG"; }
  set +e
  jouer_les_suites 2>&1 | tee "$TMPLOG"
  EXIT_CODE=${PIPESTATUS[0]}
  set -e

  say "\n═══════════════════════════════════════════════\n"
  say "  SUMMARY\n"
  say "═══════════════════════════════════════════════\n"

  TOTAL_PASSED=0
  TOTAL_FAILED=0
  # Suites qui ont rendu un résultat sans jouer un seul test.
  VIDES=0

  # **Une ligne de résultat par suite : la dernière avant la suivante.** Les
  # suites qui relancent un fils par `current_exe()` font imprimer au fils sa
  # propre ligne « test result » dans le même journal ; apparier par le rang
  # décalait alors tout le tableau (des « ? », des « AUCUN TEST » à contresens)
  # — justement quand une suite échoue et qu'on lit le résumé pour savoir
  # laquelle (contre-épreuve du 11 octobre 2026). La ligne du parent vient
  # toujours après celles de ses fils.
  mapfile -t SUITE_NAMES < <(grep -oP '(?<=Running tests/)\w+' "$TMPLOG")
  mapfile -t RESULTS < <(awk '
    /Running tests\// { if (n++) print (last == "" ? "test result: ok. 0 passed; 0 failed;" : last); last = ""; next }
    /^test result:/ { last = $0 }
    END { if (n) print (last == "" ? "test result: ok. 0 passed; 0 failed;" : last) }
  ' "$TMPLOG")

  for i in "${!RESULTS[@]}"; do
    line="${RESULTS[$i]}"
    suite="${SUITE_NAMES[$i]:-?}"
    passed=$(echo "$line" | grep -oP '\d+ passed' | grep -oP '\d+')
    failed=$(echo "$line" | grep -oP '\d+ failed' | grep -oP '\d+')
    TOTAL_PASSED=$((TOTAL_PASSED + ${passed:-0}))
    TOTAL_FAILED=$((TOTAL_FAILED + ${failed:-0}))

    if [ "${failed:-0}" -ne 0 ]; then
      say "  %-30s %3d passed, %d FAILED\n" "$suite" "$passed" "$failed"
    elif [ "${passed:-0}" -eq 0 ] && [ -n "$TEST_FILTER" ]; then
      # Sous un filtre, une suite sans test correspondant n'est pas une suite
      # compilée hors du lot : elle n'a simplement rien que le filtre désigne.
      say "  %-30s aucun test ne correspond au filtre « %s »\n" "$suite" "$TEST_FILTER"
    elif [ "${passed:-0}" -eq 0 ]; then
      # **Zéro test n'est pas une réussite.** Une suite entièrement compilée
      # hors du lot — un `#![cfg(feature = ...)]` que le lot de features
      # n'active pas — rend « ok. 0 passed », ce qui s'aligne dans ce tableau
      # exactement comme un succès. C'est le même défaut que les vingt-deux
      # tests sans `#[ignore]` du 5 septembre : une suite verte qui ne joue
      # rien. Elle se nomme maintenant.
      VIDES=$((VIDES + 1))
      say "  %-30s AUCUN TEST — compilée hors du lot ? (feature manquante)\n" "$suite"
    else
      say "  %-30s %3d passed\n" "$suite" "$passed"
    fi
  done

  if [ "$VIDES" -gt 0 ]; then
    say "\n  ⚠ %d suite(s) n'ont joué aucun test. Vérifier leur attribut cfg de module :\n" "$VIDES"
    say "    une suite compilée hors du lot ne prouve rien, et le total non plus.\n"
  fi

  # **Un saut nommé n'est pas un vert** (18 octobre 2026) : un test dont le
  # fournisseur n'a pas répondu (quota épuisé, panne : `LlmError::Unavailable`)
  # imprime `SAUTÉ <suite> — <raison>` et rend la main ; libtest le compte
  # réussi. Il est retiré des verts, nommé, et la passe se dit incomplète.
  mapfile -t SAUTES < <(grep -oP '^SAUTÉ \K.*' "$TMPLOG")
  if [ ${#SAUTES[@]} -gt 0 ]; then
    TOTAL_PASSED=$((TOTAL_PASSED - ${#SAUTES[@]}))
    say "\n  sautés (le fournisseur n'a pas répondu ; un saut n'est pas un vert) :\n"
    for t in "${SAUTES[@]}"; do
      say "    · %s\n" "$t"
    done
  fi

  # **Nommer ce qui a cassé.** Un compteur dit qu'il y a un problème ; il ne
  # dit pas lequel, et c'est justement ce qu'on vient chercher.
  mapfile -t FAILED_NAMES < <(grep -oP '^test \K[\w:]+(?= \.\.\. FAILED)' "$TMPLOG" | sort -u)
  if [ ${#FAILED_NAMES[@]} -gt 0 ]; then
    say "\n  échecs :\n"
    for t in "${FAILED_NAMES[@]}"; do
      say "    · %s\n" "$t"
    done
  fi

  # Les suites demandées qui n'ont pas rendu de résultat (compilation en
  # échec, abandon) : le total n'est pas un total.
  NOT_RUN=0
  for ((i = 0; i < ${#SUITES_DEMANDEES[@]}; i++)); do
    expected="${SUITES_DEMANDEES[$i]}"; binaire="${SUITES_BINAIRE[$i]}"
    found=false
    for s in "${SUITE_NAMES[@]}"; do
      { [ "$s" = "$expected" ] || { [ -n "$binaire" ] && [ "$s" = "$binaire" ]; }; } && found=true && break
    done
    if [ "$found" = false ]; then
      say "  %-30s NOT RUN\n" "$expected"
      NOT_RUN=$((NOT_RUN + 1))
    fi
  done

  # **Une suite écartée se rappelle au résumé.** Sans cette ligne, un total
  # vert dirait « tout passe » alors qu'un backend entier n'a pas été touché —
  # un saut qui se déguise en succès est exactement ce qu'on répare partout
  # ailleurs.
  if [ "$PG_DANS_LA_PASSE" = false ]; then
    say "  %-30s ÉCARTÉE — %s\n" "e2e_postgres" "$PG_RAISON"
  fi
  for nom in "${ECARTEES_CARTE_LOCALE[@]}"; do
    say "  %-30s ÉCARTÉE — carte locale\n" "$nom"
  done
  for nom in "${ECARTEES_BANCS[@]}"; do
    say "  %-30s ÉCARTÉE — banc à services (%s)\n" "$nom" "$(services_du_banc "$nom")"
  done
  for nom in "${ECARTEES_MODELE[@]}"; do
    say "  %-30s ÉCARTÉE — appelle un vrai modèle\n" "$nom"
  done

  say "───────────────────────────────────────────────\n"
  if [ "$TOTAL_FAILED" -eq 0 ]; then
    say "  %-30s %3d passed\n" "TOTAL" "$TOTAL_PASSED"
  else
    say "  %-30s %3d passed, %d FAILED\n" "TOTAL" "$TOTAL_PASSED" "$TOTAL_FAILED"
  fi
  if [ ${#SAUTES[@]} -gt 0 ]; then
    say "  %-30s INCOMPLÈTE — %d test(s) SAUTÉ(S), le fournisseur n'a pas répondu\n" "" "${#SAUTES[@]}"
    [ "$EXIT_CODE" -eq 0 ] && EXIT_CODE=1
  fi
  if [ ${#ECARTEES_CARTE_LOCALE[@]} -gt 0 ]; then
    say "  %-30s complète hors carte locale (%d suites écartées)\n" "" "${#ECARTEES_CARTE_LOCALE[@]}"
  fi
  if [ ${#ECARTEES_BANCS[@]} -gt 0 ]; then
    say "  %-30s complète hors bancs à services (%d suite(s) écartée(s))\n" "" "${#ECARTEES_BANCS[@]}"
  fi
  if [ "$NOT_RUN" -gt 0 ]; then
    say "  %-30s INCOMPLETE — %d suite(s) not run\n" "" "$NOT_RUN"
    [ "$EXIT_CODE" -eq 0 ] && EXIT_CODE=1
  fi
  if ! moteur_inchange > /dev/null; then
    say "  %s\n" "$(moteur_inchange)"
    EXIT_CODE=1
  fi
  if ! suites_vides "$TMPLOG" > /dev/null; then
    say "  %s\n" "$(suites_vides "$TMPLOG")"
    [ "$EXIT_CODE" -eq 0 ] && EXIT_CODE=1
  fi
  say "═══════════════════════════════════════════════\n"

  # **Ce que la passe a coûté à la machine.** Un total de tests verts ne dit
  # pas si le poste était inutilisable pendant une demi-heure.
  if [ -n "$CHARGE_PID" ]; then
    kill "$CHARGE_PID" 2>/dev/null || true
    CHARGE_PID=""
    echo ""
    echo "  CHARGE"
    "$WEAVER/charge.py" --resume "$CHARGE_LOG" || true
    echo "  journal de charge : $CHARGE_LOG"
  fi
  echo "  journal complet : $E2E_LOG"
  exit "$EXIT_CODE"
else
  set +e
  jouer_les_suites 2>&1 | tee "$E2E_LOG"
  EXIT_CODE=${PIPESTATUS[0]}
  set -e
  echo ""
  if ! suites_vides "$E2E_LOG"; then
    [ "$EXIT_CODE" -eq 0 ] && EXIT_CODE=1
  fi
  # **Le compte des suites** : jouées contre demandées. Une suite qui n'a pas
  # rendu de résultat (compilation, abandon) fait un total qui n'en est pas un.
  # Chaque suite demandée (SUITES_DEMANDEES) doit avoir sa ligne `Running tests/<nom>.rs`,
  # ou celle de son binaire (`tests/e2e/main.rs`) ; les filets du
  # binaire (`e2e_filets`) ne se comptent pas.
  compter_les_suites "$E2E_LOG"
  if [ ${#MANQUANTES[@]} -gt 0 ]; then
    echo "✗ ${#MANQUANTES[@]} suite(s) demandée(s) n'ont rendu aucun résultat : ${MANQUANTES[*]}"
    [ "$EXIT_CODE" -eq 0 ] && EXIT_CODE=1
  fi
  if [ ${#ECARTEES_MODELE[@]} -gt 0 ]; then
    echo "Passe complète hors suites à modèle : ${ECARTEES_MODELE[*]} écartée(s)."
  fi
  if ! moteur_inchange; then
    EXIT_CODE=1
  fi
  if [ ${#ECARTEES_CARTE_LOCALE[@]} -gt 0 ]; then
    echo "Passe complète hors carte locale : ${ECARTEES_CARTE_LOCALE[*]} écartées."
  fi
  if [ ${#ECARTEES_BANCS[@]} -gt 0 ]; then
    echo "Passe complète hors bancs à services : ${ECARTEES_BANCS[*]} écartée(s)."
  fi
  if [ -n "$CHARGE_PID" ]; then
    kill "$CHARGE_PID" 2>/dev/null || true
    CHARGE_PID=""
    echo "CHARGE"
    "$WEAVER/charge.py" --resume "$CHARGE_LOG" || true
    echo "Journal de charge : $CHARGE_LOG"
  fi
  echo "Journal complet : $E2E_LOG"
  echo "Tip: run with --summary for a per-suite results table."
  exit "$EXIT_CODE"
fi
