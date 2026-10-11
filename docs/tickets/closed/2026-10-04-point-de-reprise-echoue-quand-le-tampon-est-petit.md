# Le point de reprise échoue pendant une indexation quand le tampon du moteur est petit

- **État** : corrigé `47a283e6c` **pour les écritures ligne à ligne (CREATE, MERGE), pas pour le COPY** (la confirmation en blobs du 11 octobre reste rouge : ticket ouvert `2026-10-11-copy-de-blobs-en-point-de-reprise-force-sous-un-petit-tampon.md`) (11 octobre 2026, seconde session cœur C++) — des segments de chaînes bornés en octets ; témoins `StringSplitTest` et `BlobCheckpointTest` (`33792e69d`), rouges avant, verts après
- **Gravité** : blocage (l'indexation s'arrête, la base doit être rouverte)
- **Atteignable en service** : oui — tout poste dont le tampon du moteur est
  plus petit que ce qu'un point de reprise veut tenir en mémoire
- **Touche rag3weaver** : oui (première indexation d'un dépôt, poussée des
  blobs du plein texte)

## Ce que c'est

Avec un tampon du moteur de 2 Gio, la première indexation du dépôt rag3db
(6 900 fichiers, 62 Mo de texte) meurt vers le 2 000ᵉ fichier : un point de
reprise déclenché pendant la poussée des blobs du plein texte ne trouve plus
de mémoire, échoue, et la base refuse toute requête jusqu'à sa réouverture.

Message exact :

```
index persistence failed: ingest: must reopen: Query execution failed:
Runtime exception: Buffer manager exception: Unable to allocate memory!
The buffer pool is full and no memory could be freed! A checkpoint of this
database failed, so it must be closed and reopened before it is used again:
the checkpoint did not complete.
```

## La recette, sans rag3weaver (4 octobre, 14 h 45)

`extension/rag3weaver/tests/sonde_tampon_et_blobs.rs` : la connexion seule
et du Cypher brut, base sur disque, tampon du moteur à 2 Gio, point de reprise
automatique par défaut (16 Mio).

```cypher
CREATE NODE TABLE Blobs (_key STRING, _data BLOB, _deleted_gen INT64, PRIMARY KEY(_key));
-- répété, par lots de 32 Mo au plus (8 blobs de 4 Mo, octets incompressibles) :
UNWIND $items AS item WITH item, item.key AS k
MERGE (b:Blobs {_key: k}) SET b._data = item.data, b._deleted_gen = -1;
```

| Mode | Tampon | Résultat |
|---|---|---|
| `ajoute` : des clés toujours neuves, rien supprimé | 2 Gio | **échoue après 544 Mo écrits** (génération 8, 55 s), mémoire résidente 2,6 Go |
| `remplace` : chaque génération remplace la précédente, octets vidés deux générations plus tard | 2 Gio | passe 8 Go écrits en 621 s, mais **mémoire résidente 5,0 Go, plus du double du tampon** |

Même message que dans le produit : « Buffer manager exception: Unable to
allocate memory! The buffer pool is full and no memory could be freed! A
checkpoint of this database failed, so it must be closed and reopened ».

Donc :

- **Le défaut est dans le moteur**, sans rien de rag3weaver : une table qui
  ne porte que **544 Mo** de BLOB fait échouer un point de reprise dans un
  tampon de **2 Gio**. Le point de reprise d'une colonne de BLOB demande
  plusieurs fois la taille de la colonne.
- **Le moteur garde de la mémoire hors de son tampon** quand on écrit des
  BLOB : 5 Go de mémoire résidente pour 2 Gio de tampon, en mode
  `remplace`.
- Une seule exécution de chaque. Pas encore isolé : le seuil de 32 Mio des
  lots, la taille des blobs, le point de reprise automatique coupé.

## La recette par le produit


Pas encore de recette en Cypher brut. Par le produit, base sur disque :

