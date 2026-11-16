#!/bin/bash
# Une passe épinglée sur une lib du magasin (session recherche, puis toutes les sessions). Entrée dans le dépôt le
# 14 novembre 2026 (c7) : la copie du cache, appelée par des dizaines de scripts, n'était dans aucun dépôt et différait
# d'un poste à l'autre (luciepc disait RAG3WEAVER_MOTEUR_ANCIEN au résumé, le portable non). Cette version part de celle
# de luciepc. Les postes l'appellent depuis `~/.cache/rag3weaver-build/passe_epinglee.sh`, recopiée d'ici.
# Usage : passe_epinglee.sh <worktree> <dossier des journaux> <filtre du témoin> [temoins|-] [témoin navigateur de la lib]
#   avec « temoins » en 4e argument : le témoin seul (le rouge), sinon la passe entière :
#   le témoin, e2e_lectures, e2e_code, check_features (et la preuve), la lib entière
#   sous le jeu le plus large. Dernière ligne du résumé : VERT, ou ROUGE et l'étape.
set -u
W=$HOME/git_workspaces/$1
# **Le dossier des journaux est un nom**, sous ~/.cache/rag3weaver-build/ (15 novembre 2026, ticket d'hotfixs) : un chemin
# absolu s'y emboîtait en silence, et le verdict s'écrivait là où ni sa session ni la garde du push (qui lit
# ~/.cache/rag3weaver-build/<journaux>) ne le lisaient. Il est refusé, nommé ; la passe dit ensuite où elle écrit.
case "$2" in
  /*) echo "ROUGE passe épinglée : « $2 » est un chemin absolu ; le dossier des journaux est un nom sous $HOME/.cache/rag3weaver-build/ (la garde du push l'y lit)" >&2; exit 2 ;;
esac
J=$HOME/.cache/rag3weaver-build/$2
echo "journaux : $J"
FILTRE=$3
SEUL=${4:-}
[ "$SEUL" = - ] && SEUL=
mkdir -p $J
cd $W/extension/rag3weaver || exit 1
export CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/${CIBLE:-target-97} CARGO_BUILD_JOBS=8 CARGO_INCREMENTAL=0
# Les services d'embarquement : locaux à luciepc, par les tunnels ailleurs ; le régime doux
# hors de luciepc (l'écran de Lucie).
if [ "$(hostname)" = LuciePC ]; then
  # 7880 (granite-107m) et 7981 (son tunnel) sont arrêtés depuis le 12 novembre 2026, au mot de Lucie : ni l'un ni
  # l'autre n'est cité (un service cité qui ne répond pas fait attendre le client).
  export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879
  # La base de test de luciepc (rag3weaver-pg), pour e2e_postgres : un lot qui touche un gabarit la joue.
  export RAG3WEAVER_PG=postgres://rag3weaver:rag3weaver@127.0.0.1:5433/rag3weaver_test
  # Le repli des tests sur place (lot repli-de-l-aide d'embarquement, 15 novembre 2026) : un modèle qu'aucun service
  # listé ne sert (granite-107m, depuis l'arrêt de 7880) se charge sur l'iGPU de luciepc, card1, l'i915 ARL, qui ne
  # porte aucun écran (ses quatre sorties sont « disconnected », relevé d'embarquement). Jamais au portable : son seul GPU
  # porte l'écran de Lucie, et `igpu:0` l'y désignerait.
  export RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE=igpu:0
else
  unset RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE
  export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980
  export RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70
fi
export RAG3WEAVER_TARGET_REPRENDRE=1
# Épinglée : une bascule de `courant` ne change pas la lib sous la passe.
export RAG3DB_LIB=rag3db-build-59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF
# Depuis le 16 novembre 2026 : lib 59ac9c86bc posée aux deux postes, hôtes source 927941a67a ;
# les passes de cette source ne demandent plus MOTEUR_ANCIEN. Avant : 373b5477b (14 novembre),
# puis 29a7bc52a, 9bae60786, cbc06c3e1, dadbf0041.
# Le plafond du target de run_e2e.sh, 32 Go mesurés par G (d845744a2) : une branche d'avant garde 20.
export RAG3WEAVER_TARGET_MAX_GO=32
# Les paquets React des témoins de l'aperçu à chaud (scripts/paquets_react.sh), s'ils sont déballés.
[ -f $HOME/.cache/rag3weaver-build/paquets-react/react-refresh/package.json ] && export RAG3WEAVER_PAQUETS_REACT=$HOME/.cache/rag3weaver-build/paquets-react
FEAT=rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm
R=$J/resume.log
# La base testée, pour la garde hors pile (15 nov. 2026, note d'ED) : le dernier commit de master sous la tête.
git -C $W merge-base HEAD origin/master > $J/base 2>/dev/null || rm -f $J/base
echo "commit $(git -C $W rev-parse --short HEAD) — $(date '+%d/%m %H:%M') — lib ${RAG3DB_LIB:13:9}" > $R
rouge() { echo "ROUGE $1" >> $R; exit 1; }
# La règle 7 : un C++ du lot plus neuf que la lib épinglée se joue sous RAG3WEAVER_MOTEUR_ANCIEN=1, dit ici.
[ -n "${RAG3WEAVER_MOTEUR_ANCIEN:-}" ] && echo "RAG3WEAVER_MOTEUR_ANCIEN=$RAG3WEAVER_MOTEUR_ANCIEN : le C++ du lot est plus neuf que la lib ${RAG3DB_LIB:13:9}" >> $R

# **Les suites qu'exige le lot** (16 novembre 2026, ticket de c7) : master a rougi sur e2e_code_sync quand chaque passe
# disait « e2e_code 36/36 », le seul binaire e2e_code. tools/poste/required_suites.txt, dans l'arbre du lot, dit quels
# chemins exigent quelle suite ; elle s'ajoute à la passe, même sous SUITES, et la garde du push la redemande.
EXIGEES=""
if [ -x $W/tools/poste/required_suites.sh ] && [ -s $J/base ]; then
  EXIGEES=$(bash $W/tools/poste/required_suites.sh "$(cat $J/base)" HEAD) || rouge "suites exigées : required_suites.sh a échoué"
  [ -n "$EXIGEES" ] && echo "suites exigées par le lot : $(echo "$EXIGEES" | awk '{printf "%s%s (%s)", (NR > 1 ? ", " : ""), $1, $2}')" >> $R
fi
EXIGEES_NOMS=$(echo "$EXIGEES" | awk 'NF {print $1}' | tr '\n' ' ')
echo "== témoin" >> $R
# SUITE_DU_TEMOIN=lib (15 novembre 2026, ticket d'hotfixs) : le témoin est un test de la lib, joué par `run_e2e.sh --lib`.
if [ "${SUITE_DU_TEMOIN:-}" = lib ]; then
  timeout 3600 ./run_e2e.sh --features openai-llm,anthropic-llm --lib "$FILTRE" > $J/temoins.log 2>&1; c=$?
else
  timeout 3600 ./run_e2e.sh --features openai-llm,anthropic-llm --test ${SUITE_DU_TEMOIN:-e2e_lectures} "$FILTRE" > $J/temoins.log 2>&1; c=$?
fi
grep -E '^test result|panicked at' $J/temoins.log >> $R
[ "$SEUL" = temoins ] && { echo "TEMOINS code=$c" >> $R; exit $c; }
[ $c = 0 ] || rouge "témoin (code=$c)"

# SUITES=<liste> (14 novembre 2026, au mot de c7) : la liste des suites e2e remplace celle par défaut
# (e2e_lectures, e2e_code, SUITE_DU_TEMOIN, SUITES_EN_PLUS) ; SUITES=- n'en joue aucune. Le témoin,
# check_features, la preuve, la lib entière et le témoin navigateur restent : c'est le rejeu court
# d'un lot qui ne touche ni le bus ni le code indexé (une ligne de lib, un crochet de test).
if [ -n "${SUITES:-}" ]; then
  LISTE_DES_SUITES=$(echo "$SUITES" | tr "," " " | sed 's/^-$//')
  echo "suites réduites : ${LISTE_DES_SUITES:-aucune} (SUITES=$SUITES)" >> $R
  LISTE_DES_SUITES="$LISTE_DES_SUITES $EXIGEES_NOMS"
else
  # La lib n'est pas une suite e2e : elle se rejoue entière à l'étape « lib ».
  LISTE_DES_SUITES="$EXIGEES_NOMS e2e_lectures e2e_code $([ "${SUITE_DU_TEMOIN:-}" = lib ] || echo "${SUITE_DU_TEMOIN:-}") ${SUITES_EN_PLUS:-}"
fi
JOUEES=""
for suite in $(echo $LISTE_DES_SUITES | tr " " "\n" | awk 'NF && !vu[$0]++'); do
  echo "== $suite" >> $R
  JOUEES="$JOUEES $suite"
  timeout 5400 ./run_e2e.sh --features openai-llm,anthropic-llm --test $suite > $J/$suite.log 2>&1; c=$?
  grep -E '^test result|FAILED|panicked' $J/$suite.log | head -20 >> $R
  [ $c = 0 ] || rouge "$suite (code=$c)"
  # Une suite non jouée faute de feature est rouge (procédure, 7 novembre 2026).
  grep -q 'NON JOUÉES' $J/$suite.log && rouge "$suite : des suites non jouées faute de feature"
done

echo "== check_features" >> $R
scripts/check_features.sh > $J/check_features.log 2>&1; c=$?
tail -7 $J/check_features.log >> $R
if [ $c -ne 0 ] || grep -q '^ROUGE' $J/check_features.log || ! tail -1 $J/check_features.log | grep -q '^CHECK_FEATURES VERT'; then rouge "check_features"; fi
touch src/lib.rs
timeout 1800 scripts/cargo_magasin.sh check --tests --features $FEAT > $J/preuve.log 2>&1
grep -Eq '(Checking|Compiling) rag3weaver ' $J/preuve.log || rouge "pas de Checking ni de Compiling rag3weaver"
echo "preuve : rag3weaver revérifié" >> $R

echo "== lib" >> $R
timeout 1800 scripts/cargo_magasin.sh test --lib --features $FEAT > $J/lib.log 2>&1; c=$?
grep -E '^test result|FAILED|panicked' $J/lib.log | head -20 >> $R
[ $c = 0 ] || rouge "lib (code=$c)"

# Les témoins navigateur (depuis 3ec37ef1e, ignorés dans la lib sans la feature `navigateur`) :
# une étape à part, après la lib, navigateur exigé (un SAUTÉ y est rouge).
if [ -x scripts/temoins_navigateur.sh ]; then
  echo "== témoins navigateur" >> $R
  FEATURES=$FEAT timeout 1800 scripts/temoins_navigateur.sh > $J/navigateur.log 2>&1; c=$?
  grep -E '^test result|panicked at|SAUTÉ|ROUGE' $J/navigateur.log | head -10 >> $R
  [ $c = 0 ] || rouge "témoins navigateur (code=$c)"
elif [ -n "${5:-}" ]; then
  # Un arbre d'avant 3ec37ef1e : le témoin navigateur nommé, seul et exigé.
  echo "== témoin navigateur" >> $R
  RAG3WEAVER_EXIGER_NAVIGATEUR=1 timeout 1800 scripts/cargo_magasin.sh test --lib --features $FEAT -- "$5" > $J/navigateur.log 2>&1; c=$?
  grep -E '^test result|panicked at' $J/navigateur.log >> $R
  [ $c = 0 ] || rouge "témoin navigateur (code=$c)"
fi
echo "suites jouées :${JOUEES:- aucune}" >> $R
echo VERT >> $R
