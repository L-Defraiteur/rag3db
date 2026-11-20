# L'état d'une vue sans montre ne voit pas son retard

- **État** : corrigé (20 novembre 2026), par le lot `arbre-vue-sans-montre` (7e8963f71f, PR 202) ; témoin
  `views::tests::a_view_without_a_watch_sees_its_lag`, rouge f781c78ae7 puis vert.
- **Famille** : produit
- **Gravité** : une vue dont la couche n'a pas de montre se dit à jour même quand ses fichiers ont changé depuis
  son index. C'est le comportement d'aujourd'hui, par cellule : rien ne régresse, mais rien ne le dit.
- **Atteignable en service** : oui, pour un backend sans montre déclarée.
- **Touche rag3weaver** : oui.

## Ce que c'est

`views::view_state` lit `files_behind` dans `Lag`, pour la couche de la vue. Seule la montre y note, y compris par
sa réconciliation de départ. Sans montre, rien n'y est noté : `files_behind` vaut 0 et `up_to_date` est vrai.

L'accord avec recherche, le 18 novembre, prévoyait deux sources :

- le compte de `Lag` quand la montre de la couche tourne ;
- un diff d'arbres seulement à froid, sans montre : l'arbre noté de la couche (`indexed_tree`, racine, commit et
  état sale) contre l'arbre de sa pointe.

## Ce qu'il faut pour le fermer

- La marque d'arbre indexé **par couche** : aujourd'hui, `indexed_tree:<org>/<project>` est par cellule.
- `Lag` sait si une montre sert la couche.
- `view_state` fait le diff quand aucune montre ne sert la couche, hors du verrou du catalogue (`git` coûte), avec
  un résultat gardé tant que la pointe ne bouge pas.
- Un témoin rouge d'abord : une couche sans montre, un fichier changé depuis son index, la vue ne se dit pas à jour.

## La fermeture (20 novembre 2026)

- `Lag` sait si une montre sert une couche : `Lag::watching_guard`, tenu par le fil de la montre toute sa vie, et
  `Lag::is_watched` ; `ViewState::watched` le dit.
- La marque d'arbre indexé est par couche (`Catalog::note_indexed_tree_of_layer`) : la couche du backend garde la clé
  de la cellule ; une couche d'histoire note `{root, commit, ref}` à l'écriture de sa pointe ; une couche de travail,
  l'arbre de son worktree. La marque garde le blob de chaque fichier modifié tel qu'indexé (`dirty_blobs`).
- `views::cold_files_behind` fait le diff hors du verrou du catalogue, et le garde tant que la pointe ne bouge pas
  (le HEAD, l'heure et la taille de chaque chemin que `git status` nomme) ; `ViewState::with_cold_behind` ne
  l'applique qu'à une vue sans montre.
- **Reste dit** : les fichiers non suivis par git ne comptent pas, la marque ne les note pas. Et `view_state` n'a pas
  encore d'appelant du produit : c'est la réponse des outils, chez recherche, qui l'exposera.