```
RAG3DB_BUFFER_POOL_SIZE=2147483648 \
RAG3WEAVER_ESTIMATE_REPO=1 RAG3WEAVER_ESTIMATE_BATCH_FILES=512 \
RAG3WEAVER_ESTIMATE_DB_DIR=~/.cache/rag3weaver-build/base-mesure \
./run_e2e.sh --test e2e_estimate ce_depot_est_cherchable_par_mots
```

Échec après 130 s, entre 2 048 et 3 072 fichiers (4 octobre 2026, moteur
`e3daa2836`). Sans la borne (tampon par défaut du moteur, poste de 121 Go),
la même passe finit en 823 s avec un pic de 13,1 Go de mémoire résidente.

**4 Gio ne suffisent pas non plus** (4 octobre, 14 h, moteur rebâti à
12 h 43, seuil du point de reprise automatique relevé à 512 Mio, poste seul
sous `poste mesure`) : deux passes à 512 sur disque, l'une avec les fusions
lucivy bornées, meurent toutes deux sur le même message entre 4 096 et
5 120 fichiers. La mémoire résidente anonyme y atteint 7 à 8 Go avant
l'échec : au moins 3 Go ne sont donc pas dans le tampon du moteur. Les bases
dans l'état « à rouvrir » sont gardées sous
`~/.cache/rag3weaver-build/base-echouee-mem-tampon4g` et
`base-echouee-mem-les-deux`.

**8 Gio passent ; abaisser le seuil ne sauve pas 4 Gio** (4 octobre,
14 h 20, sous le verrou exclusif) :

| Tampon | Seuil du point de reprise | Résultat |
|---|---|---|
| 8 Gio | 512 Mio | passe, 384 s, pic 13,5 Go |
| 4 Gio | 512 Mio | échoue vers 4 500 fichiers |
| 4 Gio | 64 Mio | échoue **plus tôt**, entre 3 072 et 4 096 fichiers |
| 2 Gio | 16 Mio | échoue vers 2 500 fichiers |

- **Ce n'est pas le journal accumulé** : avec un seuil huit fois plus bas, la
  passe meurt plus tôt, pas plus tard. Ce qui remplit le tampon, ce sont des
  pages que le moteur ne rend pas — à isoler en Cypher brut avant d'accuser
  le moteur.
- **Ce n'est pas la taille de l'instruction** : à 4 Gio, l'instruction qui
  meurt est le COPY de 512 arêtes de `File_CHUNKED_FROM`, la file des liens
  étant vide ; à 2 Gio, c'est une poussée des blobs du plein texte. Les deux
  morts suivent de près une poussée du plein texte (piste de la session de
  l'arbre principal : les blobs lucivy rangés dans la base).
- **L'échec ne laisse aucun dégât durable** (session mémoire, deux bases
  rouvertes) : la réouverture rejoue le journal, lignes et index sont là.

## Le témoin

`extension/rag3weaver/tests/e2e_estimate.rs`,
`ce_depot_est_cherchable_par_mots_avant_ses_vecteurs`, avec les variables
ci-dessus. Ce n'est pas un témoin de batterie : il prend le dépôt entier.
Une seule exécution à ce jour.

## La cause

Pas établie. Ce qu'on sait : l'erreur sort de la persistance de l'index plein
texte (`flush_blob_store`), donc d'un point de reprise posé pendant qu'on
écrit de gros BLOB ; le moteur a son point de reprise automatique actif avec
un seuil de 16 Mio de journal (`database.h`). Ce qu'on ne sait pas : ce que le
point de reprise épingle en mémoire (tout le journal ? les pages des BLOB ?),
ni la plus petite taille de tampon qui passe.

## Reproduit sur master et localisé (11 octobre 2026, seconde session cœur C++)

- **La sonde Rust du ticket, sur la lib commune de luciepc (master `5c5feb527`)** : en mode
  `ajoute` sous 2 Gio, elle échoue toujours après 544 Mo, à la génération 8, au mot près du
  4 octobre. En mode `remplace`, elle passe 3 Go, avec un pic de mémoire résidente de 3,4 Go.
