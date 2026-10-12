# Une lecture refusée pendant une réaction en chaîne, environ une fois sur neuf

- **État** : ouvert
- **Gravité** : test instable, et peut-être une lecture refusée en service (« Catalogue occupé »)
- **Atteignable en service** : peut-être (une réaction de plusieurs centaines de fichiers pendant
  des lectures)
- **Touche rag3weaver** : oui (`dataflow/reindex_nodes.rs`, `dataflow/catalog_read.rs`)
- **Ouvert le** : 12 octobre 2026, session everything declarative, pendant le lot de la porte
  unique de l'index
- **Pour** : everything declarative

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
