# Session recherche, 8 novembre 2026 : knowledge dump

Ce qu'une session qui reprend doit savoir sans relire la journée.

## Les scripts de passe (single source : ~/.cache/rag3weaver-build, copiés à luciepc)

- `passe_epinglee.sh <worktree> <journaux> <filtre du témoin> [temoins|-]` : le témoin, e2e_lectures,
  e2e_code, `SUITE_DU_TEMOIN`, `SUITES_EN_PLUS`, check_features, la lib entière, les témoins
  navigateur. La dernière ligne de `resume.log` dit VERT ou ROUGE.
- `lib_seule.sh <worktree> <cible> <journal> <filtre>` : un test de la lib seul ; `FEAT=` règle les
  features (ajouter `navigateur` pour un témoin navigateur).
- `e2e_entier.sh <worktree> <cible> <journal>` : toute la passe e2e, une mesure, pas un push.
- `push_garde.sh <worktree> <journaux>` sous `poste push rag3db-97 --` : il pousse si la passe est
  VERTE sur ce commit, ou sur un commit qui n'ajoute que des .md, sous une seule lib, et sans code
  arrivé sur master depuis la base testée. Puis les trois contrôles.
- Pour de la doc seule depuis ici : `push_doc.sh <worktree>` (scratchpad), sous `poste push`.
- La lib épinglée depuis le 8 novembre 23 h 40 est 9bae60786, sans MOTEUR_ANCIEN. On édite un
  script ici, puis on le copie vers un fichier temporaire suivi d'un `mv`, jamais par-dessus un
  script qui tourne.

## Le registre des fils d'un catalogue (arret-du-backend)

- `catalog_threads::{register, running, latest, stop_and_join}`. Une entrée porte un `Weak` du
  catalogue, son drapeau d'arrêt, son genre (`Index(EmbedWatch)` ou `FtsRebuild`) et son fil.
- `index_nodes::indexings_running` et `embed_watch_of` ne comptent que les index (accordé avec ED :
  close_slot, lancer_sur, index_states). `stop_indexings_of` arrête et joint tous les fils.
- `Backend::shutdown` est idempotent par `ferme`. Le Drop l'appelle. Une réaction figée laisse le
  catalogue ouvert et se dit.

## Le témoin de suite (residus-de-suite)

- La fonction `residus_de_la_suite` de run_e2e.sh juge le TMPDIR de chaque suite avant son
  `rm -rf`. Format de la liste : `<suite>` (toujours, propre est rouge) ou `<suite> <ticket>`
  (intermittent : « résidu connu : … »). Sous un filtre de tests, « propre est rouge » ne
  s'applique pas.
- Le cache d'index des suites va dans leur dossier disque (`RAG3WEAVER_CACHE_DIR`), effacé avec
  elles.

## Mes recherches

- La source est le journal des runs (`appel {client, humain, outil, arguments, rendu}`). Une
  recherche est l'appel d'un outil dont le graphe porte `SEARCH_NODES` (SearchSourceNode).
- `JournalDesRuns::append_to_list` passe sous le verrou `ecritures`. `reecrire` garde l'âge du
  fichier.
- `SEARCH_VIEW_TOOL` (ide.rs) est l'outil de la vue recherche ; une ligne est `relancable` si elle
  vient de lui.

## Les gestes de la journée

- Un rouge se prouve sur la base sans le correctif. Si le correctif est déjà commité, on applique
  le commit du témoin sans le valider sur master (`cherry-pick --no-commit`, avec les témoins dont
  il dépend), puis on revient à la tête.
- Un e2e entier qui embarque est un lourd GPU : il passe quand la carte est à personne (c7).
