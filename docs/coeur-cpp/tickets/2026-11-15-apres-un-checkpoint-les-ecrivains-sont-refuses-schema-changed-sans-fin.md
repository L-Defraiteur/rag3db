# Après un CHECKPOINT, les écrivains d'une table sont refusés « Schema changed, retry: » sans fin

- **État** : ouvert (15 novembre 2026, cœur c++) — vu une fois en 42 tours, non reproduit depuis
- **Famille** : moteur
- **Gravité** : blocage (les écritures d'une table et ses ALTER refusés jusqu'à la réouverture)
- **Atteignable en service** : à établir. Vu sous vingt écrivains et un pool plein. Les hôtes de
  Lucie jouent `373b5477b`, d'avant le lot du verrou de schéma : rien n'est exposé en service.
- **Touche rag3weaver** : par le moteur. L'écrivain qui recommence sur ce refus, comme il le doit,
  tourne en rond.
- **Signalé par** : cœur c++. `TwentyWriters.DdlEveryMinute` ×10 sur `ee598e074` (le correctif de
  l'ADD refusé à mi-chemin, sur master `101ff50a6`, qui porte le lot du verrou de schéma).
- **Pour** : cœur c++

## Ce qu'on voit

Tour 7 sur 10, joué à luciepc le 15 novembre, journal
`~/.cache/rag3db-moteur-notes/ddl10-fix/ddl-7.log`. Sous le pool plein, l'ADD rang (minute 2), le
CREATE REL TABLE (3) et le CREATE_VECTOR_INDEX (4) sont refusés par le gestionnaire de tampons. Le
CHECKPOINT de la minute 5 réussit (3,2 s). À partir de lui, les 20 écrivains de Doc reçoivent
« Schema changed, retry: the schema of table Doc changed after this transaction started… » à
chaque essai. Ils recommencent pendant 75 s, jusqu'à l'arrêt du scénario. L'ADD a et l'ADD b de
la minute 6 sont refusés de même : d'où `failed-migration-disturbs-no-one`, puis « Cannot find
property b ».

## Ce qui est établi

- Un refus **sans fin** n'est possible que si la tête de l'entrée du catalogue est une version
  **non validée** (un identifiant de transaction, au-delà de `START_TRANSACTION_ID`). Une tête
  validée à C ne gêne plus aucune transaction partie après C.
- Avant le CHECKPOINT, les écrivains échouaient presque tous sur le pool plein (63 134 refus hors
  DDL) : ils n'atteignaient pas le contrôle. Le CHECKPOINT libère le pool, et ils l'atteignent.
- Ce n'est pas une annulation marquée à moitié faite : avec `rollbackFailed`, le CHECKPOINT est
  refusé ; il a réussi.
- `UndoBuffer::rollback` s'arrête au premier enregistrement qui lève : ceux qui suivent, dont celui
  du catalogue, ne sont pas défaits. C'est une source possible de tête non validée qui reste, mais
  elle devrait marquer l'annulation à moitié faite. Non prouvé.

## Les essais qui ne le reproduisent pas

- Quatre suites courtes : un ADD refusé par le point de faute `add-column-between-chunked-groups` ;
  un CREATE_VECTOR_INDEX ; un ADD refusé et un CREATE REL TABLE ; un ADD réussi. Chacune est suivie
  d'un CHECKPOINT puis d'écritures : tout passe.
- 20 tours de `DdlEveryMinute` sous un binaire sonde, hors dépôt, où le contrôle dit sur stderr
  la tête et l'instantané quand il refuse : aucun tour bloqué. Dans les 12 derniers, la sonde
  n'écrit que les têtes non validées : aucune vue. Journaux :
  `~/.cache/rag3db-moteur-notes/sonde-diag/`.

- 12 tours de plus le 16 novembre, sur master `2385b5f616` (le pool corrigé), sous la sonde filtrée
  et un pool réduit pour recréer des DDL refusés. À 384 Mio, aucun DDL n'est refusé : le pool ne
  sature plus. À 200 Mio, seul le CREATE_VECTOR_INDEX l'est, jamais un ADD ni un CREATE REL TABLE.
  Aucun tour bloqué, aucune tête non validée vue. Journaux :
  `~/.cache/rag3db-moteur-notes/stuck-probe/`. Depuis le correctif du pool, les conditions du tour
  bloqué (des ADD et des CREATE REL TABLE refusés par le pool plein, puis un CHECKPOINT qui
  réussit) ne se recréent pas en jouant sur la taille du pool.

- Une course vue par TSan le 16 novembre (passe du dictionnaire sous 200 Mio, sur master
  `64eee2e1f`, 7 rapports en deux tours) : le COMMIT écrit l'horodatage d'une entrée du catalogue
  (`UndoBuffer::commitCatalogEntryRecord` → `CatalogEntry::setTimestamp`) sans le verrou du
  `CatalogSet`, pendant qu'un lecteur le lit sous ce verrou
  (`CatalogSet::traverseVersionChainsForTransactionNoLock`, depuis `PlanMapper::mapScanNodeTable`).
  Une écriture de 8 octets alignés : elle ne se déchire pas sur x86-64, et elle ne peut pas laisser
  pour toujours une tête non validée. Pas la cause probable ; à garder pour la lecture. Rapports :
  `~/.cache/rag3db-moteur-notes/dict-tsan/tsan-<tour>.*` à luciepc.

## Pour avancer

- La sonde reste posée (`scratchpad` de cœur c++ : `sonde-diag.py`, à appliquer à
  `src/catalog/catalog.cpp`). Rejouer `DdlEveryMinute` sous elle jusqu'à un tour bloqué, et lire
  la tête : son identifiant, et la transaction qui l'a posée.
- Une garde possible, indépendante de la cause : une tête non validée dont la transaction n'est
  plus active n'est pas un changement de schéma. À ne poser qu'avec la cause, pour ne pas masquer
  une version morte du catalogue.

## Pour fermer

La cause lue sur un tour bloqué, un témoin, puis le correctif.
