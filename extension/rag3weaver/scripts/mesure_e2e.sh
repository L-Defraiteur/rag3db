#!/bin/bash
# Mesure du poids et du temps de bâti des e2e — avant/après la refonte en un
# binaire. Usage : mesure_e2e.sh <étiquette> <worktree>
# Reproduit le bâti de run_e2e.sh (features, moteur partagé, debug) sur un
# target VIDE, puis un rebâti après retouche d'un fichier de src/, puis une
# batterie courte fixe. Tout est écrit dans ~/.cache/rag3weaver-build/mesure-e2e/<étiquette>/.
set -u
ETIQ="$1"; WT="$2"
CRATE="$WT/extension/rag3weaver"
L=~/.cache/rag3weaver-build/mesure-e2e/$ETIQ
mkdir -p "$L"; R="$L/resultats.txt"; : > "$R"
BUILD="${RAG3DB_BUILD:?RAG3DB_BUILD : le dossier build/lecteurs-csv du moteur partagé}"
export RAG3DB_BUILD=$BUILD RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=$BUILD/src RAG3DB_INCLUDE_DIR=$BUILD/src
export LD_LIBRARY_PATH=$BUILD/src RAG3DB_ROOT="${RAG3DB_ROOT:?RAG3DB_ROOT : l arbre dont l extension vector est bâtie}"
export CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-mesure-$ETIQ
export CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=8
FEATURES=rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres
cd "$CRATE" || exit 1
rm -rf "$CARGO_TARGET_DIR"

dit() { echo "$*" | tee -a "$R"; }
poids() {
    local T=$CARGO_TARGET_DIR/debug
    dit "  target total : $(du -xsh "$CARGO_TARGET_DIR" | cut -f1)"
    dit "  deps/ : $(du -xsh $T/deps | cut -f1) ; build/ : $(du -xsh $T/build 2>/dev/null | cut -f1)"
    local n=0 tot=0
    for b in $T/deps/e2e*-*; do
        [ -f "$b" ] && [ -x "$b" ] && [[ "$b" != *.d ]] || continue
        n=$((n+1)); tot=$((tot + $(stat -c %s "$b")))
    done
    dit "  binaires e2e exécutables : $n, $((tot/1024/1024)) Mo au total, $(( n > 0 ? tot/n/1024/1024 : 0 )) Mo en moyenne"
    local plus=$(ls -S $T/deps/e2e*-* 2>/dev/null | grep -v '\.d$' | head -1)
    [ -n "$plus" ] && dit "  le plus gros : $(basename $plus) $(( $(stat -c %s $plus)/1024/1024 )) Mo"
    dit "  moteur partagé (une fois, hors target) : librag3db.so $(( $(stat -c %s $BUILD/src/librag3db.so)/1024/1024 )) Mo"
}

# Les cibles : ce que run_e2e.sh bâtit (un --test par e2e_*.rs) — avant ;
# après la refonte, la cible unique « e2e ».
if [ -f tests/e2e/main.rs ]; then
    CIBLES="--test e2e"
else
    CIBLES=$(ls tests/e2e_*.rs | xargs -n1 basename | sed 's/\.rs$//' | sed 's/^/--test /' | tr '\n' ' ')
fi
dit "== $ETIQ — $(git -C "$WT" log --oneline -1) — $(date '+%d/%m %H:%M')"
dit "cibles : $(echo $CIBLES | wc -w | awk '{print $1/2}') binaire(s) de test demandés"

dit "1. bâti à froid (target vide)"
t0=$(date +%s)
cargo test --no-run --features $FEATURES $CIBLES > "$L/bati_froid.log" 2>&1
dit "  code=$? durée=$(( $(date +%s)-t0 )) s"
poids

dit "2. rebâti après retouche de src/catalog.rs (la boucle de travail)"
touch src/catalog.rs
t0=$(date +%s)
cargo test --no-run --features $FEATURES $CIBLES > "$L/bati_retouche.log" 2>&1
dit "  code=$? durée=$(( $(date +%s)-t0 )) s"
dit "  éditions de liens : $(grep -c 'Finished\|Executable' "$L/bati_retouche.log") lignes d'exécutables"

dit "3. batterie courte fixe (par run_e2e.sh)"
export RAG3WEAVER_SANS_CARTE_LOCALE=1
: "${RAG3WEAVER_EMBED_SERVICE:?RAG3WEAVER_EMBED_SERVICE : les adresses du service d embarquement}"
export RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70 RAG3WEAVER_NO_GC=1
export RAG3WEAVER_VERROU_TENU="exclusif (mesure du lot e2e, tenu par poste mesure)"
t0=$(date +%s)
./run_e2e.sh --test e2e_checkpoint --test e2e_dataflow_observe --test e2e_scope \
    --test e2e_liens --test e2e_entites_derivees --test e2e_catalogue_gabarits \
    > "$L/batterie.log" 2>&1
dit "  code=$? durée=$(( $(date +%s)-t0 )) s"
grep -E "^test result" "$L/batterie.log" | awk '{p+=$4; f+=$6} END {print "  tests : " p " passés, " f " échoués"}' | tee -a "$R"
dit "  target après batterie : $(du -xsh "$CARGO_TARGET_DIR" | cut -f1)"
dit "== fin $(date '+%H:%M')"
rm -rf "$CARGO_TARGET_DIR"
