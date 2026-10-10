# Le Cypher au-dessus des dialectes — l'inventaire, et un langage intermédiaire

10 octobre 2026, session embarquements, chantier F. Pour que Lucie décide :
« j'aimerais qu'on évite complètement le Cypher au-dessus des dialectes, qu'on
ait notre langage intermédiaire s'il le faut ». Pas de code. Lu sur
`2e67ce3e0` (agent de lecture ; trois sites vérifiés à la main :
`code_sync.rs:265`, `graph_walk.rs:118`, `catalog.rs:2069`). Suite de
[01-le-contrat-du-dialecte.md](01-le-contrat-du-dialecte.md).

## En une phrase

**42 endroits** du crate envoient du Cypher sans passer par le dialecte. Une
vingtaine sont les nœuds du graphe de code, et presque tous sont **un seul
motif** : « depuis une liste d'uuids, un saut par une relation, dans un sens,
rendre des champs ». Le langage intermédiaire n'a donc pas besoin d'être un
langage. **Cinq formes** suffisent pour faire disparaître la famille (a).

## L'inventaire, en trois familles

Rien ne regarde `speaks_cypher` avant ces appels : sur PostgreSQL, chacun
**échoue**, ou rend vide quand l'erreur est avalée.

### (a) Du Cypher qui devrait passer par le dialecte — 42 sites

| Groupe | Sites | Lecture / écriture | Ce que c'est |
|---|---|---|---|
| **Nœuds du graphe de code** | `graph_walk.rs` (degré, voisins), `neighborhood_nodes.rs` (4), `links_nodes.rs`, `usage_nodes.rs` (6), `search_nodes.rs` (`fetch_related`), `react_nodes.rs`, `code.rs` (2), `code_tools.rs` (`voisins_des_scopes`) | lecture | un saut depuis des uuids ; le degré par uuid ; des fiches par uuid ; des déclarations par `CONTAINS` |
| **Synchronisation du code** | `code_sync.rs` : oublier les arêtes d'un fichier (2), « source déjà indexée », `BEGIN` / `COMMIT` / `ROLLBACK` | écriture, lecture, transaction | supprimer des arêtes par un prédicat sur le nœud ; un existe-t-il ; la transaction (page 01, §4.1) |
| **Replis de `search.rs` sans moteur de recherche** | `search_vector_hnsw`, `…_filtered`, `enrich_results_with_data`, `resolve_and_enrich`, `search_vector_bruteforce` (code mort), fragments de motifs (tests seulement) | lecture | atteints seulement quand aucun `SearchBackend` n'est monté |
| **Le reste, un par un** | `catalog.rs:2069` (compte après rebâti vectoriel, erreur avalée), `composable_results.rs` (sélection filtrée et compte), `trace_nodes.rs`, `node_registry.rs` (valeurs d'énumération, erreur avalée), `ref_nodes.rs`, `backend.rs` (journal de conversation ; `LOAD EXTENSION`), `query.rs` (constructeur public, aucun appelant), `relation_directions.rs` (exemples et tests), `dataflow/record.rs` (enregistreur public, aucun appelant), l'annulation de `CypherNode` | surtout lecture | chacun tient dans une méthode existante ou une petite nouvelle |

### (b) Les implémentations rag3db elles-mêmes — légitimes

`rag3db_connection.rs`, `rag3db_search_backend.rs`, `cypher_blob_store.rs`, le
`CypherCheckpointStore`, et les générateurs de `schema.rs` (tests seulement).
Une soixantaine de requêtes. Elles sont **sous** le dialecte, c'est leur place.

### (c) Ce qui parle la langue de la base par nature — 6 sites

`CypherNode` et `ValidateNode` des migrations (requêtes fournies par
l'utilisateur), la requête `cypher()` du démon, `Catalog::execute_raw` et
`execute_raw_with_params`.

`execute_raw` est la porte par laquelle passe **presque toute la famille
(a)**. Il faut la garder pour (c), mais la refuser par son nom à un dialecte
qui ne déclare pas `cypher`. Ce refus, c'est le test 14 de la batterie, écrit
dans le code.

## Ce que les 99 méthodes couvrent déjà

Le vocabulaire existe, éparpillé en méthodes à usage unique :

| Forme | Déjà là | Manque |
|---|---|---|
| compter les lignes d'une table | `count_rows` | un compte avec prédicat |
| lire par uuids, par champ, tout | `select_by_uuids`, `batch_select`, `select_by_field`, `select_all` | une limite ; un OU sur deux champs ; `CONTAINS` en instruction entière |
| lire avec filtre, tri, page | `select_page_after_offset`, `filter_*` (fragments) | **la forme entière** : prédicat + tri + limite |
| un saut par une relation | `join_select` (une clé), `kb_gather_content` (liste d'uuids, sans propriétés d'arête) | **la forme générale** (ci-dessous) |
| degré par uuid | `count_relations_of` (total, `None` sur PostgreSQL) | par uuid et par sens |
| écrire par lot, lier, supprimer | `batch_upsert`, `batch_link*`, `batch_delete*`, `batch_update_fields` | supprimer des arêtes par un prédicat sur leurs nœuds |
| recherche vectorielle | `SearchBackend` | rien : les replis de `search.rs` doivent passer par le moteur |
| transaction | `capabilities().transactions` | `begin`, `commit`, `rollback` sur la connexion |

## Le langage intermédiaire minimal

Pas un langage de requêtes : **cinq structures de données** que chaque
dialecte traduit, à la place d'une trentaine de chaînes Cypher.

1. **`Select`** : une table, des champs, un prédicat, un tri, une limite.
   Le prédicat est l'arbre de filtres qui existe déjà (`FilterCondition`,
   compilé par dialecte dans `filter.rs`), étendu de `OR` et `CONTAINS` si
   besoin. Couvre les lectures par champ, par uuids, filtrées et paginées.
2. **`Count`** : `Select` sans champs, groupé ou non par une clé.
3. **`Hop`** : depuis une liste d'uuids, une relation, un sens ; les champs
   du voisin, les propriétés de l'arête, un filtre d'arête optionnel, une
   limite par source ; rend toujours l'uuid de départ. Couvre à lui seul
   graph_walk, neighborhood, links, usage, fetch_related, react et les
   voisins de `code_tools`. Le degré, c'est `Count` sur `Hop`.
4. **`Write`** : les lots qui existent déjà (upsert, lien, mise à jour,
   suppression), plus « supprimer les arêtes de R qui touchent les nœuds où
   P ».
5. **`Tx`** : début, validation, annulation, tenus sur une seule session.

Les chemins à plusieurs sauts (`Moteur::aussi` de neighborhood) sont une
**suite de `Hop`**, pas une forme de plus. Le dialecte rag3db peut les
fondre en un seul `MATCH` s'il y gagne : l'optimisation reste sous le
dialecte, invisible au-dessus.

Ce que l'IR **ne fait pas** : il ne remplace pas (c), il ne remplace pas la
recherche (vecteur et plein texte restent au `SearchBackend`), et il ne
cherche pas à tout dire de Cypher. Une forme nouvelle s'ajoute quand un nœud
en a besoin, comme une méthode aujourd'hui.

## Ce que cela coûte, et dans quel ordre

1. **La porte unique** (voie du milieu, déjà prise) : `execute_raw` et les
   corps par défaut refusent par leur nom sans `cypher`. Petit et sans
   risque sur rag3db. Cela rend le trou **visible** sur PostgreSQL.
2. **`Hop` et `Count` d'abord** : ils vident la vingtaine de sites du graphe
   de code. C'est le gros du gain, et les tests du graphe de code
   (`e2e_code`, `usages`) disent si la traduction rag3db reste identique.
3. **`Select`** ensuite, pour les sites un par un et `composable_results`.
4. Les replis de `search.rs` **passent par le `SearchBackend`** ; le code mort
   (`search_vector_bruteforce`) et les API publiques sans appelant
   (`query.rs`, `dataflow/record.rs`) sont **à retirer ou à garder**, au
   choix de Lucie.
5. Les méthodes à usage unique du trait se réécrivent **sur** l'IR quand on
   y passe, sans grand soir.

Parité exigée à chaque pas : sur rag3db, la requête produite par l'IR doit
rendre les **mêmes lignes** que le Cypher qu'elle remplace (comparaison champ
à champ sur le dépôt indexé, pas un comptage).

