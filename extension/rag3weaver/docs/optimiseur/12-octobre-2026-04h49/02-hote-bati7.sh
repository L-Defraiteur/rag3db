#!/bin/bash
# luciepc, sous `poste lourd` : la lib du moteur à la tête de master (≥ 3cca0ae1e,
# l'insertion HNSW du cœur, qui contient e6f626df4, le correctif du balayage filtré), publiée au magasin ; le binaire de l'hôte lié par
# RUNPATH à l'identifiant ; rien n'est installé ni relancé (l'hôte est à Lucie).
#
# Ce qu'il exige :
#   - le worktree ~/git_workspaces/rag3db-hote de luciepc à une tête qui contient
#     3cca0ae1e (le vérifie, refuse sinon, code 2) ; l'avancer est à Lucie ;
#   - le poste libre pour un lourd : se lance par `poste lourd bash hote-bati7.sh` ;
#   - build/ est créé ici (hote-bati6 était mort sur son absence, le 12 oct. à 6 h 40) ;
#   - cargo passe par scripts/cargo_magasin.sh avec RAG3DB_LIB=<identifiant publié>
#     (pas `courant`, qui peut bouger) ; le RUNPATH vers ce dossier reste posé ici.
#
# Ce qu'il produit :
#   - la lib et l'extension publiées au magasin de luciepc (~/.cache/rag3db-libs/<id>) ;
#   - ~/.local/bin/rag3weaver-backend.pret et rag3db-check-lib.pret, avec leurs md5,
#     vérifiés (symboles rag3db tous définis dans la lib, rag3db-check-lib à 0) ;
#     ils remplacent le .pret du 11 oct. à 22 h 13, l'ancien (d'avant B4) ;
#   - check-lib.conf.pret pour l'unité (inactif) ;
#   - la note ~/.local/share/rag3weaver/POUR-LUCIE-hote.md.
# Journaux : build/lib.configure.log, build/lib.bati.log, ~/.cache/rag3weaver-build/hote-cargo7.log.
set -u
W=$HOME/git_workspaces/rag3db-hote
B=$W/build/lib
echo "== $(date +%H:%M:%S) tête : $(git -C $W log --oneline -1 | cut -c1-60)"
git -C $W merge-base --is-ancestor 3cca0ae1e HEAD || { echo "la tête ne contient pas 3cca0ae1e"; exit 2; }
mkdir -p $W/build
cmake -S $W -B $B -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_EXTENSIONS=vector -DBUILD_SHELL=FALSE > $B.configure.log 2>&1 || { echo "configure rouge"; exit 3; }
cmake --build $B -j 12 > $B.bati.log 2>&1 || { echo "bâti de la lib rouge"; tail -20 $B.bati.log; exit 3; }
echo "== $(date +%H:%M:%S) lib bâtie : $(ls -la $B/src/librag3db.so | awk '{print $5}') octets"
python3 $W/extension/rag3weaver/scripts/rag3db_libs.py publier $B --racine $W || { echo "publication refusée"; exit 4; }
L=$(python3 $W/extension/rag3weaver/scripts/rag3db_libs.py courant) || exit 4
echo "== magasin : $L"
cd $W/extension/rag3weaver
# Le pont et la lib par l'enveloppeur du magasin (master e38cc586b), sur l'identifiant
# publié, pas sur `courant` qui peut bouger ; le RUNPATH reste à nous.
export RAG3DB_LIB=$(basename $L) RUSTFLAGS="-C link-args=-Wl,-rpath,$L" CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/target-G-hote CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=12
cargo clean --release -p rag3db -p rag3weaver
bash scripts/cargo_magasin.sh build --release --bin rag3weaver-backend --bin rag3db-check-lib --features rag3db-native,code,daemon,openai-llm > $HOME/.cache/rag3weaver-build/hote-cargo7.log 2>&1 || { echo "bâti cargo rouge"; tail -20 $HOME/.cache/rag3weaver-build/hote-cargo7.log; exit 5; }
X=$CARGO_TARGET_DIR/release/rag3weaver-backend; C=$CARGO_TARGET_DIR/release/rag3db-check-lib
readelf -d $X | grep -i runpath
manque=0; for s in $(nm -D --undefined-only $X | awk '{print $2}' | grep '^_ZN6rag3db'); do nm -D --defined-only $L/librag3db.so | grep -q " $s$" || { echo "MANQUE $s"; manque=1; }; done
echo "symboles rag3db manquants : $manque"; [ $manque = 0 ] || exit 6
$C $X $L/librag3db.so; rc=$?; echo "rag3db-check-lib : $rc"; [ $rc = 0 ] || exit 6
install -m 755 $X $HOME/.local/bin/rag3weaver-backend.pret && install -m 755 $C $HOME/.local/bin/rag3db-check-lib.pret
md5sum $HOME/.local/bin/rag3weaver-backend.pret $HOME/.local/bin/rag3db-check-lib.pret
mkdir -p $HOME/.config/systemd/user/rag3weaver-daemon.service.d
printf '[Service]\n# Le binaire et la lib de son RUNPATH sont du même bâti, sinon l unité ne démarre pas.\nExecStartPre=%%h/.local/bin/rag3db-check-lib %%h/.local/bin/rag3weaver-backend %s/librag3db.so\n' "$L" > $HOME/.config/systemd/user/rag3weaver-daemon.service.d/check-lib.conf.pret
cat $HOME/.config/systemd/user/rag3weaver-daemon.service.d/check-lib.conf.pret
echo "== $(date +%H:%M:%S) prêt"
N=$HOME/.local/share/rag3weaver/POUR-LUCIE-hote.md
{
echo "# L'hôte de luciepc : ce qui est prêt, et l'ordre pour le remplacer"
echo
echo "Préparé par la session optimiseur (G), bâti le $(date '+%d octobre %Y à %H h %M'), sur master $(git -C $W log --oneline -1 | cut -c1-9)."
echo "Rien n'est installé ni relancé : l'hôte figé est à toi (pile gdb, puis arrêt)."
echo "Les deux .pret ci-dessous viennent de ce bâti ; ils ont remplacé le .pret du 11 oct. à 22 h 13 (l'ancien, d'avant B4)."
echo
echo "## Ce qui est prêt"
echo
echo "- \`~/.local/bin/rag3weaver-backend.pret\` — md5 $(md5sum $HOME/.local/bin/rag3weaver-backend.pret | cut -c1-32)"
echo "- \`~/.local/bin/rag3db-check-lib.pret\` — md5 $(md5sum $HOME/.local/bin/rag3db-check-lib.pret | cut -c1-32)"
echo "- La lib du moteur, avec le correctif du balayage filtré (e6f626df4) et l'insertion HNSW (3cca0ae1e), au magasin :"
echo "  \`$L\` (le binaire y est lié par RUNPATH ; un rebâti ailleurs ne le touche pas)"
echo "- \`~/.config/systemd/user/rag3weaver-daemon.service.d/check-lib.conf.pret\` : l'ExecStartPre qui"
echo "  refuse de démarrer un binaire et une lib de bâtis différents (inactif tant qu'il porte .pret)"
echo
echo "## L'ordre"
echo
echo "1. La pile de l'hôte figé, puis son arrêt (\`systemctl --user stop rag3weaver-daemon\`)."
echo "2. Installer : \`mv ~/.local/bin/rag3weaver-backend.pret ~/.local/bin/rag3weaver-backend\` et"
echo "   \`mv ~/.local/bin/rag3db-check-lib.pret ~/.local/bin/rag3db-check-lib\`."
echo "3. Activer la vérification : \`mv ~/.config/systemd/user/rag3weaver-daemon.service.d/check-lib.conf.pret ~/.config/systemd/user/rag3weaver-daemon.service.d/check-lib.conf\`"
echo "   puis \`systemctl --user daemon-reload\`."
echo "4. \`systemctl --user start rag3weaver-daemon\`, puis \`/sante\` (même pid que daemon.json, build $(basename $L))."
echo "5. Les bases de code sont d'avant la règle des chemins relatifs (B4) : codeparsers les efface et les"
echo "   réindexe avec cet hôte. Jamais avant que la lib ne porte e6f626df4 : sinon la boucle revient."
} > $N
echo "== note : $N"
