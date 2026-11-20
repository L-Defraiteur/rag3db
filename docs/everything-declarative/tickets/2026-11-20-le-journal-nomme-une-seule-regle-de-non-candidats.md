# Le journal d'un index nomme une seule règle de non-candidats, et en compte deux

- **État** : ouvert (20 novembre 2026, soir)
- **Famille** : produit (le journal d'un index)
- **Gravité** : faible. Le nombre est juste, son libellé ne l'est pas ; il a fait chercher un écart qui n'en était pas un.
- **Atteignable en service** : oui, à tout index complet d'une base en règles 13 ou plus.
- **Touche rag3weaver** : oui (`src/dataflow/index_nodes.rs`, la ligne `non_candidates_set_aside`).
- **Pour** : everything declarative
- **Signalé par** : G, au repassage du dépôt de la tablette (20 novembre 2026, 21 h 13).

## Ce qu'on voit

« 12373 versions de conteneurs repliés écartées de la recherche (plein texte et morceaux) ». G en attendait 9 449, le
compte des conteneurs repliés sur une copie en règles 9.

## La cause

`Catalog::set_aside_non_candidates` additionne les gestes de **toutes** les règles de `ChunkingConfig::not_candidate`.
Depuis les règles 13, il y en a deux : les conteneurs repliés (morceaux et plein texte) et les déclarations de valeur
courtes (morceaux seulement). Le libellé ne nomme que la première, et dit « plein texte » pour les deux.

## Ce qu'il faut pour le fermer

Le compte par règle, chacun nommé par la règle et par ce qu'elle retire (« N conteneurs repliés : morceaux et plein
texte ; M déclarations courtes : morceaux »). Le rapport de synchronisation porte alors un compte par règle.
