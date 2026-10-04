# Un COPY de relations vers des nœuds locaux garde leurs décalages provisoires

- **État** : ouvert — sans effet avec un seul écrivain ; témoin attendu de la marche A3′ (écritures parallèles)
- **Gravité** : réponse fausse (relations attachées aux nœuds d'une autre transaction) — en mode multi-écrivains seulement
- **Atteignable en service** : non (le mode multi-écrivains est éteint hors du banc)
- **Touche rag3weaver** : non aujourd'hui ; oui quand les écritures parallèles seront allumées (un lot de nœuds par `MERGE` puis des relations par `COPY` dans la même transaction est sa forme courante)
- **Ouvert le** : 5 octobre 2026, session cœur C++ (lecture, en préparant le versement des lignes locales avant un `COPY`)
- **Pour** : cœur C++ (marche A3′)

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
