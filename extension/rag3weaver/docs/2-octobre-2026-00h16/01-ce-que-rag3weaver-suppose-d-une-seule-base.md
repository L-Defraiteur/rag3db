# Ce que rag3weaver suppose d'une seule base

**2 octobre 2026, 00 h 16.** La moitié rag3weaver de la question des
écritures parallèles. Lucie a décidé le 2 octobre d'aller « jusque B »
(plusieurs processus écrivains), et proposé une forme : ne pas tout garder
dans un seul fichier, avec une découpe par genre d'index plutôt que par
source. La moitié moteur est étudiée par la session cœur C++ (rapport à venir
dans `docs/` à la racine). Ici : ce que le crate fait aujourd'hui, et ce que
chaque forme lui demanderait.

**Statut des affirmations.** L'état des lieux (§1) vient d'un agent de
lecture, sur `master` à `5b47a836c` ; je n'ai revérifié moi-même que le
gabarit `search_base.mmd` et `FuseResultsNode`. Les références `fichier:ligne`
sont celles de cet arbre. Rien n'a été modifié ni exécuté. Ce qui n'a pas été
vérifié est listé au §4.

## En dix lignes

- Un `Catalog` = une connexion = une base, en dur.
- **Les index plein texte et sparse sont rangés dans la base elle-même**
  (table `_index_blobs`) : aujourd'hui, écrire un index, c'est écrire dans la
  base, donc tenir sa place d'écrivain.
- **Ces index adressent les lignes par leur position interne dans la base**,
  pas par uuid : un index n'a de sens qu'avec sa base.
- L'index vectoriel est créé dans la base (`CREATE_VECTOR_INDEX`).
- Les relations relient deux tables de la même base, dans une seule requête.
- Deux choses sont déjà prêtes pour plusieurs fichiers : **l'uuid ne dépend
  que du nom d'entité et du contenu**, et **la recherche sait déjà chercher
  en éventail puis fondre par uuid** (c'est ce qu'elle fait entre cellules).
- La découpe **par genre d'index** demande moins au crate que la découpe par
  source : les lignes et les relations restent dans un seul graphe, seul le
  rangement des index change. Son point dur est l'adressage par position.

## 1. L'état des lieux

### Une base par catalogue

`Catalog` porte un seul jeu de champs (`src/catalog.rs:114-247`) : `conn`,
`sync_conn`, `dialect`, `search_backend`, `blob_store`, `checkpoint_store`,
`node_id_cache`, `cache_base`, et une seule `config`. Il ne connaît jamais le
chemin de la base : `Catalog::new(conn, embedder, config)` (`:287`) reçoit une
connexion déjà ouverte. Tout `initialize` se dérive de `self.conn`
(`:866-1010`).

Deux défauts sont partagés entre bases, hors base : le cache sous
`$TMP/rag3weaver_cache` (`catalog.rs:341`) et le dossier de checkpoints
`$TMP/rag3weaver-checkpoints` (`dataflow/checkpoint_store.rs:50`). Les noms
d'index ne distinguent que la cellule, pas la base (`scope.rs:124-135`).

### Les cellules : le précédent le plus proche

`scope::Scope` (`src/scope.rs:72`) est la cellule `(org, project)` : une
partition **à l'intérieur** d'une base. Les tables et l'index vectoriel sont
partagés (colonnes `_org` / `_project`) ; **les index plein texte et sparse
sont séparés, un par cellule** (nom suffixé, `scope.rs:120-135`). Une
recherche peut viser plusieurs cellules : chacune est cherchée tour à tour
(`catalog.rs:6709-6752`), puis le tout est fondu par RRF sur l'uuid
(`fusionner_par_cellule`, `:7038`). C'est le modèle direct d'une recherche
sur plusieurs index.

### Le domaine de travail

`WorkDomain` (`src/work_domain.rs:102`) est une sélection de sources, dépôts,
chemins et langages. Il devient un filtre, résolu soit en positions lucivy
(`allowed_ids_for`, `generic_search_nodes.rs:1878`), soit en jointure SQL.
Il est déjà multi-dépôts, mais il filtre une seule base. Aucun appelant de
production ne le pose (seuls les tests).

### Le démon et les lecteurs

Un démon tient une base (`src/bin/rag3daemon.rs:63-77`,
`src/daemon/db.rs:151-168`). Côté client, `acces::ouvrir_lecteur(base,
Acces::{Direct, Demon, Auto}, serveur)` (`src/acces.rs:73-124`) choisit entre
ouverture directe en lecture seule et passage par le démon. La lecture seule
a son budget de 250 ms et sa boucle de reprise
(`rag3db_connection.rs:80-111`).

Les commentaires se contredisent sur « un lecteur direct pendant qu'un
écrivain travaille » (`rag3db_connection.rs:46-48` et `:400-404` contre
`acces.rs:5-7`), et le test qui le garderait est `#[ignore]`
(`tests/e2e_prise_atomique.rs:331`). Avec la course trouvée le 2 octobre côté
moteur (journal des chantiers, §6), **le chemin sûr reste : écrivain = démon,
lecteurs = clients du démon**.

### Les liens et l'uuid

Une relation exige ses deux entités dans la même config
(`catalog.rs:1600-1605`) et s'insère par un `MATCH` des deux bouts sur l'uuid,
dans une requête (`dialect.rs:917-920`).

L'uuid `hashsafe` est un blake3 de `"{entité}:{valeurs}"` (`src/uuid.rs:35-39`,
`catalog.rs:1226-1241`) ; ceux des chunks dérivent de celui du parent
(`uuid.rs:46-50`). **Il ne dépend ni de la base ni de la cellule.**

