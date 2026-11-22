# Le graphe dépend de la taille des paquets de fichiers

- **État** : ouvert (23 novembre 2026, 00 h 4x)
- **Famille** : produit
- **Gravité** : moyenne. Le même arbre indexé par le même binaire ne donne pas le même graphe selon la taille des
  paquets de fichiers (`RAG3WEAVER_BATCH_FILES`) : des liens et des symboles en plus ou en moins, que rien ne dit.
  Une mesure qui change la taille des paquets compare alors deux graphes différents ; et une borne des paquets en
  durée (le lot de l'arrêt pendant une repasse) rendrait le graphe dépendant de la vitesse de la machine.
- **Atteignable en service** : oui. Toute indexation entière, dès que l'arbre tient en plus d'un paquet.
- **Touche rag3weaver** : oui (`code_sync`, la résolution entre paquets, `Catalog::ingest_code`, les rendez-vous)
- **Pour** : session codeparsers
- **Signalé par** : la session codeparsers (mesure du coût d'`OCCURS`, 23 novembre 2026)

## Constat

Un même instantané de rag3db (fe38b5e5d8, 9 656 fichiers, 8 830 retenus) indexé de rien par le chemin de l'hôte, en
plein texte seul (le banc `worktrees_index_cost_measure.rs` de la branche `codeparsers/occurrences-cost-measure`), sous
`poste mesure` à la tablette, par le même binaire (8e42489ffa, release) :

| | paquets de 2 048 (défaut) | paquets de 128 |
|---|---|---|
| `Symbol` | 73 219 | 73 364 |
| `CONSUMES` | 131 733 | 131 770 |
| `MENTIONS` | 367 650 | 369 657 |
| `OCCURS` | 726 860 | 729 309 |
| `DEFINES` | 99 673 | 99 673 |
| `File`, `Scope` | 8 830, 86 060 | 8 830, 86 060 |

Les fichiers et les scopes sont les mêmes ; les liens et les symboles non. Avec de plus petits paquets, il y en a
**plus** : une part de la résolution dépend de ce qui est déjà en base quand un fichier est écrit.

## Ce qu'il faut

- Un témoin : un petit dépôt à plusieurs fichiers liés indexé d'un seul paquet, puis à un fichier par paquet ; les
  mêmes liens par relation, et les mêmes symboles.
- Trouver ce qui dépend de l'ordre : un rendez-vous (`MENTIONS`) posé quand sa cible n'est pas encore là et qu'un
  paquet suivant ne reprend pas, un symbole créé selon ce qui précède, ou un site d'`OCCURS` compté deux fois.

## Mesure (pour mémoire)

Durée de la passe entière, même instantané et même machine, une passe de chaque côté (bruit non mesuré) :
avant les occurrences (3da33881ce) 120,6 s ; après (8e42489ffa) 131,8 s ; après, paquets de 128 : 185,6 s. Taille de
la base : 1 040,1 Mio, 1 058,3 Mio, 1 056,0 Mio. Journaux à la tablette :
`~/.cache/rag3weaver-build/mesures/occurrences-cout-20261123-001511/`.
