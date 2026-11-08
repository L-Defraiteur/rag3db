# Banc, 7 novembre 2026 : le ménage et l'atterrissage

À la reprise demandée par Lucie : nommer les worktrees obsolètes, libérer le disque, atterrir ce
qui était à moitié fini.

## Le disque libéré

Chaque dossier a été vérifié avant d'être effacé : tête dans master ou rien de neuf, aucun fichier
ouvert (`lsof`).

| poste | effacé | taille |
|---|---|---|
| portable | worktree rag3db-vingt (banc-fils, dans master ; un extension/lucivy non suivi) | 585 Mio |
| portable | worktree rag3db-masque (banc-pente-tardive, dans master) | 781 Mio |
| portable | worktree wt-piles (248bf03cc, dans master) | 433 Mio |
| portable | target-fils-local | 5,6 Gio |
| portable | piles-isole | 1,7 Gio |
| luciepc | worktrees rag3db-mes-a, -b, -c | 3 × 632 Mio |
| luciepc | worktree rag3db-avant et build-avant (le binaire « avant » de la fenêtre du 25 octobre) | 438 + 198 Mio |
| luciepc | build-asan (bâti ASAN du 25 octobre, la vérification de 58b46a9bf faite) | 3,6 Gio |

Total : environ 9,1 Gio sur le portable et 6,1 Gio à luciepc.

Les changements non commités de rag3db-mes-a, -b et -c étaient des copies de mesure du témoin de
(A), `VectorIndexCreateUnderManyWriters.NoWriterWaitsLongDuringTheCreate`, déjà sur master avec sa
ligne known_red. S'y ajoutait une instrumentation (`TCCHRONO` dans `catchUpCommittedAfter`) qui ne
devait pas partir. Leurs diffs sont gardés à luciepc, dans `~/.cache/rag3weaver-build/banc-archives/`.

Gardés : le worktree rag3db-vingt et build-vingt à luciepc, pour la mesure des lignes hors du
graphe.

## Atterri

- **072c467d7** : les témoins V2-2 (banc-temoins-v22) et le témoin hors graphe corrigé
  (banc-hors-graphe-4). Comparateur complet vert sur ea74b5d0a (15 rouges, 306 verts). Les trois
  `CheckpointOpenReader.*` sont verts. Les deux témoins de l'écrivain sous V2-2c sont en known_red
  avec la page 19 du cœur ; seule l'attente rougit, la ligne survit.
- **5d7d9513a** : `SameVectorForEveryRow…/SameRowsFirstCopy` passe à probabilistic.txt (vert une
  fois sur onze au banc complet du cœur).

## Les branches distantes

45 branches `banc-*` sur origin. La suppression distante (`git push --delete`) est refusée par la
permission des sessions et revient à Lucie. Critères proposés : tête ancêtre de master, ou
`git cherry` sans commit `+`, ou remplacée par une version -N. Par ces critères, 32 partent sans
rien perdre ; les autres sont de vieux brouillons dont master porte la version relue.

`banc-elagage-en-cours` n'est pas du travail en suspens. L'élagage est sur master depuis le
10 octobre (9a2818df9) ; la branche n'en est que le premier jet, d'avant la relecture.
`archive/banc-elagage-hnsw` en est une copie, redondante.

## Reste

La mesure des lignes hors du graphe (`VectorIndexMeasure.MeasureSearchOverRowsOutsideTheGraph`,
`hg-mesure.sh` à luciepc), à la fenêtre calme que l'orchestration donnera.
