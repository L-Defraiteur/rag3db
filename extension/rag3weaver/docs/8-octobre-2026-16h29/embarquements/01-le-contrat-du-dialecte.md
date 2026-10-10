# Le contrat du dialecte — la page, avant le code

10 octobre 2026, session embarquements, chantier F du plan de reprise. Rien
n'est codé. Lu sur `27eeb6a7f` : `src/dialect.rs`, `src/connection.rs`,
`src/search_backend.rs`, `src/postgres_*.rs`, et les endroits qui tranchent
selon le dialecte. Ce qui est **vérifié** est marqué ainsi ; le reste est une
lecture, à confirmer par la batterie (§5).

## 1. Ce qu'est un dialecte aujourd'hui

Un dialecte, ce sont **cinq pièces** posées à la main sur le `Catalog` avant
`initialize()` :

| Pièce | Rôle | rag3db | PostgreSQL |
|---|---|---|---|
| `SchemaDialect` | **écrit** les requêtes (ne les exécute jamais) | `Rag3dbDialect` | `PostgresDialect` |
| `DbConnection` | exécute, de façon synchrone | `Rag3dbConnection` | `PostgresConnection` (pool, `block_on`) |
| `SearchBackend` | vecteurs, et plein texte s'il le sert | `Rag3dbSearchBackend` | `PostgresSearchBackend` (pg_trgm, pgvector) |
| `BlobStore` | fichiers d'index lucivy | `CypherBlobStore` en tampon | `PostgresBlobStore` (`bytea`) |
| `CheckpointStore` | reprise des graphes | `CypherCheckpointStore` | `PostgresCheckpointStore` |

Aucun manifeste ni aucune variable ne choisit le dialecte. Seul le code le
fait, par `set_dialect` et ses voisins. Les binaires livrés sont tous en
rag3db, et le seul montage PostgreSQL complet est celui de
`tests/e2e_postgres.rs`.

## 2. Ce qu'un `SchemaDialect` doit fournir

Le trait compte 99 méthodes, dont environ la moitié sans corps par
défaut, rangées en sept familles :

1. **Identité** : nom, schéma interne, instructions de mise en place.
2. **Schéma** : types, tables de nœuds et de relations, colonnes ajoutées,
   index secondaires, tables internes (méta, blobs, mise de côté).
3. **Écritures par lot** : upsert, suppression, liens, mises à jour, marqueurs
   du cycle de vie ; un lot arrive toujours en `$items`.
4. **Lectures** : par uuid, par champ, par page, jointures, comptes, dette
   d'embarquement, résolution chunk → parent.
5. **Chargement en masse** : `copy_nodes_from_csv`, `copy_links_from_csv`,
   `supports_copy_from`.
6. **Vecteurs et embarquement** : index HNSW, écriture et relecture des
   vecteurs, empreintes.
7. **Filtres** : jointure, comparaisons, listes.

**Le piège.** Une quarantaine de méthodes ont un corps par défaut **écrit en
Cypher**. Un dialecte nouveau qui en oublie une envoie du Cypher à sa base
sans que rien ne le dise. PostgreSQL en a déjà un cas :
`select_page_after_offset`, le rebâti du plein texte en fond. Ce rebâti est
neuf (5 octobre) et `PostgresDialect` ne le redéfinit pas (**vérifié**).
Avec PostgreSQL et lucivy comme moteur de texte, il enverrait
`MATCH (n:T) WHERE OFFSET(id(n)) > $apres …` à PostgreSQL.

## 3. Ce que chaque dialecte déclare savoir faire

Rien n'est déclaré d'un seul tenant. Les capacités sont éparses :

- deux booléens du dialecte : `speaks_cypher`, `supports_copy_from` ;
- deux booléens du moteur de recherche : `sert_le_plein_texte`,
  `honore_le_filtre` ;
- des `Option` à `None` qui veulent dire « je ne sais pas » ;
- deux tests sur le nom, `name() != "rag3db"` : filtres imbriqués, et champs
  liste ou structure.

État réel :

