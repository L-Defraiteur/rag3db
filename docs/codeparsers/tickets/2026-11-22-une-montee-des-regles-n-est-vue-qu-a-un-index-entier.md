# Une montée des règles n'est vue qu'à un index entier

- **État** : ouvert (22 novembre 2026)
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
