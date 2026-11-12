# Adjacence des undo du scénario vingt écrivains — lecture seule

12 novembre 2026. Ticket :
[annulation sous pool plein](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md),
pour cœur c++. Sources relues dans le worktree privé au commit moteur
0511dece74f94db6937fb03e6ec45f0c58828c5a. Aucun run, build ou changement moteur.

## Conclusion pour le choix A

Le scénario ne justifie pas de choisir le regroupement strict du plan 10.
Pour les insertions ordinaires de Doc et Note, chaque transaction contient au
plus **une ligne nouvelle de sa table de nœuds**. Il n'existe donc pas une série
de plusieurs INSERT_INFO de cette table à fusionner. Ce n'est pas la preuve
qu'une série serait systématiquement séparée par des UPDATE_INFO : la série
n'existe pas sur ce chemin. Le gain N×U du microbanc ne démontre aucun gain sur
ce scénario. Le plan A reste une hypothèse pour COPY et les transactions à
plusieurs plages ; il n'est plus proposé comme prochain correctif de ce cas.

## Chemin sourcé des écritures ordinaires

- `test/transaction/concurrence/twenty_writers_scenario_test.cpp:423` : transact
  ouvre et termine la transaction de chaque itération ; une itération sur dix
  est un bloc explicite, sans regrouper dix itérations.
- Même fichier, lignes 470–492 : DocWriter fait un MERGE sur une clé primaire,
  trois SET et une relation Link ; NoteWriter fait un CREATE d'une seule Note.
  Le bloc explicite sépare MERGE et relation en deux statements, pas en lots de
  créations Doc. La clé primaire borne le MERGE à une ligne.
- `src/storage/table/node_table.cpp:575` : insert passe par LocalNodeTable.
  `src/storage/local_storage/local_node_table.cpp:104` puis ligne 116 : insertion
  locale par `nodeGroups.append(&DUMMY_TRANSACTION, ...)`.
  `src/storage/table/node_group_collection.cpp:340` n'enregistre les INSERT_INFO
  que pour une collection persistante avec shouldAppendToUndoBuffer.
  Les SET de la ligne encore locale ne constituent pas des insertions persistantes
  supplémentaires.
- `src/transaction/transaction.cpp:175` : le commit appelle LocalStorage::commit
  avant de publier les versions d'undo. `src/storage/local_storage/local_storage.cpp:63`
  traite les tables NODE avant les tables REL.
- `src/storage/table/node_table.cpp:850` : le commit transfère les groupes locaux
  via NodeGroupCollection::append. Dans `node_group_collection.cpp:67`, la boucle
  produit une plage par morceau transféré. Avec une seule ligne locale non vide,
  il y a au plus une plage de la table Doc ou Note, même à une frontière de groupe.
  Un échec avant ce transfert peut n'en laisser aucune.

Un MERGE qui trouve Doc peut produire des UPDATE_INFO sur les colonnes existantes
(`NodeTable::update`) et des effets d'index secondaire. Ceux-ci ne prouvent pas
une alternance répétée INSERT_INFO/UPDATE_INFO de Doc : il n'y a pas de seconde
création Doc dans cette transaction. Les vingt connexions possèdent leurs
transactions ; leurs undo ne sont pas concaténés dans un tampon commun.

## Relations et création de l'index : distinction indispensable

Les relations Link et les relations internes HNSW ont leurs propres handlers.
`src/storage/table/rel_table_data.cpp:288` choisit un handler CSR persistant ou
mémoire. Même si leurs INSERT_INFO sont contigus dans un tampon, le plan 10
propose un opt-in NodeTable seulement : ces records ne fusionnent pas avec Doc.

`extension/vector/src/function/create_hnsw_index.cpp:453` et ligne 455 créent
les tables de relations internes ; le chargement utilise HNSWRelBatchInsert.
Ce chemin peut produire plusieurs plages **de relations**. Il ne faut donc pas
étendre « une plage Doc par itération » à toute la transaction DDL ni prétendre
que tous les INSERT_INFO du scénario sont isolés. L'ordre exact et les frontières
de tampon du DDL ne sont pas enregistrés dans les traces disponibles ; aucun
nombre de fusions possibles de ces relations n'est déductible ici. Leur éventuel
regroupement demanderait une autre preuve de composition des handlers CSR,
hors opt-in NodeTable proposé.

## Ce que les traces conservées prouvent et ne prouvent pas

Lecture distante des fichiers existants, sous
`~/.cache/rag3db-moteur-notes/annulation/` à luciepc :

- `isole-TwentyWriters.DdlEveryMinute.log` : refus BEGIN pool plein, durées DDL,
  erreurs et agrégats de fenêtres ; aucune séquence typée d'undo avec handler,
  plage et identifiant de tampon.
- `base-TwentyWriters.DdlEveryMinute.log` : CREATE_VECTOR_INDEX dure 752409 ms,
  agrégats par acteur ; aucune séquence d'undo non plus.
- `trace.log:304` et `:553` identifient le **témoin COPY**, pas TwentyWriters.
  La pile `:539` atteint rollbackPKIndexInsert. Elle prouve le chemin d'allocation
  du témoin rouge, pas l'adjacence des records du scénario vingt écrivains.

Ces journaux ne permettent ni un taux d'adjacence observé, ni un nombre de scans
PK annulés, ni une attribution du différentiel DDL au facteur U. La conclusion
sur les écritures Doc/Note est structurelle, tirée du code ; celle du DDL reste
bornée par l'absence de séquence enregistrée. Aucun nouveau run pour combler cela.

## Suite proposée à la revue

Ne pas activer A pour traiter TwentyWriters sur la seule base du microbanc.
Conserver séparément la correction d'absence de matérialisation des clés sous
pool plein et la question de son coût. Avant une nouvelle implémentation, choisir
avec la revue le chemin réellement visé : une plage Doc (réduire le coût d'un seul
scan), COPY à plusieurs plages (A éventuellement utile), ou relations CSR du DDL
(autres invariants). Aucune de ces options n'est autorisée ou exécutée ici.
