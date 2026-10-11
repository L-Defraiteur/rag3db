# Le COPY de blobs en point de reprise forcé fait échouer la validation sous un petit tampon

- **État** : ouvert — **forme retenue, non engagée ; filet produit en place ; reprise après la
  condition 3** (orchestration, 11 octobre 2026). La forme est dans
  `extension/rag3weaver/docs/10-octobre-2026-coeur-cpp-tickets/04-le-copy-de-blobs-borne-en-octets.md`.
  Le témoin rouge (`BlobCopyCheckpointTest`) attend sur la branche `copy-de-blobs`, hors de master.
  Le filet côté produit (arbre principal) borne en octets les COPY de blobs du paquet.
- **Gravité** : blocage (la validation du paquet échoue, la base est à rouvrir)
- **Atteignable en service** : oui. C'est le stockage en blobs du plein texte de rag3weaver : il
  n'est plus le défaut d'une base neuve, mais toutes les bases d'avant la bascule du 10 octobre y
  sont.
- **Touche rag3weaver** : oui (la poussée des blobs du plein texte, par paquet)

## Ce que c'est

Sous un tampon de 2 Gio, la première indexation de ce dépôt en blobs meurt encore, au-delà de
3 072 fichiers. Le message est celui du ticket fermé
`closed/2026-10-04-point-de-reprise-echoue-quand-le-tampon-est-petit.md` : « valider le paquet :
must reopen … The buffer pool is full and no memory could be freed! A checkpoint of this database
failed ». La base fait 584 Mo à la mort. Ce ticket fermé a été **corrigé pour les écritures ligne
à ligne** (CREATE, MERGE de blobs : segments de chaînes bornés en octets, `47a283e6c`), **pas pour
le COPY**. Le paquet du produit écrit `_index_blobs` par un COPY, en point de reprise forcé.

## La recette

Par le produit (arbre principal, 11 octobre, environ une minute) : sur luciepc,
`~/.cache/rag3weaver-notes-A/confirmation/jouer-blobs.sh`. C'est la recette
`e2e_estimate ce_depot_est_cherchable_par_mots` avec `RAG3WEAVER_FTS=blobs`,
`RAG3DB_BUFFER_POOL_SIZE=2147483648` et 512 fichiers par paquet. Le journal va dans
`~/.cache/rag3weaver-notes-A/confirmation/e2e-blobs.log`. Les résultats sont au ticket
`2026-10-11-confirmer-le-premier-index-sous-un-petit-tampon.md`.

Sans harnais : **à écrire**, d'abord. Un COPY d'un CSV de BLOB dans une table
`(_key STRING PRIMARY KEY, _data BLOB)`, `force_checkpoint_on_copy=true`, sous 256 Mio de tampon,
avec une taille de blobs et un nombre de lignes qui dépassent un quart du tampon. Puis vérifier que
ce cas sans harnais échoue comme le produit avant d'accuser le moteur.

## Reproduit sans harnais (11 octobre, 10 h 40, luciepc, master `daed7a4da`)

Dans `test/transaction/blob_checkpoint_test.cpp`, sous 256 Mio de tampon,
`force_checkpoint_on_copy=true`, avec des blobs d'1 Mio lus d'un CSV :

- `BlobCopyCheckpointTest.SuccessiveForcedCopiesBeyondAQuarterOfTheBufferPool` : sept COPY de 16
  blobs, jusqu'à 112 Mio, **passent** ;
- `BlobCopyCheckpointTest.OneForcedCopyBeyondAQuarterOfTheBufferPool` : **un seul COPY de 112
  blobs échoue**, sur « Buffer manager exception: Unable to allocate memory! The buffer pool is
  full », en 50 s.

Ce n'est donc pas le point de reprise : c'est **le COPY lui-même**, quand une seule instruction
porte plus d'environ un quart du tampon en blobs. Le paquet du produit pousse ses blobs en un seul
COPY.

La pile (gdb, `catch throw` sur l'exception du tampon) :
`NodeBatchInsert::executeInternal` → `NodeBatchInsert::appendIncompleteNodeGroup` →
`InMemChunkedNodeGroup::append` → `StringChunkData::appendStringColumnChunk` →
`DictionaryChunk::appendString` → `ColumnChunkData::resize` → `MemoryManager::allocateBuffer`.

## La cause

Localisée par la pile, pas encore corrigée. Les fils du COPY versent leurs groupes incomplets dans
un groupe en mémoire partagé, qui tient tout un groupe de lignes (jusqu'à 2^17 lignes) avant son
écriture. Pour des blobs, ce groupe pèse toute la donnée de l'instruction. Son bloc de chaînes
grandit par doublements (`bit_ceil`, dans `appendString`), l'ancien et le nouveau tampon
coexistant pendant la copie. Le vidage d'après (`InMemChunkedNodeGroup::flush` → `finalize()` →
`split(true)`) tiendrait en plus sa copie découpée. La lecture de la nuit est donc confirmée sur
le principe, mais l'échec tombe plus tôt, à l'assemblage. Pour des blobs de quelques Mio, un groupe de quelques
centaines de lignes pèse déjà des centaines de Mo, et le double au vidage. Le correctif du ticket
fermé borne les segments, mais pas la mémoire d'un groupe que le COPY tient entier.

## Témoin

À écrire avec la recette sans harnais, rouge d'abord. Puis la confirmation de l'arbre principal
rejouée en blobs.

## Pour le fermer

Un correctif générique : un groupe de blobs qui se vide par morceaux bornés, pas une rustine sur
`_index_blobs` (orchestration). Relecture de la session cœur C++, témoin rouge puis vert, et la
passe en blobs de la confirmation, verte sous 2 Gio.
