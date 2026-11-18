# Les outils de référence par des tables à soi, avant l'index (B)

- **État** : ouvert (18 novembre 2026, session recherche). À faire après C, au mot de Lucie.
- **Pour** : recherche et codeparsers
- **Famille** : produit
- **Gravité** : attente. Avant qu'un premier index ait posé ses relations, `usages`, `callees` et `impact` ne
  rendent rien ou rendent une partie : 42 s sur rag3weaver/src, 196 s sur le dépôt entier aujourd'hui. Depuis le
  lot 3 (b2272d070c), la réponse dit qu'un index écrit la vue, mais elle ne répond pas mieux.
- **Atteignable en service** : oui, à tout premier index d'un dépôt, et à chaque réindexation complète.
- **Touche rag3weaver** : oui (`src/code.rs` et la résolution des symboles, les nœuds des outils de référence,
  `backend.rs`).

## Ce que c'est

La page de conception est `docs/recherche/18-11-2026/02-les-outils-de-reference-avant-l-index.md`. Lucie a
répondu « C d'abord, puis B quand même ». C réordonne l'index réel. B est un repli sans base : des tables bâties
par le parsing de la vue, que les outils de référence lisent tant que l'index de la vue n'est pas prêt.

**Le détail retenu** : Lucie ne l'a pas tranché ; c7 recommande, le 18 novembre :
- des **tables compactes**, bâties fichier par fichier : on retient les définisseurs, les mentions et les
  rendez-vous de chaque `CodeAnalysis`, puis on le relâche. Sans elles, l'analyse du dépôt entier tient 8,7 Gio ;
- l'image est bâtie **au premier appel** d'un outil de référence qui passe la porte, pas à l'ouverture du backend.

## Ce qu'il faut pour le fermer

- **codeparsers** : le lecteur de symboles à deux faces (la base, et les tables), les mêmes fonctions de choix,
  et une sonde de parité : arêtes des tables contre arêtes de la base, au couple près, sur rag3weaver/src et
  `src/` du moteur. Rouge d'abord : une face vide ne rend aucune arête là où la base en rend.
- **recherche** :
  - la porte (l'index de la vue absent ou en retard, aucune autre vue explicite) ;
  - l'aiguillage de `usages`, `callees` et `impact` vers les tables ;
  - la ligne « par l'analyse en mémoire, index en cours : ~N s » ;
  - l'invalidation par la montre ;
  - l'image lâchée quand la vue est à jour ;
  - un témoin de bout en bout, rouge d'abord : sur un backend neuf, `usages` répond avant l'index et le dit.
- **La mesure** : la sonde `probe_index_timeline` montre `usages` complet bien avant l'index ; la mémoire des
  tables est relevée sur le dépôt entier.
