# Rapport de session : l'arbre principal, le 13 novembre au soir

Session Claude « arbre principal », après la passation de Codex. Les hash sont sur master sauf
mention contraire.

## Ce qui est livré

- **Le rouge CI du raccord physique** (`73586bf92`). L'écrivain du raccord passe par le registre des
  écrivains, et les photos d'octets attendent la vraie fin de la fermeture (`Weak<ReopenState>`, pas
  `Weak<Database>`). CI verte sur `327c3bba4` (run 43466739505), ticket fermé.
- **La liaison branche → couche** (`0672d624f`), page 30 :
  - l'état des branches d'un dépôt ;
  - les branches suivies et les parentes estampillées dans `_catalog_meta`, résolues dans l'ordre
    estampille, reflog local, fourche ;
  - une tâche entre aux suivies avec sa parente déclarée, et en sort à sa fermeture ;
  - l'état des couches dans `index_states`.

  Les deux défauts de la relecture d'ED sont corrigés : la base suit le rebase, et la marque de
  fermeture ne survit plus à une réouverture. Chaque pas a été vu rouge avant d'être vert.
- **Les pages** :
  - 29, le contrat de la version de scope ;
  - 30, la liaison aux couches, avec les décisions de Lucie, la rétention proposée, le budget de
    30 Go, le plein texte et la réponse du cœur sur le dense.

## Ce qui est en cours : le lot 1a (la version de scope)

Branche `arbre-principal/scope-version-3`, `297f31130`, rebasée sur `0672d624f`.

- La lib entière est verte à luciepc, sur un target neuf : 2363/0/71.
- Les e2e attendent une lib bâtie à la tête du moteur (`0c41f9fe6`). La lib épinglée `e93710f36` est
  plus vieille que ces sources, et c7 ne veut pas de `MOTEUR_ANCIEN` pour 1a, puisque le verrou de la
  clé primaire touche les suites de transaction.
- Le tour du 13 au soir sur `40d35aae0`, avant ce rebase, était vert sur 11 suites sur 12. La
  douzième, `e2e_tx_par_paquet_arret`, rougissait sur une étiquette de compte, corrigée en `6b9fe55f5`.

Ce que 1a a trouvé et corrigé en route, chaque fois rouge d'abord :
- **dans le mécanisme générique des entités de contenu** :
  - la garde des comptes du plein texte comptait les occurrences au lieu des versions ;
  - le rebâti du plein texte en fichiers lisait les lignes de l'entité publique ;
  - l'état d'index ne voyait pas les marques en attente posées pour l'entité publique ;
  - le rebâti en blobs réindexait une entité sans signaux ;
  - les morceaux d'une version privés de leur arête n'étaient pas refaits à la relance ;
- **sur le chemin du code** : le chemin de masse du premier index ignorait l'index vectoriel des
  versions ;
- **un défaut moteur, trouvé par le repreneur d'un paquet défait** : le cœur l'a corrigé, et la lib
  `e93710f36` rend le témoin vert 10/10 ;
- **le refus générique du creux** sur une entité de contenu, conseillé puis retiré : il cassait les
  témoins de Codex.

## Les tickets ouverts par cette session

- 1a-2 : les sondes, bancs et mesures qui lisent `Scope_Chunk` en dur.
- Le creux d'une version, avant son embarquement, doit dire « pas prêt ».
- Le témoin de mort du raccord physique meurt par un signal, une fois sur dix.
- Un fichier de plus apparaît sous `templates/` pendant la lib entière, une fois (non nommé).
- Chez ED, pour moi :
  - l'identifiant du dépôt par la racine de l'ancre ;
  - l'état des couches qui relance git à chaque lecture ;
  - la couche de travail lisible des autres sessions.

## Reprendre

1. À l'id de la lib à `0c41f9fe6` (G, via c7) : l'épingler dans
   `~/.cache/rag3weaver-build/wt-scope-version-tour.sh` et `wt-scope-version-passe.sh` à luciepc.
2. Jouer le tour (worktree `~/.cache/rag3weaver-build/wt-scope-version` à luciepc, branche
   `arbre-principal/scope-version-3`, target `target-scope-version-luciepc`), puis la passe entière
   si le tour est vert.
3. « Vert » à c7, puis au push : la garde (`push_garde.sh --code-apporte`, `--derive`) et les trois
   contrôles.