- **Une sonde C++ jetable** (des transactions de 8 blobs de 4 Mo, `MERGE` préparé, la mémoire
  utilisée du tampon relevée ; `~/.cache/rag3db-tickets-notes/blobs_checkpoint_probe_scratch_test.cpp`)
  dit trois choses :
  - **ni fuite ni accumulation** : le tampon tient 38 Mo avant chaque validation et retombe à
    6 Mo après chaque point de reprise. Le point de reprise automatique passe après chaque
    validation (32 Mo de journal), il n'est jamais reporté ;
  - **un CHECKPOINT explicite après chaque validation échoue de même**, à la validation où la
    table franchit 512 Mio ;
  - **le seuil suit le tampon, en puissance de 2** : il est franchi à 256 Mio sous 1 Gio, à
    512 Mio sous 2 Gio, à 1 024 Mio sous 4 Gio. Le point de reprise d'une colonne BLOB demande
    donc environ quatre fois la taille de la colonne, arrondie à la puissance de 2 supérieure.
- **La pile de l'échec** (gdb, `catch throw` sur l'exception du tampon, 1 Gio) :
  `Checkpointer::writeCheckpoint` → `NodeTable::checkpoint` → `NodeGroup::checkpointInMemAndOnDisk`
  → `ColumnChunk::checkpoint` → `StringColumn::checkpointSegment` →
  `Column::checkpointColumnChunkOutOfPlace` → `StringChunkData::write` →
  `DictionaryChunk::appendString` → `ColumnChunkData::resize` → `MemoryManager::allocateBuffer`.
- **Lecture, à éprouver** (repérage par agent, lignes à `c011643d2`) :
  - une insertion de 4 Mo ne tient jamais dans la place restante d'un segment de chaînes, donc
    le segment est toujours réécrit hors place (`DictionaryColumn::canDataCommitInPlace`) ;
  - le segment est relu en entier dans un tampon arrondi par `std::bit_ceil`
    (`DictionaryColumn::scan`) ;
  - chaque ajout qui déborde double le tampon (`DictionaryChunk::appendString`), et
    `ColumnChunkData::resize` alloue le nouveau tampon avant de libérer l'ancien ;
  - puis `split()` recopie le tout, par lots de 64 valeurs, avant le premier `flush`.

  Le seuil mesuré dit que le segment réécrit grandit avec toute la colonne : le découpage par
  segments ne le borne pas pour ces blobs. Pourquoi, c'est à lire, d'abord `shouldSplit`,
  `split` et `isLastSegment`.
- **Ce que ça veut dire pour le produit** : une table de BLOB ne doit pas dépasser environ le
  quart du tampon (moins le reste), sinon son prochain point de reprise échoue, et la base est
  à rouvrir.

## Ce qu'il faut pour le fermer

- Une recette minimale au niveau du moteur (une table à colonne BLOB, des
  `MERGE` de plusieurs dizaines de Mo, un tampon étroit).
- Savoir si c'est le moteur (un point de reprise qui devrait vider par
  morceaux) ou nous (des blobs trop gros en une instruction, à découper).
- Dans rag3weaver, au minimum : dire à l'utilisateur quoi faire (taille du
  tampon, réouverture) au lieu d'un échec d'ingestion nu ; `estimate`
  pourrait prévenir quand le dépôt dépasse ce que le tampon permet.
- Une première indexation de ce dépôt qui passe avec un tampon de 2 Gio.

## Corrigé (`47a283e6c`, `dd4fd0a35`)

- **La cause** : pour découper une colonne en segments, `ColumnChunkData::split()` estime la
  taille d'un segment par `getSizeOnDiskInMemoryStats()`. Pour les chaînes, cette estimation
  lisait les statistiques en mémoire du bloc d'octets, que `DictionaryChunk::appendString` ne tient
  pas à jour (il copie par `memcpy`) : elle disait le bloc constant, donc vide. `split()` gardait
  alors toute la colonne dans un seul segment, et chaque point de reprise le relisait et le
  réécrivait hors place, dans un tampon agrandi par doublements (`bit_ceil`), l'ancien tampon et le
  nouveau coexistant. D'où le seuil : un quart du tampon, en puissance de 2.
