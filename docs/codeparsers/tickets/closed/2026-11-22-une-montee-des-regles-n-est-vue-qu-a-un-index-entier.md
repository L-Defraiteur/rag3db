# Une montée des règles n'est vue qu'à un index entier

- **État** : fermé le 22 novembre 2026 par le lot `repass-on-open` (session codeparsers). Voir « Fermeture ».
- **Famille** : produit
- **Gravité** : moyenne. Une base garde les lignes de l'ancienne analyse tant que personne ne relance `index`. Les
  outils servent donc ce que la montée devait corriger, sans le dire.
- **Atteignable en service** : oui. Constaté à la pose de l'hôte fbe65f7511 (règles 16), à la tablette, le
  22 novembre à 06 h 46.
- **Touche rag3weaver** : oui (l'ouverture d'un backend, le rattrapage, la montre ; `code_sync::sync_source`)
- **Pour** : session codeparsers, ou celle qui tient l'hôte, selon c7
- **Signalé par** : G (constat à la pose), c7, et la session codeparsers (lecture du code)

## Constat

À la pose de l'hôte en règles 16, l'ouverture du dépôt de Lucie n'a repris que 6 fichiers, ceux qui avaient changé
sur le disque. Aucun fichier n'est repassé pour la nouvelle analyse. G a dû lancer `index` (`confirm: true`) à la main.

## Ce que dit le code

- `version_de_l_analyse` (`code.rs`) hache le commit de codeparsers lié, `REGLES_DE_L_ANALYSE` et le gabarit du code.
  Chaque ligne `File` porte cette valeur (champ `analyse`).
- **Elle n'est comparée qu'à un seul endroit** : `code_sync::sync_source`, la passe entière qu'appelle l'outil `index`
  (`index_nodes.rs`). Tout fichier dont la version diffère y repasse, et l'avancement compte
  `repasses_pour_l_analyse`.
- **Le rattrapage de l'ouverture et la montre ne la comparent pas.** Ils ne reprennent que les fichiers dont le
  contenu a changé.

La promesse « une montée des règles fait repasser toutes les bases à leur prochaine synchronisation » (commentaire de
`REGLES_DE_L_ANALYSE`, page 06, § 13) ne vaut donc que pour un `index` entier.

## Ce qu'il faut

À l'ouverture d'un backend, ou à son premier rattrapage : s'il porte une ligne `File` d'une autre version d'analyse
que la courante, l'hôte lance la passe entière. Ou il la propose et `daemon_status` le dit, mais il ne sert pas en
silence une analyse périmée.

## Témoin attendu

Une base indexée sous une version d'analyse, puis rouverte sous une autre (`version_de_l_analyse` changée), sans
`index` explicite. Ses fichiers repassent (le compte `repasses_pour_l_analyse` est égal au nombre de fichiers), ou
`daemon_status` dit qu'une repasse est due. Il faut le voir rouge sur master.

## Fermeture (22 novembre 2026, session codeparsers)

Le choix de c7 est tenu : la repasse se fait à l'ouverture, en fond.

- **Ouverture.** `a_l_ouverture` (`src/daemon_host/worktrees.rs`) a une troisième raison de relancer l'index. Elle
  s'ajoute au premier index coupé et aux vecteurs dus : des lignes `File` **visibles** portent une autre version
  que `version_de_l_analyse`. L'index entier repart alors en fond, quoi que déclare le manifeste, et l'hôte dit
  pourquoi (« n fichier(s) analysés sous une autre version de l'analyse — la repasse de l'analyse »).
- **Pendant la repasse.** L'index tient l'état `EmbedWatch.analysis_repass` (n/N), effacé quand sa génération est
  publiée. L'état de l'index le rend (`analysis_repass`). Chaque lecture d'outil ajoute « Index : repasse de
  l'analyse en cours : n/N fichiers — la réponse porte sur l'analyse d'avant. ».
- **Témoins** (`src/daemon_host/worktrees_analysis_version_tests.rs`) :
  - `a_base_analysed_under_another_version_repasses_on_open` : rouge sans le correctif (« 120 s after the
    reopening, 3 of 3 files still carry the analysis of before »), vert avec, en 5 s ;
  - `a_repass_in_progress_says_so_in_the_state_and_the_answers` : rouge, puis vert.

**Deux pièges du témoin**, à retenir pour qui en écrit un pareil :
- forcer `analyse` par un `SET` ne change pas le hachage de la ligne, et l'écrivain, qui compare le hachage, ne la
  réécrit pas ; il faut indexer sous la version d'avant ;
- une repasse retire les lignes d'avant sans les effacer, donc on ne compte que les lignes visibles.

