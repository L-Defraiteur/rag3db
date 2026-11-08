# Une base s'ouvre sans dire qu'une extension dont une table dépend n'a pas pu être chargée

- **État** : ouvert — la proposition du refus avec option reste à trancher (orchestration, 8
  novembre 2026). Le lot G3 de hotfixs (8 novembre, Lucie : « rouvre toujours les extensions notées,
  absolument ») a fait le reste : toute ouverture charge les extensions notées, journal vide ou non,
  et un échec se dit au journal du moteur et dans la colonne `extension_error` de SHOW_INDEXES.
  Proposition de l'orchestration du 10 octobre 2026
- **Gravité** : perte d'un index (une donnée dérivée, à rebâtir), sans que l'ouverture le dise
- **Atteignable en service** : oui, si le fichier d'une extension manque ou ne se charge pas au
  moment d'un rejeu
- **Touche rag3weaver** : oui (index vectoriel ; il lit `getRecoveryLoadFailures` en
  avertissement seulement, d'après la session de l'arbre principal)
- **Ouvert le** : 10 octobre 2026, seconde session cœur C++, sur relevé de rag3db-96 et de
  l'orchestration
- **Pour** : cœur C++
- **Famille** : moteur

## Ce que c'est

Quand le rejeu ne peut pas charger une extension (fichier absent ou refusé), la base s'ouvre
quand même ; l'index qui en dépend est détaché et devient « à rebâtir ». L'ouverture réussit
sans erreur : l'échec n'est lisible que par `ExtensionManager::getRecoveryLoadFailures`.

## Ce qui est vérifié dans le code (à `ba11e003b` et suivants)

- `WALReplayer::replayLoadExtensionRecord` (`wal_replayer.cpp:788-795`) charge l'extension par
  `loadExtensionForRecovery` : un échec n'interrompt pas la reprise ; il est rangé dans
  `recoveryLoadFailures` (`extension_manager.cpp:121-127`, lu par `getRecoveryLoadFailures`,
  `extension_manager.h:59`).
- C'est la garde 1 (`fcd9a7882`) : l'index non chargé est détaché (`IndexHolder::detach`), n'est
  plus écrit par le point de reprise ; son entrée reste au catalogue — l'état « à rebâtir ». La
  recherche le refuse par son nom (« is behind its table »), `DROP_VECTOR_INDEX` fonctionne.
- Les lignes de la table ne sont pas perdues. Ce qui l'est, c'est l'index : le point de reprise
  qui suit la reprise écrit la table sans lui, et le journal qui aurait permis de le tenir à jour
  est vidé ; il faut le rebâtir (`DROP_VECTOR_INDEX` puis `CREATE_VECTOR_INDEX`).

## Rapporté, non vérifié ici

- Neo4j refuserait de démarrer une base quand le fournisseur d'un index manque (orchestration) :
  à vérifier dans sa documentation avant de s'y appuyer.

## La proposition

Refuser l'ouverture en écriture quand le rejeu n'a pas pu charger une extension dont un index
d'une table dépend, par une erreur nommée qui dit l'extension et la table ; une option explicite
pour ouvrir quand même (en lecture seule, ou en acceptant de rebâtir). Ce qui change pour
rag3weaver : il reconnaît l'erreur, propose le rebâti, et ouvre avec l'option.

## Le témoin à écrire

Une base avec un index vectoriel, un journal non vide qui porte le `LOAD EXTENSION`, le fichier
de l'extension rendu introuvable, un processus neuf : aujourd'hui l'ouverture réussit et
`getRecoveryLoadFailures` n'est pas vide ; après le correctif, l'ouverture est refusée par son
nom, et l'option l'ouvre. La famille `ExtensionIndexRecovery` du banc a déjà des formes proches.

## Voir aussi

`2026-10-05-extension-chargee-au-rejeu-sans-controle-de-bati.md` (une extension chargée au rejeu
sans contrôle de bâti).

## Vu au banc le 7 novembre 2026

`SingleWriterCrash.DeathDuringDeletesAndVectorSearchKeepsTheIndexExact`
(`test/transaction/concurrence/single_writer_crash_test.cpp`) : une de ses six manches, tuée
après 633 ms avec 2 000 documents, rouvre la base sans l'extension vector dont l'index `doc_index`
dépend (« VIOLATION vector-extension-loaded: index doc_index on Doc: the vector extension is not
loaded, the index cannot be checked »). Mesure : 1 manche sur 6 au banc complet sous charge (lot
projection-lecture-seule-6), 0 sur 36 seul sur le même bâti, vert au banc du lot du retrait. Le
cas est inscrit à `probabilistic.txt` en citant ce ticket ; il en sort dans le commit du correctif.

## Le lot G3 (8 novembre 2026)

- **Toute ouverture charge les extensions notées** à côté de la base, journal vide ou non
  (`WALReplayer::replay`, sur les quatre chemins : sans journal, journal vide, enregistrement de
  point de reprise, rejeu). Une mort juste après un point de reprise laissait sinon l'index attendre
  son extension selon l'instant de la mort : c'est le cas du banc
  `SingleWriterCrash.DeathDuringDeletesAndVectorSearchKeepsTheIndexExact`, retiré de
  `probabilistic.txt` par ce lot.
- **Un échec ne refuse pas la base.** Il s'écrit au journal du moteur (nom, chemin, cause) et se lit
  par `extension_error` dans SHOW_INDEXES : c'est le ticket fermé
  `closed/2026-10-10-echecs-de-chargement-au-rejeu-que-personne-ne-lit.md`.
- **Les témoins** (`test/transaction/vector_index_crash_reopen_test.cpp`) :
  - `ADeathRightAfterACheckpointStillLoadsTheNotedExtension` : rouge avant, 51 au lieu de 50 ;
  - `AMissingNotedExtensionIsSaidBySHOWINDEXES`.
  `AWriteWithoutTheExtensionIsRefusedByName` a changé de prémisse, pas d'attente : une base sans
  son extension est maintenant une base dont la liste manque.
- **Reste la proposition** du refus d'ouvrir avec une option, ci-dessus. Elle n'est pas tranchée.