- **Le correctif** :
  - l'estimation compte les octets réels des chaînes. Le dictionnaire est brut sur le disque, donc
    c'est leur taille exacte, à la page près ;
  - `split()`, pour les chaînes, retire un lot qui fait déborder le segment et le retente deux fois
    plus petit, jusqu'à une valeur, et `finalize()` rend les octets des valeurs retirées. Un segment
    reste sous la taille visée, sauf un segment d'une seule valeur plus grande qu'elle ;
  - les autres types gardent le découpage d'origine (`dd4fd0a35`). Pour une liste, `truncate()`
    laisse les valeurs retirées dans le bloc fils, et reculer y entassait des valeurs mortes : la
    première version faisait manquer de tampon le point de reprise des FLOAT[960] (trois cas vector,
    vus par la liste complète).
- **Les témoins** :
  - `StringSplitTest` (`test/storage/string_split_test.cpp`) : rouge avant, avec 16 blobs d'1 Mio
    dans un seul segment de 16 Mio ; vert après, avec un segment par blob, aucun segment de
    plusieurs valeurs au-dessus de 256 Kio, et aucun octet perdu ;
  - `BlobCheckpointTest.ATableLargerThanAQuarterOfTheBufferPoolCheckpoints` (112 Mio de BLOB sous
    256 Mio de tampon, un CHECKPOINT après chaque validation) : rouge avant, à 72 Mio ; vert après,
    avec relecture exacte après réouverture.
- **La sonde Rust du ticket**, sur une lib de la branche, 3 Go sous 2 Gio : en `ajoute`, elle
  passe en 65 s (sur master, échec à 544 Mo) ; en `remplace`, elle passe en 70 s (sur master,
  195 s).
- **Pour le produit** : la limite « une table de BLOB d'au plus un quart du tampon » tombe. Le mode
  blobs du plein texte n'est plus le défaut de rag3weaver, mais l'index creux vit encore dans des
  blobs : c'est lui qui en profite d'abord.
- **Ce qui reste** :
  - la première indexation de ce dépôt sous 2 Gio, avec embarquements : une passe de
    confirmation pour l'arbre principal, de jour, en régime doux (orchestration), ticket
    `2026-10-11-confirmer-le-premier-index-sous-un-petit-tampon.md` ;
  - la mémoire résidente au-delà du tampon (3,2 à 3,7 Go de RSS pour 2 Gio de tampon) : ticket à
    part, `2026-10-11-memoire-residente-au-dela-du-tampon-sous-des-blobs.md` ;
  - une colonne LIST ou STRUCT qui contient des chaînes (une liste de BLOB) : `split()` ne voit que
    le type de tête, donc elle garde l'ancien découpage par lots de 64, sans recul. Son estimation
    passe par celle de ses chaînes, qui compte maintenant les vrais octets, mais un segment peut
    dépasser la cible de jusqu'à un lot de 64 valeurs (remarque de la relecture du cœur). Ce n'est
    pas corrigé.

## Fermé sur une preuve plus étroite que le défaut (11 octobre 2026, 10 h 40)

La confirmation de l'arbre principal, sous 2 Gio avec la lib de `4820e9154`, est verte en
fichiers. Mais ce chemin ne met aucun blob en base. **En blobs, elle reste rouge**, au-delà de
3 072 fichiers, avec le même message. Les témoins et la sonde de ce ticket écrivent les blobs ligne
à ligne (CREATE, MERGE). Le produit les écrit par un COPY de `_index_blobs`, en point de reprise
forcé, et ce chemin n'était pas couvert. La fermeture vaut pour les écritures ligne à ligne. Le
COPY continue dans `../2026-10-11-copy-de-blobs-en-point-de-reprise-force-sous-un-petit-tampon.md`.