### La recherche

`SearchSourceNode` résout une seule cible. Le port `signals` de
`FuseResultsNode` fond déjà plusieurs listes venues d'entités différentes
(`templates/tools/search_related_scoped.mmd`). Les services du graphe ont
des noms fixes (`"catalog"`, `"conn"`, `"fts_handles"`, `"sparse_handles"`) :
un catalogue par graphe. `UnifiedResult` est clé par uuid et entité, sans
champ de provenance (`dataflow/resultat.rs:24-28`).

### Où vivent les index

- **Plein texte (lucivy) et sparse** : dans la base, table `_index_blobs`,
  clé `{index}/{fichier}` (`src/cypher_blob_store.rs:1-27`). Le cache mmap
  local est jetable. Il existe un mode non défaut, `FtsStorage::LocalFs
  { base_path }`, qui garde une copie durable dans un dossier
  (`fts_handle.rs:260-270`).
- **Les documents y sont indexés par position interne** (`table_id:offset`,
  via `NodeIdCache`, `record_nodes.rs:405-450`).
- **Vecteurs** : colonnes des tables de chunks, index HNSW créé dans la base
  (`dialect.rs:806`).
- Aussi dans la base : les checkpoints, la méta du catalogue, la marque de
  travail en attente.

### Le backend déclaratif

Un `database` par manifeste (`src/backend.rs:31`), un `Catalog` par backend,
ouvert en écrivain exclusif (`src/bin/rag3weaver-backend.rs:22-27`).

## 2. Ce que chaque forme demanderait au crate

### La découpe par genre d'index (la piste de Lucie)

Le graphe — lignes et relations — reste dans un fichier, avec son écrivain.
Chaque index dérivé a son rangement et son écrivain : plein texte, sparse,
vecteurs.

Ce qui joue pour elle :

- **Les index sont déjà des données dérivées dont le retard est une dette**
  retrouvée par requête (disponibilités `DONNEE`, `PLEIN_TEXTE`, `SPARSE`,
  `DENSE`). Un index en retard sur les lignes est un état que le crate sait
  déjà dire et rattraper ; l'atomicité entre le graphe et un index n'est pas
  requise.
- Les relations, les requêtes de graphe et l'undo ne changent pas.
- Le calcul lourd (vecteurs, plein texte) cesse d'occuper la place
  d'écrivain du graphe.

Ce qu'elle demande :

1. **Sortir les blobs d'index de la base.** `FtsStorage::LocalFs` est un
   début pour le plein texte ; il faut le rendre durable et premier, et
   donner l'équivalent au sparse.
2. **Adresser par uuid, ou garantir la stabilité des positions.** Tant que
   l'index désigne une ligne par `table_id:offset`, il dépend de l'histoire
   d'écriture de la base. C'est le point dur, et il touche le filtrage
   (`allowed_ids_for` résout un filtre en positions).
3. **Un écrivain d'index qui lit la base sans la tenir.** Il lui faut une
   lecture cohérente pendant que l'écrivain du graphe travaille — exactement
   ce que la course du lecteur interdit aujourd'hui en ouverture directe ;
   par le démon, c'est possible dès maintenant.
4. **Les vecteurs** : question moteur (l'index HNSW peut-il vivre dans un
   autre fichier que sa table ; que demande la descente de prédicat) — posée
   à la session cœur C++.

Ce qu'elle ne donne pas : deux écrivains de **lignes** en parallèle. Le
fichier du graphe garde un écrivain unique.

### La découpe par source (une base par dépôt)

Elle donne des écrivains de lignes en parallèle, un par source. Elle demande
davantage : un jeu de handles et un `NodeIdCache` par base, un registre de
services qui sait nommer plusieurs catalogues, la provenance dans
`UnifiedResult`, un filtre résolu base par base, la vérification de
l'identité du démon — et surtout **les liens entre bases n'ont aucun
support** (ni relation, ni expansion par `FetchRelatedNode`).

### Plusieurs écrivains sur un même fichier (A puis B, côté moteur)

Rien à changer dans la forme du crate ; ce qui change est qu'une écriture
peut échouer sur un conflit et doit être rejouée. Le drain travaille déjà par
lots bornés et rejouables, et `embarquer_le_retard` réclame déjà son travail
entre processus.

## 3. Trois défauts possibles vus en passant

À vérifier avant d'en faire des bugs :

1. **Un client peut s'attacher au démon d'une autre base** : la sonde ne
   vérifie que le nom du service (`daemon/mod.rs:276-281`) et
   `DaemonConnection::depuis` ne compare pas `identite.base` au chemin
   attendu (`daemon/db.rs:296-301`). `Identite.base` existe, la vérification
   est courte à ajouter.
2. **Le même contenu dans deux cellules a le même uuid** (l'uuid est calculé
   avant l'estampille de cellule, `catalog.rs:5257` puis `:5270`).
3. **Deux bases sur le même poste partagent le cache et le dossier de
   checkpoints par défaut**, avec des noms d'index qui ne portent pas la
   base.

## 4. Non vérifié

- L'emplacement physique de l'index HNSW dans le cœur C++.
- Les chemins de cache de `sparse_vector::SparseHandle` et de lucivy, donc la
  réalité du risque n° 3.
- `postgres_blob_store.rs` en détail.
- La collision d'uuid entre cellules, en base.
- L'absence d'appelant de production du service `"work_domain"` hors du
  crate.