| Capacité | rag3db | PostgreSQL | Où c'est dit |
|---|---|---|---|
| Transactions explicites (paquet) | oui | **non** — voir §4 | nulle part : `code_sync.rs` envoie `BEGIN TRANSACTION` / `COMMIT` en texte brut |
| Chargement en masse | COPY CSV, nœuds et liens | non : ligne à ligne | `supports_copy_from`, `copy_*_from_csv` → `None` |
| Plein texte | lucivy (en base ou en fichiers) | natif pg_trgm, ou lucivy | `sert_le_plein_texte`, `MoteurTexte` |
| Vecteurs : index et recherche | HNSW | pgvector, l'index n'est pas nommé | `create_vector_index`, `vector_search` |
| Vecteurs : relire | oui | **non** : un vecteur se relit `Null` | `select_chunk_vectors` → `None` |
| Filtre appliqué par la base | oui | oui | `honore_le_filtre` |
| Filtres sur listes et structures | oui | refusés | `name()` |
| Compter les relations d'une entité | oui | non | `count_relations_of` → `None` |
| Blobs d'index | `CypherBlobStore` | `PostgresBlobStore`, obligatoire | `speaks_cypher` |
| Magasin de reprise | oui | oui | `nouveau_magasin_de_checkpoints` |

## 4. Ce que PostgreSQL montre comme trous

Par ordre de gravité :

1. **Une transaction de paquet n'est pas une transaction.** **Vérifié** :
   chaque `execute` de `PostgresConnection` prend une connexion du pool
   (`pool.get()`, lignes 279 et 314). Le `BEGIN` et le `COMMIT` de
   `code_sync.rs` (912 et 935) peuvent donc partir sur deux sessions
   différentes. Aujourd'hui, c'est derrière `RAG3WEAVER_TX_PAR_PAQUET=1`. Si
   la branche `defauts-bascules` en fait le défaut, une synchronisation
   PostgreSQL validerait n'importe quoi. Il faut une primitive de
   transaction dans `DbConnection`, tenue sur une seule session. Ce n'est
   pas à moi de l'écrire avant que C ait fini (§6).
2. **Le rebâti du plein texte parle Cypher** (§2). La correction est une
   redéfinition dans `PostgresDialect` (`ORDER BY _uuid` ou une colonne de
   rang). Mais `_node_id` est un décalage de nœud rag3db : lucivy sur
   PostgreSQL demande une clé stable, à décider.
3. **Les paramètres se renomment par simple remplacement de texte**
   (`translate_params`, ligne 98, **vérifié**). Dans l'ordre des paramètres,
   `$filter_p1` est remplacé à l'intérieur de `$filter_p10`. À partir de
   onze paramètres de filtre, la requête serait fausse. Lu, pas encore
   reproduit.
4. **Ce qui se relit `Null` sans erreur** : `vector`, `jsonb`,
   `timestamptz`. Cela explique `select_chunk_vectors` → `None` : les lignes
   mises de côté sont ré-embarquées au lieu d'être relues.
5. **Les suppressions en cascade ne cascadent pas** : `batch_cascade_delete`
   compte sur `ON DELETE CASCADE`, mais aucune clé étrangère n'est posée. Les
   lignes de relation restent orphelines. Lu.
6. **Les directions ignorées** : `join_select`, `kb_gather_content`,
   `resolve_chunks_with_parent` et `fetch_with_chunks` ne regardent pas le
   sens de la relation. `fetch_with_chunks` joint dans le sens inverse de
   `CHUNKED_FROM`. Lu ; à confirmer par un test.
7. **Du Cypher hors du dialecte** : un `MATCH … count(n)` après un rebâti
   vectoriel (`catalog.rs`, erreur avalée : −1 sur PostgreSQL), des aides de
   `search.rs`, et les nœuds de migration et d'usage. Ces chemins ne servent
   pas forcément sur PostgreSQL ; la batterie le dira.

## 5. La batterie qu'une implémentation nouvelle fait passer

Une suite **unique, paramétrée par le montage** : une fonction qui rend les
cinq pièces. Elle tourne sur rag3db (sans service extérieur) et sur
PostgreSQL (`RAG3WEAVER_PG`, image `pgvector/pgvector:pg17`). Un
dialecte de plus n'ajoute qu'un montage.

**Règle :** chaque test lit la capacité déclarée. Si la capacité est
déclarée, le test exige le résultat. Sinon, il exige un **refus nommé** :
jamais un résultat vide, jamais du Cypher envoyé à une base qui ne le parle
pas.

