# Mémoire des index et erreurs de persistance — 19 septembre 2026

## Constats

Deux reconstructions avec collection et capacités dans une même base ont fini en SIGSEGV avec un pool natif de 1 Gio. Avant le deuxième crash, le catalogue a journalisé un échec de sauvegarde des blobs d’index : `Buffer manager exception: Unable to allocate memory! The buffer pool is full and no memory could be freed!`.

Cela établit un manque de capacité du pool pendant cette exécution ; cela ne suffit pas à prouver où ni pourquoi l’accès mémoire invalide se produit ensuite. La mémoire disponible de la machine, mesurée avant relance, était d’environ 62 Gio sur 93 Gio. Ce message concerne le pool de rag3db, pas un épuisement établi de la RAM système ni de la VRAM.

Le lancement expérimental Magic utilise désormais **8 Gio par défaut**. `RAG3DB_BUFFER_POOL_SIZE` peut le remplacer dans le pilote d’ingestion et le lanceur MCP. Ce n’est ni un nouveau défaut global du framework, ni une correction du SIGSEGV.

Les fichiers des deux bases accidentées sont conservés. La nouvelle base est `experiments/mtga/data/engine-search-8g.rag3db`. Elle est reconstruite depuis les sources et une copie fermée proprement de la reproduction ayant ingéré les 6 063 capacités. Aucun WAL accidenté n’est supprimé pour faire passer une ouverture.

## Défaut confirmé dans le framework

`Catalog::flush_blob_store` absorbait l’erreur après émission d’un événement et d’un message stderr. `ingest_entities` pouvait alors retourner un succès et annoncer les disponibilités des index alors que leurs blobs n’étaient pas persistés. La rétention des données dans le tampon autorise une tentative ultérieure ; elle ne prouve pas leur durabilité.

Correction :

- `flush_blob_store` retourne `Result<(), CatalogError>` avec une erreur distincte `IndexPersistence`.
- Ingestion, réindexation, rattrapages et fermeture explicite propagent cette erreur à l’appelant.
- `drain`, dont le contrat retourne un rapport, ajoute une opération en échec, un avertissement et n’annonce aucune disponibilité acquise pour le rapport.
- Le destructeur continue à journaliser : il ne peut pas retourner d’erreur à l’appelant.
- Les blobs en attente sont conservés. Aucune atomicité ni annulation des écritures déjà effectuées n’est promise.

Trois tests injectent une panne de stockage : conservation des octets et erreur à la fermeture ; erreur d’ingestion au lieu d’un acquittement ; rapport de drain en échec sans index annoncé prêt. Les 81 tests du catalogue passent.

## Diagnostic natif

Reproduction à 1 Gio sous GDB dans `/tmp/mtga-memory-repro.rag3db`, séparée de la base Magic. Pile et journal dans `/tmp/mtga-memory-gdb.log`. L’exécution à 8 Gio est suivie dans `/tmp/mtga-search-8g.log`.

Le résultat de cette reproduction et les vérifications de la base reconstruite seront consignés ci-dessous. La correction Rust de propagation d’erreur ne doit pas être présentée comme une correction du crash C++ sans preuve.


## Pile obtenue et correction native

La reproduction non corrigée a bien planté sous GDB à 1 Gio, sur le thread `scheduler-1` :

```text
std::_Hash_bytes
DictionaryChunk::appendString
StringChunkData::finalize
Column::checkpointColumnChunkOutOfPlace
...
Checkpointer::writeCheckpoint
TransactionManager::checkpointNoLock
```

`StringChunkData::finalize` réécrivait chaque index de ligne pendant la construction du nouveau dictionnaire. Si `appendString` échouait sur une allocation ultérieure, le nouveau dictionnaire était détruit mais certains index avaient déjà changé. Ils pointaient donc dans l’ancien dictionnaire avec de mauvaises positions. Une reprise pouvait lire une valeur incorrecte ou une vue invalide ; la pile du crash montre cette dernière utilisée pour un hachage.

Le test natif `StringFinalizeTests` force un manque de mémoire après le premier remappage, pour STRING et BLOB. **Avant correction : les deux tests échouent**, une valeur `replacement` devient `obsolete` et reste incorrecte après reprise. **Après correction : les deux tests passent**, les valeurs et NULL restent intactes après échec puis après reprise.

La correction construit séparément le dictionnaire et le tableau de remappage. Les index vivants ne sont modifiés qu’une fois toutes les allocations de construction réussies. La publication finale ne nécessite plus d’allocation. `needFinalize` est remis à false après succès.

La reconstruction complète à 8 Gio s’est terminée et a passé le test MCP après redémarrage : inventaires exacts, recherche dense sur les capacités, parcours vers les cartes, union pondérée restreinte au deck et dédoublonnage. Les **984 tests Rust** passent. Le scénario natif existant `agg/hash_leak.test` passe aussi.

Le rejeu intégral corrigé à 1 Gio est suivi dans `/tmp/mtga-memory-fixed-gdb.log` ; son issue est à confirmer avant de considérer la reprise sous pression mémoire comme entièrement validée.


### Résultat du rejeu intégral à 1 Gio (20 septembre)

Le rejeu après ces corrections a encore produit un SIGSEGV après 40 lots acquittés, dans la même chaîne `DictionaryChunk::appendString → StringChunkData::finalize → checkpoint`. Les défauts couverts par les tests sont corrigés, mais **ils ne suffisent pas à résoudre le crash réel sous pression mémoire**. La base à 8 Gio a passé les vérifications MCP ; ne pas confondre ce succès avec une validation complète du comportement à mémoire insuffisante. Journal conservé : `/tmp/mtga-memory-fixed-gdb.log`.
