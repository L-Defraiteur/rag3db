# Tentative 5 des fiches classées — préparée le 9 novembre 2026, pas encore jouée

*La question que pose l'outil `archive` (pas 3 de M4, ff7a18fe2) : que fait le modèle d'une
fiche dont le sujet est net mais qu'aucun candidat ne porte ? Même jeu que la tentative 4
(`tests/data/banc-fiches-classees/v1.json`, 39 fiches, 10 sujets, trois candidats par
`recall_subjects`), même forme par fiche, même instruction, plus une étiquette : `create`.*

## Ce qui change

- **Les étiquettes fixes** sont celles d'`archive` : `none` et `create` (`tentative.json` les
  nomme ; la tentative 4 disait `aucun` et n'avait pas de création).
- **L'instruction** garde celle de la tentative 4 et ajoute la phrase de création d'`archive` :
  « si le sujet de la fiche est net et qu'aucun candidat ne le porte, réponds create ».
- **Le compte** reste celui de la tentative 4, plus trois nombres : les créations sur une fiche
  sans sujet (8 dans le jeu), celles sur une fiche dont le bon sujet manque aux candidats (2 à
  la tentative 4), et les **créations fautives**, quand le bon sujet était candidat. Une
  création ne compte pas comme abstention.

## Ce que la mesure tranche

Si le modèle crée là où le bon sujet était candidat, `archive` démarre avec `allow_create`
éteint pour Lucie, et le dit (page 06 de la mémoire longue, §6). Les créations sur les huit
fiches sans sujet se lisent une à une : un thé l'après-midi n'est pas un sujet, un connecteur
Slack ou le rechargement du gestionnaire de fenêtres en sont peut-être.

## Comment la jouer

À luciepc, la lib des passes épinglée (`exporter_la_lib`), avec le service d'embarquement et
les identifiants Vertex. Le banc est une suite « à services » : `run_e2e.sh` ne la joue que
nommée par `--test`, et la refuse sans sa voie.

    cd extension/rag3weaver
    RAG3WEAVER_BANC_VERTEX=1 RAG3WEAVER_LLM=vertex \
    RAG3WEAVER_BANC_TENTATIVE=templates/experiments/2026-11-09-fiches-classees-5 \
    RAG3WEAVER_BANC_SORTIE=templates/experiments/2026-11-09-fiches-classees-5/resultats/llm-gemini.json \
    ./run_e2e.sh --test e2e_banc_fiches_classees

Avant, la même commande sur la tentative 4 (`2026-10-18-fiches-classees-4`) prouve que le
banc changé la compte toujours de même.

39 décisions, environ six minutes à la tentative 4.
