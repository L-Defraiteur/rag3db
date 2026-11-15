#!/bin/bash
# Prépare la pose du binaire d'embarquement (rag3weaver-embeddings) à luciepc, sans rien poser (15 nov. 2026).
# Bâti de <hash de master> dans rag3db-controle-G, target à part (target-G/embarquement), features de la session
# embarquement : daemon,burn-embedder,burn-ocr. Copie dans nuit-<hash>/, puis témoin sur une copie d'essai au port
# 7992, granite-107m sur le processeur (aucune carte touchée) : /sante (max_chars_per_text 12000, chars_per_token 3),
# un /embed court, un /embed de 13 000 caractères qui rend 413 sans calcul : à la pose, en place (poser-embarquement.sh).
# Usage : preparer-embarquement.sh <hash de master> [ancien]   (« ancien » : la tête n'a pas encore service-503)
set -u
H=${1:?hash}; ANCIEN=${2:-}
ssh -o BatchMode=yes luciepc "bash -s" <<X
set -u
cd ~/.cache/rag3weaver-build
W=\$HOME/git_workspaces/rag3db-controle-G
git -C \$W fetch -q origin master || exit 2
git -C \$W merge-base --is-ancestor $H FETCH_HEAD || { echo "$H n'est pas dans origin/master : rien bâti"; exit 3; }
git -C \$W checkout -q --detach $H && git -C \$W submodule update -q extension/rag3weaver/codeparsers || exit 2
echo "== \$(date +%T) bâti de rag3weaver-embeddings sur \$(git -C \$W log --oneline -1 | cut -c1-12)"
cd \$W/extension/rag3weaver
JOBS=8 ~/.cache/rag3weaver-build/poste lourd env CARGO_TARGET_DIR=\$HOME/.cache/rag3weaver-build/target-G/embarquement CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=8 \
  bash scripts/cargo_magasin.sh build --release --bin rag3weaver-embeddings --features daemon,burn-embedder,burn-ocr > ~/.cache/rag3weaver-build/bati-embarquement-$H.log 2>&1 || { echo "bâti rouge"; tail -8 ~/.cache/rag3weaver-build/bati-embarquement-$H.log; exit 4; }
cd ~/.cache/rag3weaver-build
mkdir -p nuit-$H && cp -p target-G/embarquement/release/rag3weaver-embeddings nuit-$H/
B=\$PWD/nuit-$H/rag3weaver-embeddings
echo "binaire : \$(sha256sum \$B | cut -c1-16) ; en place : \$(sha256sum ~/.local/bin/rag3weaver-embeddings | cut -c1-16)"
# L'essai à côté est impossible (15 nov.) : sur CPU burn refuse un tampon de 384 Mo (Vulkan(Cpu)), card0 est à seat1,
# card2 aux services. La preuve du binaire neuf se lit dans le binaire ; la preuve de service se fait en place, à la pose.
R=0
for champ in max_chars_per_text chars_per_token; do
  n=\$(grep -c -a "\$champ" \$B)
  echo "champ \$champ dans le binaire : \$n"
  if [ "$ANCIEN" = ancien ]; then [ "\$n" = 0 ] || R=1; else [ "\$n" -gt 0 ] || R=1; fi
done
[ \$R = 0 ] && echo VERT || echo ROUGE
X
