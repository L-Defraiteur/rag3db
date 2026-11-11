#!/bin/bash
# Le repli décidé par c7 (8 nov. 2026, après le SIGSEGV de 8 h 50) : au second SIGSEGV de l'hôte du portable,
# reposer l'hôte d'avant la bascule — binaire .avant-dadbf004 (eb154bf5f) sur la lib 17a094d12 —, courant inchangé.
set -u
S=$(dirname "$0")
B=$HOME/.local/bin
N17=rag3db-build-17a094d12d8c29418fbf3b3ed941ad64e73b4c29-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF
[ -x $B/rag3weaver-backend.avant-dadbf004 ] && [ -x $B/rag3db-check-lib.avant-dadbf004 ] || { echo "pas de .avant-dadbf004 : rien posé"; echo ROUGE; exit 3; }
cp -p $B/rag3weaver-backend.avant-dadbf004 $B/rag3weaver-backend.nouveau && cp -p $B/rag3db-check-lib.avant-dadbf004 $B/rag3db-check-lib.nouveau || { echo ROUGE; exit 3; }
INSTALLEUR=$S/installer-l-unite-de-l-hote.sh bash $S/bascule-pose.sh $N17 avant-repli-17a | grep -E '^==|VERT|ROUGE'
bash $S/bascule-reveil.sh $N17 127.0.0.1:7979 127.0.0.1:7980 127.0.0.1:7981 | grep -E 'service|MainPID|LISTEN|jeton|VERT|ROUGE|INJOIGNABLE'
echo "courant : $(readlink $HOME/.cache/rag3db-libs/courant | grep -o 'rag3db-build-[0-9a-f]\{9\}')"
