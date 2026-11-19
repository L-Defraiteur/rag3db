# Une lecture refusée pendant une réaction en chaîne, environ une fois sur neuf

- **État** : fermé (19 novembre 2026, everything declarative) ; ouvert le 12 octobre 2026
- **Gravité** : test instable, et peut-être une lecture refusée en service (« Catalogue occupé »)
- **Atteignable en service** : peut-être (une réaction de plusieurs centaines de fichiers pendant
  des lectures)
- **Touche rag3weaver** : oui (`dataflow/reindex_nodes.rs`, `dataflow/catalog_read.rs`)
- **Ouvert le** : 12 octobre 2026, session everything declarative, pendant le lot de la porte
  unique de l'index
- **Pour** : everything declarative
- **Famille** : produit

## Ce qui est vu

`backend::tests::a_read_gets_through_a_reaction_that_chains_files` (300 fichiers créés d'un coup,
`usages` toutes les 20 ms pendant leur réindexation) rend « Catalogue occupé » :

- une fois dans la lib entière, et une fois sur trois joué seul, sur la branche de la porte ;
- six fois de suite vert ensuite, sur la même branche, puis trois fois vert sur master nu
  (f8428d4f9). Au total : 8 verts sur 9 avec la porte, 3 sur 3 sans. La fréquence est trop basse
  pour départager.

Dans la passe rouge, la réaction n'avait fait que sept lots (4 à 8 fichiers, tenus 300 à 940 ms,
verrou attendu 0 à 10 ms). Aucun lot ne dépassait la seconde d'attente d'une lecture : quelque
chose d'autre a tenu le catalogue plus d'une seconde à ce moment-là.

## Ce qu'il faut pour le fermer

Savoir qui tient le verrou quand la lecture est refusée. Une piste : `lock_for_read` peut noter
le tenant (le nom du fil, ou l'étape enregistrée en cours) au refus, et la réponse « occupé »
le dire. Ensuite, corriger le tenant, ou dire pourquoi c'est la lecture qui doit attendre plus.

## Fermé le 19 novembre 2026 (ED) : la vue de lecture a retiré le refus

**Toutes les observations datent d'avant la vue de lecture.** La vue (lot L2, `9d1e2dff6b`) est arrivée le
12 octobre à 12 h 22 ; le dernier code observé, master nu `f8428d4f9`, est de 11 h 25. Depuis, une lecture d'outil
(`usages`, `impact`, `callees`, les liens) passe par `read_for_tool`. Quand la vue est publiée, elle essaie le
catalogue sans jamais l'attendre ; tenu, elle lit l'état validé par la vue et le dit (`WRITER_HOLDS`). « Catalogue
occupé » ne peut plus en sortir.

**Le témoin** `backend::tests::a_long_hold_of_the_catalog_does_not_refuse_a_tool_read` tient le catalogue jusqu'à la
réponse de la lecture, plus longtemps que toute attente d'un lecteur. C'est un ordre, pas une durée. `usages` répond,
avec `main`, la ligne de l'écrivain, et sans « occupé ».

Ce n'est pas un rouge vu puis corrigé : c'est la preuve que le mécanisme du ticket n'existe plus. J'avais d'abord
écrit le témoin pour faire nommer le tenant par le refus, comme le ticket le proposait. Il a montré que la lecture
n'attendait plus le verrou.

**Restent possibles**, sans signalement à ce jour :
- `read_catalog_in`, sans vue publiée (avant la première publication) ou pour une autre cellule que celle du
  catalogue ;
- la résolution de la cible d'une recherche (`generic_search_nodes.rs`, `lock_for_read`), qui se replie sur un
  balayage ;
- l'état du backend (`backend.rs`, `{"busy": true}`).

Si l'un d'eux rougit, la piste de ce ticket vaut toujours : une tenue déclarée, que le refus nomme.
