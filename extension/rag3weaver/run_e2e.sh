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
#   ./run_e2e.sh --features openai-llm    # add features to the default set

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
# On se relance sous `flock`, avant tout effet de bord (journal de charge,
# compilation), pour que le verrou couvre la passe entière.
if [ -z "${RAG3WEAVER_VERROU_TENU:-}" ] && command -v flock >/dev/null; then
  VERROU="${HOME}/.cache/rag3weaver-build/poste.lock"
  PORTE="${HOME}/.cache/rag3weaver-build/porte.lock"
  mkdir -p "$(dirname "$VERROU")"
  MODE_VERROU="-s"; NOM_VERROU="partagé, priorité basse"
  for a in "$@"; do
    case "$a" in --build|--build-only) MODE_VERROU="-x"; NOM_VERROU="exclusif (rebâti du moteur), priorité basse" ;; esac
  done
  if [ "${RAG3WEAVER_MESURE:-}" = 1 ]; then MODE_VERROU="-x"; NOM_VERROU="exclusif (mesure)"; fi
  BASSE=""
  [ "${RAG3WEAVER_MESURE:-}" = 1 ] || BASSE="nice -n 15 ionice -c 3"
  if ! flock -n "$MODE_VERROU" "$VERROU" true || ! flock -n -x "$PORTE" true; then
    echo "▸ verrou du poste $NOM_VERROU : attente d'une passe en cours ($VERROU)"
  fi
  export RAG3WEAVER_VERROU_TENU="$NOM_VERROU"
  # **Par le script du poste quand il est là** (5 octobre 2026, après la
  # panne de mémoire du 4 à 22 h 51) : il prend la même porte et le même
  # verrou, et lance la passe dans sa propre portée systemd plafonnée
  # (POSTE_MEM_MAX, 40 Go par défaut ; POSTE_SWAP_MAX, 4 Go). Une passe qui
  # déborde est tuée seule (code 137) au lieu d'emporter les sessions. Sans
  # le script, le verrou d'avant, sans plafond.
  POSTE="${HOME}/.cache/rag3weaver-build/poste"
  if [ -x "$POSTE" ]; then
    if [ "$MODE_VERROU" = "-x" ] && [ "${RAG3WEAVER_MESURE:-}" = 1 ]; then
      exec "$POSTE" mesure "$0" "$@"
    elif [ "$MODE_VERROU" = "-x" ]; then
      # Un rebâti du moteur : exclusif, en priorité basse.
      exec "$POSTE" mesure nice -n 15 ionice -c 3 "$0" "$@"
    fi
    exec "$POSTE" lourd "$0" "$@"
  fi
  if [ "$MODE_VERROU" = "-x" ]; then
    exec flock -x "$PORTE" flock -x "$VERROU" $BASSE "$0" "$@"
  fi
  flock -x "$PORTE" true
  exec flock -s "$VERROU" $BASSE "$0" "$@"
fi
[ -n "${RAG3WEAVER_VERROU_TENU:-}" ] && echo "▸ verrou du poste : $RAG3WEAVER_VERROU_TENU"

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