## Pour Lucie

- **L'IR en cinq formes** (`Select`, `Count`, `Hop`, `Write`, `Tx`) plutôt
  qu'un langage de requêtes complet : oui ou non ?
- **`query.rs` et `dataflow/record.rs`**, publics et sans appelant dans le
  crate : à retirer, ou des utilisateurs dehors s'en servent ?
- **Ordre** : la porte unique d'abord (petit), puis `Hop` et `Count` (le
  gros) ?

## Décidé par Lucie (10 octobre)

- **Oui à l'IR en cinq formes**, « tant qu'après c'est scalable » : une
  forme de plus s'ajoute quand un nœud en a besoin, et l'optimisation (fondre
  une suite de `Hop`) reste sous le dialecte.
- **`query.rs` : à retirer** (personne ne l'instancie, c'est un ancêtre de
  `Select`). **`dataflow/record.rs` : à garder.** Il est exercé par
  `e2e_dataflow_observe` (puits fichier et base). C'est aussi la seule pièce
  qui écrit une exécution dans la base, ce dont les visions ont besoin. Il
  passera par `Write`, pas avant. « Aucun appelant » plus haut voulait dire
  aucun appelant dans `src/` : les tests l'appellent.
- **Oui à l'ordre** : la porte unique (faite, `752e858b4`), puis `Hop` et
  `Count`, puis `Select` site par site, avec une parité ligne à ligne à
  chaque pas.
- **Une crate séparée, pas un dépôt** : `rag3weaver-ir` (`41b869ba4`), sans
  mot de base dans ses types (`Value`, et non plus `CypherValue`).

## Où on en est

| Pas | Commit | Parité |
|---|---|---|
| `Hop` dans la crate, traduit par rag3db, refusé en le nommant ailleurs | `ba11e003b` | tests unitaires de la traduction |
| `graph_walk::neighbors` passe par `Hop` | `389f5a9dd` | requête identique au caractère près (test) ; e2e_code 27/27, e2e_usages 7/7, usages_rendu 6/6 (luciepc, lib d’avant 0aed3c4b5, sans effet sur ce changement) |
