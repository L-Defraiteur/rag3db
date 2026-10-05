# Un COPY refusé pour mémoire laisse la table faussée en mémoire, et le repli y écrivait

- **État** : ouvert côté moteur (cœur C++, correctif en cours). Côté rag3weaver, un filet est posé (`99e6f0ee0`), **à garder** tant qu'une preuve ne dit pas qu'il est inutile.
- **Gravité** : réponse fausse (doublons de clé, lignes introuvables par leur clé) ; plantage possible au point de reprise de fermeture.
- **Atteignable en service** : oui, sur le chemin par défaut (hors transaction par paquet), dès que le tampon du moteur ne tient pas un COPY.
- **Touche rag3weaver** : oui.

## Ce que c'est

Un COPY refusé par « Buffer manager exception: Unable to allocate memory! The buffer pool is
full and no memory could be freed! » **après** avoir réservé ses lignes ne fait reculer ni le
compte de la table, ni son curseur de réservation, et aucune annulation ne les remet, même
explicite. Dans la même session, une ligne créée ensuite n'est plus trouvée par sa clé, et
une requête suivante peut réserver 4 Gio en une demi-seconde. Un point de reprise dans cet
état, y compris celui de la fermeture, enfle sans fin dans l'index de clé primaire
(`HashIndex::splitSlots`). Après une réouverture, tout est juste (cœur C++).

Côté rag3weaver, hors transaction, le COPY refusé se repliait sur MERGE dans la **même**
session (`REPLI_EN_MASSE`, `copier_les_noeuds` et `copier_les_liens`). Ce MERGE cherchait
ses clés dans la table faussée et pouvait recréer une ligne existante. Sous transaction par
paquet, le paquet était défait, puis la base était empoisonnée : on n'y écrivait plus, mais
le point de reprise de fermeture passait encore sur la table faussée.

## Le filet côté rag3weaver

- `connection::BUFFER_POOL_FULL` (« The buffer pool is full ») est reconnu dans
  `Rag3dbConnection::engine_error`, à côté de `REOPEN_AFTER_FAILED_CHECKPOINT` : la base
  est empoisonnée et l'appelant reçoit `MustReopen`. Le message dit la cause, le tampon
  en place, et le réglage à changer : `RAG3DB_BUFFER_POOL_SIZE` ou `buffer_pool` au
  manifeste ; la règle par défaut est la moitié de la mémoire vive, au plus 8 Gio.
- Les deux replis `REPLI_EN_MASSE` ne se replient plus sur une base à rouvrir : le nœud
  échoue avec la cause.
- La base se ferme **sans point de reprise** (`CALL force_checkpoint_on_close=false`,
  posé au refus puis à la dernière connexion, après un ROLLBACK). La réouverture repart du
  disque et du journal, et elle est juste. Ce qui n'était pas validé est perdu, comme il
  se doit.
- Témoin : `extension/rag3weaver/tests/e2e_tampon_plein.rs`. Avec un tampon de 32 Mio et un
  COPY de 20 000 lignes (~80 Mo), on obtient le refus nommé, puis une session qui refuse
  tout. Rouverte avec le tampon par défaut, la base rend 20 000 lignes et 20 000 clés
  distinctes.

## Le refus d'avant l'écriture ne couvre pas ce cas

`Estimate::buffer_refusal` (`src/estimate.rs`) refuse d'indexer quand le tampon paraît trop
petit, mais seulement depuis l'outil `index` de l'agent (`src/dataflow/index_nodes.rs`).
`sync_source`, l'API du catalogue (`ingest_*`, `drain`) et `CodeIngestNode` ne
l'appellent pas. Sa borne, 8 Mo de texte par Gio de tampon, est calée sur deux mesures du
point de reprise qui échoue, pas sur le COPY.

**Où il devrait vivre** : au point où une écriture en masse se décide, c'est-à-dire là où
un lot part par COPY (`copier_les_noeuds` et `copier_les_liens`, ou leur appelant
`InsertRecordNode`/`LinkRecordNode`). Le volume du lot y est connu (la taille du fichier
CSV), et le tampon aussi (`DbConnection::buffer_pool`). Un lot plus gros que ce que le
tampon tient se découperait en lots plus petits, ou se refuserait avant d'écrire. En
complément, il faudrait aussi un contrôle en tête de `sync_source` sur le volume total,
pour dire le réglage avant de commencer, comme le fait l'outil `index`. Rien n'est codé :
la borne demande d'abord une mesure sur le COPY lui-même.

## Pour le fermer

Le correctif du moteur, où l'annulation remet le compte, le curseur et l'index de clé
primaire, et ce même témoin vert sans le filet. Le filet ne se retire que sur cette preuve.