# ── Confiner la pression mémoire ────────────────────────────────────────────
#
# Le 27 août 2026 : le poste ramait, et ce n'était pas le CPU. Un gros build
# fait défiler des gigaoctets à travers le cache de fichiers ; ça déclenche de
# la récupération mémoire en continu, et avec `vm.swappiness=150` (défaut
# CachyOS, pensé pour zram) ce qui part en zram, c'est **le bureau** — Chrome,
# l'éditeur. On revient dessus, il faut décompresser : « ça galère », CPU à 3 %.
#
# La cure serait un `swapoff/swapon` après coup, qui demande root. La
# prévention ne le demande pas : on met le build dans son propre cgroup, avec
# une limite haute. Sous la limite, rien ne change ; au-dessus, c'est **son**
# cache à lui qui est récupéré, pas les pages des applications ouvertes.
#
# `RAG3WEAVER_BUILD_MEMORY_HIGH=0` désactive, une taille (`24G`) la change.
confined() {
  local high="${RAG3WEAVER_BUILD_MEMORY_HIGH:-16G}"
  if [ "$high" = "0" ] || ! command -v systemd-run >/dev/null 2>&1 \
     || [ ! -e /sys/fs/cgroup/user.slice/user-"$(id -u)".slice/cgroup.controllers ]; then
    "$@"
    return $?
  fi
  systemd-run --user --scope --quiet --collect -p MemoryHigh="$high" -- "$@"
}

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
arreter_les_demons_de_la_passe() {
  local p
  for p in $(pidof rag3weaver-embeddings || true); do
    if tr '\0' '\n' < "/proc/$p/environ" 2>/dev/null | grep -qx "RAG3WEAVER_PASSE_E2E=$RAG3WEAVER_PASSE_E2E"; then
      echo "▸ démon d'embarquement né pendant la passe arrêté (pid $p, SIGTERM)"
      kill -TERM "$p" 2>/dev/null || true
    fi
  done
}
a_la_sortie() {
  [ -n "$CHARGE_PID" ] && kill "$CHARGE_PID" 2>/dev/null || true
  arreter_les_demons_de_la_passe
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
elif [ ! -f "$BUILD/src/librag3db.so" ]; then
  echo "▸ No existing build found, building..."
  NEED_BUILD=true
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
FEATURES="rag3db-native,burn-embedder,burn-ocr,code,daemon${EXTRA_FEATURES:+,$EXTRA_FEATURES}"

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
PG_HOTE=$(printf '%s' "$PG_URL" | sed -E 's|.*@([^:/]+).*|\1|')
PG_PORT=$(printf '%s' "$PG_URL" | sed -E 's|.*:([0-9]+)/.*|\1|')
# `/dev/tcp` de bash plutôt que `nc` : la première version employait `nc`, qui
# n'est **pas installé ici**. La sonde ne pouvait donc jamais répondre oui, et
# aurait écarté postgres pour toujours en croyant l'avoir vérifié — le défaut
# même qu'elle est censée prévenir, dans l'outil qui le prévient.
if [ -z "${RAG3WEAVER_SANS_PG:-}" ]; then
  if timeout 2 bash -c "echo > /dev/tcp/$PG_HOTE/$PG_PORT" 2>/dev/null; then
    PG_DANS_LA_PASSE=true
    FEATURES="$FEATURES,postgres"
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
# Cinq fichiers portent `#![cfg(all(..., feature = "openai-llm", ...))]` : sans
# cette feature, ils sont **compilés hors du lot** et rendent « ok. 0 passed »,
# ce qui s'aligne dans le résumé exactement comme un succès. Le tableau final
# les nomme désormais ; on l'annonce aussi **au lancement**, comme PostgreSQL,
# parce que c'est là qu'on décide quoi lancer.
#
# L'exclusion est délibérée : ces suites dépensent le quota Vertex à chaque
# passe. `--features openai-llm` les fait entrer.
case ",$FEATURES," in
  *,openai-llm,*) ;;
  *)
    echo "▸ ⚠ 5 suites NON JOUÉES (feature openai-llm absente, exclusion délibérée —"
    echo "    elles appellent un vrai modèle) : e2e_avis_du_modele, e2e_cloud_code_agent,"
    echo "    e2e_cloud_schema_probe, e2e_conversation_a_plusieurs, e2e_lecture_mermaid"
    ;;
