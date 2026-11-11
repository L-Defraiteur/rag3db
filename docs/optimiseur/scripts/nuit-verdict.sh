#!/bin/bash
# Copie de docs/procedures/scripts/verdict-e2e-code.sh pour luciepc, nuit du 8 nov. 2026 : worktree rag3db-controle-G, target-G/passe.
N=~/.cache/rag3weaver-notes-A/lib58
W=~/git_workspaces/rag3db-controle-G
T=$N/v-etat
cd $W || exit 9
[ -z "$(git status --short --untracked-files=no | grep -v codeparsers)" ] || { echo "ARRÊT sale" > $T; exit 8; }
git fetch -q origin master && git checkout -q --detach FETCH_HEAD && git submodule update -q || { echo "ARRÊT checkout" > $T; exit 7; }
echo "$(date +%H:%M:%S) début, sources $(git rev-parse --short HEAD), lib $(cat $N/id | cut -c1-23)" > $T
cd extension/rag3weaver
unset RAG3WEAVER_LINK_COPY_THRESHOLD RAG3WEAVER_MOTEUR_ANCIEN
export CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-G/passe CARGO_INCREMENTAL=0
export RAG3DB_LIB=$(cat $N/id)
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880 RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70
# e2e_code, puis e2e_lectures (7 novembre 2026) : une suite par appel (run_e2e.sh ne garde que
# le dernier filtre) ; la borne de durée dans le lourd, pas devant (les lourds font la file).
verdict=VERT
for suite in e2e_code e2e_lectures; do
  ~/.cache/rag3weaver-build/poste lourd timeout 1h ./run_e2e.sh --test $suite > $N/$suite.log 2>&1
  rc=$?
  [ $rc = 0 ] && grep -q 'test result: ok' $N/$suite.log && ! grep -q 'test result: FAILED' $N/$suite.log || verdict=ROUGE
  echo "$suite rc=$rc : $(grep -m1 'moteur lié.*librag' $N/$suite.log | grep -o 'rag3db-build-[0-9a-f]\{9\}') $(grep -E '^running [0-9]+ test' $N/$suite.log | tail -1) $(grep 'test result' $N/$suite.log | tail -1 | cut -c1-70)" >> $T
  grep -E '^test .* FAILED$|SKIPPED|⚠' $N/$suite.log | head -10 >> $T
done
# Les témoins de la branche dense filtrée sous l'instantané, par leur nom ; et la garde du
# format 43, dont le « non jugé » est un rouge sous la lib d'une bascule 43 (VERDICT_FORMAT=43).
for t in la_branche_dense_filtree_par_langage_repond_dans_l_instantane_de_l_appel \
         une_recherche_dense_bornee_a_un_chemin_tient_sous_l_instantane \
         la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel; do
  grep -qE "^test e2e_lectures::$t \.\.\. ok$" $N/e2e_lectures.log || { verdict=ROUGE; echo "pas vert : $t" >> $T; }
done
if [ "${VERDICT_FORMAT:-}" = 43 ] && grep -q 'non jugé : la lib n.a pas la colonne state' $N/e2e_lectures.log; then
  verdict=ROUGE; echo "non jugé sous une lib de bascule 43 : la lib n'a pas le format 43" >> $T
fi
echo "verdict $verdict" >> $T
echo fin >> $T