| # | Test | Capacité lue |
|---|---|---|
| 1 | le schéma se pose, deux fois de suite sans erreur | — |
| 2 | une ingestion écrit ses lignes et ses liens, recomptés | — |
| 3 | une resynchronisation n'écrit que ce qui a changé | — |
| 4 | une suppression emporte ses relations (pas d'orphelins) | — |
| 5 | un paquet défait ne laisse rien ; un paquet validé tient après réouverture | transactions |
| 6 | le chargement en masse rend les mêmes lignes que le ligne à ligne | chargement en masse |
| 7 | chaque ligne se trouve par un mot de son texte ; aucun résultat sans ligne | plein texte |
| 8 | le plein texte se rebâtit depuis les lignes | plein texte (rebâti) |
| 9 | les vecteurs classent ; relus, ils sont égaux à ceux écrits | vecteurs, relecture |
| 10 | un filtre à 12 paramètres tient | — (vise le §4.3) |
| 11 | les cellules org/projet ne se mêlent pas | — |
| 12 | chunk → parent et parent → chunks, dans les deux sens | — |
| 13 | la reprise après incident et la prise atomique de la dette | magasin de reprise |
| 14 | aucune requête Cypher n'atteint une base qui n'en parle pas | `speaks_cypher` |

Le 14ᵉ se fait par une connexion espionne : elle refuse tout texte qui
commence par `MATCH`, `MERGE`, `UNWIND` ou `CALL` quand le dialecte ne parle
pas Cypher. Il attrape d'un coup tous les défauts Cypher oubliés du §2.

Les 19 tests de `e2e_postgres.rs` couvrent déjà une bonne part de 1, 2, 7,
9, 11 et 13. Ils deviennent la suite commune, et les mesures de seuil
(`ou_vit_la_frontiere…`, `les_poids_du_combo…`) restent à part.

## 6. Ce que je propose de faire, dans l'ordre

1. **Les capacités déclarées d'un seul tenant** : une structure
   `DialectCapabilities` (transactions, chargement en masse, plein texte
   natif, relecture des vecteurs, filtres structurés, Cypher). Les deux
   tests sur `name()` passent par elle. Les booléens existants deviennent
   des lectures de cette structure, sans changer leur sens.
2. **La batterie** (§5) sur rag3db d'abord, puis sur PostgreSQL. Le rouge
   attendu est écrit en clair : chaque défaut du §4 devient un ticket dans
   `docs/tickets/`, et son test reste rouge jusqu'à la correction.
3. **Les défauts Cypher descendent dans `Rag3dbDialect`** : plus de corps
   par défaut pour les requêtes. Un dialecte nouveau ne compile pas tant
   qu'il n'a pas tout écrit. C'est le plus gros changement, et le seul qui
   touche toute l'API du trait : **à décider par Lucie**.

   **Décidé le 10 octobre : la voie du milieu, une seule porte.** Chaque
   corps par défaut écrit en Cypher commence par regarder `cypher`. Si le
   dialecte ne le déclare pas, il rend une instruction d'un seul mot :
   `RAG3WEAVER_REFUS__le_dialecte_<nom>_ne_traduit_pas__<méthode>`. N'importe
   quel moteur la rejette à l'analyse en la citant, avant même de regarder
   les paramètres. Les défauts qui rendent une `Option` rendent `None`.

   **C'est un pis-aller, imposé par les signatures** : une méthode du trait
   rend un texte, pas un `Result`, donc le refus ne peut partir qu'avec la
   requête, et échouer à l'exécution. La forme propre, refuser **avant**
   d'exécuter, vient avec le langage intermédiaire (page 02), dont les
   traductions rendent un `Result`.
4. **La transaction dans `DbConnection`** (`begin`, `commit`, `rollback` sur
   une session tenue), après que la session recherche (chantier C) a poussé
   son runtime dédié dans `postgres_connection.rs`.

Je touche `dialect.rs`, `postgres_search_backend.rs`, `postgres_blob_store.rs`
et un fichier de test nouveau ; `postgres_connection.rs` seulement après C.

## Les questions

- **Pour Lucie** : faut-il retirer les corps par défaut en Cypher (point 3),
  ou les garder et compter sur le test 14 ?
- **Pour Lucie** : PostgreSQL est-il un produit à tenir, ou une épreuve du
  contrat ? Cela décide si les défauts du §4 se corrigent maintenant ou
  restent en tickets.
- **Pour l'orchestration** : le point 1 du §4 bloque-t-il la branche
  `defauts-bascules` pour PostgreSQL ? Je dirais non, tant que personne ne
  monte PostgreSQL en production.
