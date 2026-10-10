# Une transaction qui mêle des écritures et un COPY revient à moitié après une mort pendant le point de reprise de sa validation

- **État** : corrigé le 5 octobre 2026 (« fix(transactions): une transaction dont la
  durabilité est son point de reprise n'écrit rien au journal — elle revient entière ou pas
  du tout »). Une transaction forcée n'écrit plus rien au fichier du journal ; son point de
  reprise, atomique, emporte toutes ses écritures. La suppression d'une ligne écartée par un
  COPY n'est plus journalisée. Les douze cas du banc quittent `known_red.txt`.
  **Ce qui reste à faire, hors de ce ticket** : journaliser aussi le COPY qui écarte des
  lignes, pour qu'aucun COPY ne soit plus forcé — il faut au journal une forme pour « un
  trou » (une ligne écartée est une clé en double ou nulle, que le rejeu par insertion
  ordinaire refuserait), donc un enregistrement neuf et la question de la version du journal.
- **Gravité** : perte et réponse fausse. Une transaction jamais acquittée revient en partie.
  Avec IGNORE_ERRORS, la base ne se rouvre plus.
- **Atteignable en service** : oui, par le chemin par défaut (COPY avec point de reprise
  forcé). Pour rag3weaver, seulement par la transaction par paquet
  (`RAG3WEAVER_TX_PAR_PAQUET=1`), qui n'est allumée nulle part par défaut (mesures et tests
  seulement) ; son allumage attend ce correctif.
- **Touche rag3weaver** : oui, par la transaction par paquet (MERGE puis COPY).

## Ce que c'est

La validation d'une transaction qui porte un COPY procède en deux temps :

1. elle écrit au journal ses écritures ordinaires (insertions, suppressions, mises à jour)
   et son COMMIT ;
2. elle fait le point de reprise forcé qui rend le COPY durable, puisque les lignes du COPY
   ne sont pas au journal.

Si le processus meurt entre l'écriture du journal et la marque CHECKPOINT, le rejeu rend
durables les écritures ordinaires, sans les lignes du COPY. Le COMMIT n'avait pas rendu la
main.

Pour un COPY sous IGNORE_ERRORS qui écarte une clé en double, c'est pire : la ligne écartée
est insérée puis supprimée, et sa suppression est journalisée sans son insertion. Le rejeu
supprime donc une ligne qui n'existe pas, et la réouverture tue le processus :

```
WALReplayer::replayNodeDeletionRecord → NodeTable::delete_ → NodeGroup::delete_   SIGSEGV
```

C'est le cas même sans aucune autre écriture dans la transaction. Le correctif 9c0c6532d,
de la session cœur C++, ne journalise plus ces suppressions : la base se rouvre, mais la
transaction mêlée revient encore à moitié.

## Recette minimale

`auto_checkpoint=false`, réglage par défaut pour le COPY (`force_checkpoint_on_copy=true`).
On tue le processus pendant le point de reprise du COMMIT, avant sa marque CHECKPOINT :

```
CREATE NODE TABLE Doc(id INT64 PRIMARY KEY, name STRING);
CHECKPOINT;
UNWIND range(0, 99) AS i CREATE (:Doc {id: i});
BEGIN TRANSACTION;
UNWIND range(1000, 1019) AS i CREATE (:Doc {id: i});
MATCH (n:Doc) WHERE n.id >= 1000 AND n.id % 4 = 0 DELETE n;
MATCH (n:Doc) WHERE n.id < 100 AND n.id % 10 = 0 DELETE n;
COPY Doc(id, name) FROM '<1000 lignes neuves>';
COMMIT;   -- meurt dans son point de reprise
```

À la réouverture, 105 lignes : 90 + 15, sans les 1000 du COPY. Attendu : 100 ou 1105.

## Témoin

`single_writer_crash_test.cpp`, `Points/ForcedCopyCheckpointDeath.*`, aux sept instants de
`DyingCheckpointer`. Rouges aux quatre premiers (BeforeStorage, AfterStorage,
AfterSerialize, AfterHeader), 3 passes sur 3, inscrits dans `known_red.txt` :

| Cas | Ce qui rougit | Étiquette |
|---|---|---|
| `ACopyAloneSurvivesWholeOrNotAtAll` | COPY sous IGNORE_ERRORS seul, base inouvrable | `database-opens-without-crash` |
| `AMixedTransactionSurvivesWholeOrNotAtAll` | même COPY mêlé, base inouvrable | `database-opens-without-crash` |
| `AMixedTransactionWithAnOrdinaryForcedCopySurvivesWhole` | COPY ordinaire au réglage par défaut, mêlé, 105 lignes | `transaction-all-or-nothing` |

Avec 9c0c6532d seul, les deux premiers cas se rouvrent : le cas seul tient le tout ou
rien, le cas mêlé revient à moitié (105).

`CREATE_VECTOR_INDEX`, qui garde aussi son point de reprise, n'est pas une forme de plus :
le binder le refuse dans une transaction explicite (« only supported in auto transaction
mode »).

## Pour le fermer

Le remède de la session cœur C++ doit :

- **tenir l'invariant** : une transaction n'est jamais validée sans journal ET sans point de
  reprise abouti ;
- **défaire toute la transaction** quand le point de reprise échoue sans mort.

Les douze lignes sortent alors de `known_red.txt` dans son commit. Le remède (a), un COPY
journalisé même quand il écarte des lignes, reste la cible de l'étape 4.
