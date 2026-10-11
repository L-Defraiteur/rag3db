# Le COPY de blobs en point de reprise forcé fait échouer la validation sous un petit tampon

- **État** : ouvert (11 octobre 2026) — lot suivant de la seconde session cœur C++ (orchestration)
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

## La cause

Pas établie. Lecture (seconde session cœur C++, à éprouver par sonde) : un COPY construit ses
groupes de lignes entiers en mémoire avant de les écrire. Le vidage d'un groupe
(`InMemChunkedNodeGroup::flush` → `finalize()` → `split(true)`, `chunked_node_group.cpp`) tient
tout le groupe plus sa copie découpée. Pour des blobs de quelques Mio, un groupe de quelques
centaines de lignes pèse déjà des centaines de Mo, et le double au vidage. Le correctif du ticket
fermé borne les segments, mais pas la mémoire d'un groupe que le COPY tient entier.

## Témoin

À écrire avec la recette sans harnais, rouge d'abord. Puis la confirmation de l'arbre principal
rejouée en blobs.

## Pour le fermer

Un correctif générique : un groupe de blobs qui se vide par morceaux bornés, pas une rustine sur
`_index_blobs` (orchestration). Relecture de la session cœur C++, témoin rouge puis vert, et la
passe en blobs de la confirmation, verte sous 2 Gio.
