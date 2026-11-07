# Un COPY de relations vers des nœuds locaux garde leurs décalages provisoires

- **État** : corrigé (7 novembre 2026, seconde session cœur C++, lot G1) : un COPY de relations verse
  d'abord dans leurs tables les nœuds locaux de ses bouts (`IndexLookup::initGlobalStateInternal` →
  `LocalStorage::flushNodeTable`), comme un COPY de nœuds, avant toute résolution de clé. Témoins :
  `ProvisionalOffsets.*` au banc (`test/transaction/concurrence/provisional_offsets_test.cpp`), rouges
  avant, verts après, dans les deux ordres.
- **Gravité** : réponse fausse en silence : les relations d'un COPY relient les nœuds d'une autre
  transaction.
- **Atteignable en service** : oui, depuis que le mode multi-écrivains est le défaut (11 octobre 2026) :
  deux transactions écrivaines sur la même table de nœuds, la seconde validée entre la première
  écriture de la première dans cette table et sa validation, et la première pose par COPY des
  relations vers ses propres nœuds.
- **Touche rag3weaver** : oui, par la forme de ses drains (des nœuds par `MERGE`, puis au-delà de 200 arêtes des relations par `COPY`, dans la même transaction), si un second écrivain valide sur la même table pendant ce temps ; par le code, l'hôte écrit une base par une seule connexion sous `Mutex<Catalog>`, à confirmer par le produit
- **Ouvert le** : 5 octobre 2026, session cœur C++ (lecture, en préparant le versement des lignes locales avant un `COPY`)
- **Pour** : cœur C++ (marche A3′)
- **Famille** : moteur

## Le témoin rouge (7 novembre 2026)

`ProvisionalOffsets.ACopyOfRelsToLocalNodesKeepsItsEndpointsAfterAnotherWriterCommits` : A crée dix
nœuds (1000 à 1009) dans sa transaction, B valide cinq nœuds (2000 à 2004) de la même table, A relie
ses nœuds par un COPY (1000 → 1001, …, 1008 → 1009) et valide. Sur master a91f066ea et suivants,
**la relation 1000 → 1001 se lit 2000 → 2001** : les relations du COPY désignent les nœuds de B. Le
second témoin, `…WhenAnotherWriterCommitsAfterIt`, joue l'autre ordre (B valide après le COPY, avant
la validation de A), par le même défaut.

## Ce que c'est

Dans une transaction, des nœuds créés par le chemin ordinaire vivent dans le stockage local, à des décalages provisoires qui commencent au nombre de lignes de la table. Un `COPY` de relations de la même transaction résout ses extrémités par la clé (`IndexLookup`, `src/processor/operator/index_lookup.cpp:101`, qui passe par `NodeTable::lookupPK` et donc par le stockage local) : il obtient ces décalages provisoires, les écrit directement dans la table de relations, et les journalise (`Partitioner::logRelsToWAL`, `src/processor/operator/partitioner.cpp`).

Au commit, seules les relations **locales** sont remappées vers les décalages définitifs des nœuds (`LocalRelTable::remapNodeOffsets`, appelé par `LocalStorage::commit`). Celles d'un `COPY` ne le sont pas.

Avec un seul écrivain, provisoire et définitif sont égaux : rien n'est faux, et le témoin de l'étape 3 du chargement journalisé le montre (`JournaledCopyTest`, des nœuds par `MERGE` puis des relations par `COPY` vers eux). Avec plusieurs écrivains, si une autre transaction valide des nœuds dans la même table entre la création et le commit, les nœuds atterrissent plus loin et les relations du `COPY` désignent les nœuds de l'autre.

## Recette minimale

Aucune : établi par lecture, non exécuté. Il faut le mode multi-écrivains et un entrelacement — écrivain A : `BEGIN`, crée des nœuds, `COPY` de relations vers eux ; écrivain B : crée et valide des nœuds dans la même table ; A valide.

Le versement des lignes locales avant un `COPY` de **nœuds** (5 octobre) appelle le même remappage des relations locales qu'au commit ; lui non plus n'est pas prouvé sous concurrence, et c'est écrit dans le code (`LocalStorage::flushNodeTable`).

## Témoin

Aucun. À écrire au banc de concurrence avec la marche A3′ : l'entrelacement ci-dessus, puis chaque relation relue avec les clés de ses deux extrémités.

## Pour le fermer

Au choix de la marche A3′ : verser les nœuds locaux des tables visées avant un `COPY` de relations (ils prennent leurs décalages définitifs sous le verrou de la table), ou remapper au commit les relations écrites par un `COPY`. Le témoin ci-dessus vert.
