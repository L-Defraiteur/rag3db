# Cœur C++, 18 novembre 2026 : rapport de session

La reprise après la semaine de Codex, au mot de Lucie (« chaque session évalue la pertinence de ce
qu'a fait Codex »). L'évaluation est la page 01.

## Ce qui est fait

- **Le TSan du 16 lu** : 111 rapports, aucun sur le dictionnaire, et une course neuve,
  `NodeTable::addColumn` contre `NodeTable::getColumn` (page 01).
- **Le correctif des offsets du dictionnaire**, de Codex, reposé et poussé : master `9fa16affff`.
  `DictionaryChunk::appendString` réserve ses offsets avant de publier les octets. Ticket fermé.
- **C1, le SIGSEGV du dictionnaire au point de reprise, en veille** : le petit test de Codex
  (`StringRewriteAcrossAddColumn.AutomaticCheckpointsBetweenAddsAndDrops`) est vert 400 fois sur 400,
  en quatre variantes, dont l'environnement de Codex (2 fils, 5 cœurs). La consigne pour la prochaine
  occurrence est au ticket.
- **La course des colonnes, prête** (branche `add-column-under-readers`, tête `f4dea9df5`). Le vecteur
  des colonnes de `NodeTable` et de `RelTableData` se lit sous un verrou partagé et ne change que sous
  un verrou exclusif. TSan rouge sur le témoin seul (`60b21c753`), vert sur le correctif ; en Release,
  le témoin, `transaction_test` (393) et known_red verts, `e2e_test` avec les deux rouges de master
  admis.
- **Tickets ouverts** : la course des colonnes ; le point de reprise qui détruit une colonne retirée
  qu'un lecteur plus ancien tient encore (lu dans le code) ; quatre tickets de Codex portés sur master
  (228 anomalies de chaînes, plan du lecteur ouvert, index refusé revu, et celui des offsets, fermé).
- **Le ménage de la tablette** : 37 chemins sous `~/.cache/rag3db-moteur-notes`, environ 4,4 Go
  apparents ; `core-ro-01` et `core-ro-02` gardés (la matière du refus de la plateforme de Codex).

## Reprendre

- **Le push de la course des colonnes** : après la pile Rust (ED, tty, recherche, arbre) et les deux
  lots de scripts de G, au « à toi » de c7. Reposer `add-column-under-readers` sur la tête de master du
  moment, rejouer ce que la garde demande (`CopyFilesWithHomeDir` doit passer vert avec le correctif
  du tilde `3d706f709`), voie des suites C++ transférables, BUILD_ID écrit par le binaire du témoin.
  Le témoin est dans `concurrence_test` (`AddColumnUnderReaders.*`) ; sa preuve rouge est TSan
  (`~/.cache/rag3db-moteur-notes/columns-race/` à luciepc, bâti `rag3db-colonnes/build/tsan`).
- **Ensuite, dans l'ordre de la page 01 et de la passation du 16** : la colonne retirée détruite sous
  un lecteur (témoin TSan ou ASan, puis la forme : `oldestReaderTS` ou des colonnes partagées) ; les
  228 anomalies de chaînes ; le CREATE_VECTOR_INDEX lent (mesure eu-stack d'abord) ; le CREATE REL
  TABLE qui retient un écrivain.
- **En veille** : C1 et le refus « Schema changed » sans fin, chacun avec sa consigne au ticket.
