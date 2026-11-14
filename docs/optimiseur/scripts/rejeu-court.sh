#!/bin/bash
# Rejeu court d'un lot de G qui touche scripts/poste (14 nov. 2026, la file en pile) : à luciepc, sous `poste lourd`,
# dans ~/git_workspaces/rag3db-controle-G détaché sur <branche>. Le témoin du poste, celui de la garde de push,
# check_features, puis la lib entière sous le jeu large. Verdict VERT ou ROUGE <étape> dans $OUT/verdict.
# Usage : rejeu-court.sh <branche distante, ex. tenant-signal-pile> <hash attendu>
set -u
B=${1:?branche}; H=${2:?hash}
R=$HOME/git_workspaces/rag3db-controle-G; OUT=$HOME/.cache/rag3weaver-build/rejeu-$B; mkdir -p "$OUT"; : > "$OUT/journal"
etape() { echo "▸ $(date +%H:%M:%S) $1" >> "$OUT/journal"; }
rouge() { echo "ROUGE $1" > "$OUT/verdict"; etape "ROUGE $1"; exit 1; }
vert_ou_rouge() { grep -qE 'test result: .* [1-9][0-9]* failed|^SAUTÉ |^error(\[|:)' "$1" && rouge "$2"; grep -q 'test result: ok' "$1" || rouge "$2-sans-resultat"; }
git -C "$R" fetch -q origin "$B" && git -C "$R" checkout -q --detach FETCH_HEAD && git -C "$R" submodule update -q || rouge checkout
[ "$(git -C "$R" rev-parse --short=9 HEAD)" = "${H:0:9}" ] || rouge "tete-$(git -C "$R" rev-parse --short=9 HEAD)-pas-$H"
etape "tête $(git -C "$R" log -1 --format='%h %s' | cut -c1-80)"
cd "$R/extension/rag3weaver" || rouge cd
export CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/target-G/passe
export RAG3DB_LIB=rag3db-build-0c41f9fe6731a203b330ce975c4e26e9089bf807-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF
unset RAG3WEAVER_MOTEUR_ANCIEN
export RAG3WEAVER_TARGET_MAX_GO=32 RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879 RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70
. scripts/lib_du_magasin.sh; exporter_la_lib || rouge lib-du-magasin
JEU=rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm
etape "test_poste.sh"
bash scripts/test_poste.sh "$PWD/scripts/poste" > "$OUT/test_poste.log" 2>&1 || rouge test_poste
grep -q '^ÉCHEC' "$OUT/test_poste.log" && rouge test_poste-echec
etape "test_push_garde.sh"
bash "$R/tools/poste/test_push_garde.sh" > "$OUT/test_push_garde.log" 2>&1 || rouge test_push_garde
etape "check_features"
CHECK_FEATURES_LOGS="$OUT/check" scripts/check_features.sh > "$OUT/check_features.log" 2>&1 || rouge check_features
etape "lib entière ($JEU)"
cargo test --lib --features "$JEU" > "$OUT/lib.log" 2>&1 || rouge lib
vert_ou_rouge "$OUT/lib.log" lib
echo VERT > "$OUT/verdict"; etape "VERT — $(grep 'test result' "$OUT/lib.log" | tail -1 | cut -c1-80)"