esac

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
    if [ "$SANS_CARTE_LOCALE" != 0 ] && est_de_la_carte_locale "$nom"; then
      ECARTEES_CARTE_LOCALE+=("$nom")
      continue
    fi
    CARGO_ARGS+=(--test "$nom")
  done
  if [ ${#ECARTEES_CARTE_LOCALE[@]} -gt 0 ]; then
    echo "▸ ⚠ ${#ECARTEES_CARTE_LOCALE[@]} suites NON JOUÉES (carte locale, RAG3WEAVER_SANS_CARTE_LOCALE=0 pour les jouer) :"
    echo "    ${ECARTEES_CARTE_LOCALE[*]}"
  fi
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

verifier_le_moteur() {
  local lib="$BUILD/src/librag3db.so" ext="$EXTENSION_VECTOR"
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
  for f in "$BUILD/src/librag3db.so" "$EXTENSION_VECTOR"; do
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

echo "▸ Running: cargo test ${CARGO_ARGS[*]}"

export PATH="/usr/local/cuda/bin:$PATH"
export LD_LIBRARY_PATH="$BUILD/src:/usr/local/cuda/lib64${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
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
export RAG3DB_LIBRARY_DIR="$BUILD/src"
export RAG3DB_INCLUDE_DIR="$BUILD/src"
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
  confined cargo test "${CARGO_ARGS[@]}" 2>&1 | tee "$TMPLOG"
  EXIT_CODE=${PIPESTATUS[0]}
  set -e

  say "\n═══════════════════════════════════════════════\n"
  say "  SUMMARY\n"
  say "═══════════════════════════════════════════════\n"

  TOTAL_PASSED=0
  TOTAL_FAILED=0
  # Suites qui ont rendu un résultat sans jouer un seul test.
  VIDES=0

  # Collect Running/result lines in order
  mapfile -t SUITE_NAMES < <(grep -oP '(?<=Running tests/)\w+' "$TMPLOG")
  mapfile -t RESULTS < <(grep '^test result:' "$TMPLOG")

  for i in "${!RESULTS[@]}"; do
    line="${RESULTS[$i]}"
    suite="${SUITE_NAMES[$i]:-?}"
    passed=$(echo "$line" | grep -oP '\d+ passed' | grep -oP '\d+')
    failed=$(echo "$line" | grep -oP '\d+ failed' | grep -oP '\d+')
    TOTAL_PASSED=$((TOTAL_PASSED + ${passed:-0}))
    TOTAL_FAILED=$((TOTAL_FAILED + ${failed:-0}))

    if [ "${failed:-0}" -ne 0 ]; then
      say "  %-30s %3d passed, %d FAILED\n" "$suite" "$passed" "$failed"
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
  for ((i = 0; i < ${#CARGO_ARGS[@]}; i++)); do
    if [ "${CARGO_ARGS[$i]}" = "--test" ]; then
      expected="${CARGO_ARGS[$((i + 1))]}"
      found=false
      for s in "${SUITE_NAMES[@]}"; do [ "$s" = "$expected" ] && found=true && break; done
      if [ "$found" = false ]; then
        say "  %-30s NOT RUN\n" "$expected"
        NOT_RUN=$((NOT_RUN + 1))
      fi
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

  say "───────────────────────────────────────────────\n"
  if [ "$TOTAL_FAILED" -eq 0 ]; then
    say "  %-30s %3d passed\n" "TOTAL" "$TOTAL_PASSED"
  else
    say "  %-30s %3d passed, %d FAILED\n" "TOTAL" "$TOTAL_PASSED" "$TOTAL_FAILED"
  fi
  if [ ${#ECARTEES_CARTE_LOCALE[@]} -gt 0 ]; then
    say "  %-30s complète hors carte locale (%d suites écartées)\n" "" "${#ECARTEES_CARTE_LOCALE[@]}"
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
  confined cargo test "${CARGO_ARGS[@]}" 2>&1 | tee "$E2E_LOG"
  EXIT_CODE=${PIPESTATUS[0]}
  set -e
  echo ""
  if ! suites_vides "$E2E_LOG"; then
    [ "$EXIT_CODE" -eq 0 ] && EXIT_CODE=1
  fi
  if ! moteur_inchange; then
    EXIT_CODE=1
  fi
  if [ ${#ECARTEES_CARTE_LOCALE[@]} -gt 0 ]; then
    echo "Passe complète hors carte locale : ${ECARTEES_CARTE_LOCALE[*]} écartées."
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
