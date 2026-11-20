# Ce que coûte une fourche, et l'écart d'un gros lot

20 novembre 2026, au soir, à la demande de c7, pour la décision de Lucie sur O2 (page 03). Mesure sur un reflink de la
copie migrée et repassée de G (`mesures/tracked-master-20261120-184533` à la tablette), en release, lib 4ad394f4d2,
sonde `history_layers::tests::sonde_tracked_master` (branche `arbre-principal/sonde-tracked-master-2`, hors push).
Embarqueur de hachage à la dimension de la base (768) : les vecteurs sont faux, mais ils ont la bonne taille.

## 1. Ce qui a été joué

- **La racine indexée** de la copie est `b79878da9b`, figée le 19 novembre. `origin/master` est 579 commits plus loin :
  361 fichiers en diffèrent.
- **Pas 1** : une génération qui porte ces 361 fichiers au-dessus de la génération du backend. La sonde l'écrit comme
  première génération d'une couche `refs/heads/master` (la forme d'avant la décision de 18 h 47) ; c'est **le même
  travail qu'une couche `fourche:<tête de master>`** dans la forme retenue : le diff entre la racine et le commit,
  rien entre les deux.
- **Pas 2** : la couche de travail d'un gros lot, `cx-memoire-check-proposal` (54 commits, 73 fichiers d'écart avec
  sa fourche `2716373b88`), lue par un worktree détaché à la tête de la branche (le worktree de mémoire n'est pas
  touché). La fourche est derrière la tête de master : la sonde y a écrit d'abord la génération de master à ce commit,
  comptée dans le temps.

## 2. Les chiffres

| | fichiers | durée | `File` | `Scope` | `ScopeContent` | morceaux de contenu | `Symbol` |
|---|---:|---:|---:|---:|---:|---:|---:|
| base | — | — | 17 138 | 117 548 | 103 489 | 161 873 | 91 724 |
| fourche à la tête de master | 361 | **557 s** | +354 | +13 066 | +12 711 | +18 967 | +1 139 |
| gros lot, fourche comprise | 73 | **43 s** | +101 | +3 818 | +956 | +1 842 | +78 |

- **La base** : 3 926 → 4 096 Mio (+170 Mio, +4,3 %) pour les deux pas ensemble.
- **Le partage par contenu tient** : le lot ajoute 3 818 `Scope` mais seulement 956 `ScopeContent`. Les scopes des
  fichiers réécrits à l'identique de la fourche ne dupliquent pas leur texte.
- **Le coût d'écriture est d'environ 1,5 s par fichier** dans une couche : 361 fichiers en 9 min 17.

## 3. Ce que ça veut dire pour `open_view`

- **Le coût d'une fourche suit l'écart entre la racine indexée et le commit de fourche**, pas l'âge du lot. En
  service, la racine est l'arbre de Lucie, republié à chaque fin d'index : une fourche sur `origin/master` coûte les
  fichiers changés depuis le dernier index de cet arbre. Ici, 579 commits d'écart sont un extrême (une copie figée).
- **Une fourche se paie une fois** : tout worktree qui fourche au même commit la réutilise. Treize sessions qui
  ouvrent leur lot le même jour sur la même tête de master n'en écrivent qu'une.
- **1,5 s par fichier est lent** pour un geste qu'une session attend : 100 fichiers d'écart, c'est 2 min 30. Deux
  pistes, à mesurer avant de choisir :
  - ouvrir rend le worktree tout de suite et écrit la couche en fond, la vue active disant « index pas prêt » avec son
    estimation jusqu'à la fin (les outils savent déjà le dire) ;
  - mesurer où passent les 1,5 s (l'analyse du code, l'écriture par fichier dans une couche, le plein texte) : la
    réindexation ordinaire d'un fichier n'en coûte pas autant.
- **Rien de ce coût n'exige `tracked`** : la décision de 18 h 47 tient, et `tracked: master` reste une option pour
  qui veut la vue d'historique.
