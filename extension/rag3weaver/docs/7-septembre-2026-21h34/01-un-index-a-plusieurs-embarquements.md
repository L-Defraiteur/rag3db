# Un index à plusieurs embarquements

**7 septembre 2026, 21 h 34.** Conception, avant le code. Cartographié en
lecture seule sur `5cfa26a11` ; les lignes citées sont celles de ce commit.

## 1. La demande, et ce qu'elle veut dire

> plusieurs modèles sur le même index ; migrer de modèle sans redécouper ;
> la recherche prend la colonne du modèle courant ; le garde-fou actuel devient
> le cas dégénéré.

Aujourd'hui un index **est** un modèle : `_catalog_meta["embedding_model"]`
porte une signature `nom:dim` posée par le premier embarqueur réel, et tout
autre est refusé (`catalog.rs:1964`, `EmbeddingModelMismatch`, `catalog.rs:71`).
Changer de modèle, c'est ré-indexer — le message d'erreur le dit en toutes
lettres.

Ce qui rend le changement bon marché, c'est que **presque tout est déjà
paramétré, sauf le nom** :

| déjà paramétré | figé |
|---|---|
| `create_vector_index(table, column, index)` (`dialect.rs:175`) | la colonne : le littéral `"embedding"` en huit endroits |
| `embed_set(table, embedding_col)` (`dialect.rs:499`) | le marqueur : `_embed_hash` en dur dans le même `SET` |
| `reclamer_chunks_sans_marqueur(table, marqueur, …)` (`dialect.rs:459`) | l'index : la convention `{table}_vec` reconstruite par `format!` (`search.rs:915/956/993`, `catalog.rs:1240/1264/1311/1444/1777`) |
| `EmbedNode::with_columns` (`record_nodes.rs:2317`) | jamais appelé en production — les cinq sites prennent le défaut |
| `ColumnType::Vector(dim)` par colonne (`dialect.rs:31`) | un seul `embedding_dim` global (`config.rs:898`, défaut 384) |

Et deux choses sont **déjà neutres** vis-à-vis du modèle, ce qui borne le
chantier : la clé d'un chunk (`chunk_uuid`, `uuid.rs:45` — parent, champ,
rang ; ni texte ni modèle), et le contrat offset → chunk (décalage de ligne
et bornes en octets, `fts_handle.rs:118`, `search.rs:2515` — aucune colonne
de vecteur n'y passe). **Ré-embarquer n'exige pas de redécouper** : les deux
dettes ont chacune leur colonne, leur passe et leur graphe.

## 2. Le schéma proposé

### 2.1 L'identité d'un modèle : un slug, dérivé, et jamais `?`

`Embedder::name()` rend `bge-m3`, `granite-107m`, `granite-278m`, `minilm`…
et **`"?"` par défaut** (`embedder.rs:64`) — trois implémentations le rendent
tel quel (candle, bge-m3 candle, callback). Une colonne `embedding__?` n'est
pas un nom.

- `name()` **perd son défaut** : obligatoire sur le trait. Les trois
  implémentations anonymes déclarent leur nom ou ne compilent pas.
- Le **slug** est une fonction pure du nom : minuscules, `[a-z0-9]`, tout le
  reste devient `_`, sans doublon ni bord. `bge-m3` → `bge_m3`,
  `granite-278m` → `granite_278m`. C'est lui qui nomme le stockage.
- La **signature** `nom:dim` reste ce qu'on écrit en méta et ce qu'on compare :
  deux modèles de même slug et de dimension différente sont un conflit, pas
  deux modèles.

Rien de tout ça ne connaît un modèle en particulier.

### 2.2 Le stockage d'un modèle : une colonne, un marqueur, un index

Par table à vecteurs — `{Entity}_Chunk`, mais aussi `{KB}_Index` **et**
`{KB}_Index_Chunk`, qui portent tous deux `{kb}_embedding` (`schema.rs:243`,
`:301`) :

| | entité simple | base de connaissances |
|---|---|---|
| colonne | `embedding__{slug}` | `{kb}_embedding__{slug}` |
| marqueur | `_embed_hash__{slug}` | idem |
| index HNSW | `{table}_vec__{slug}` | idem |
| dimension | `Vector(dim du modèle)` | idem |

Séparateur `__` : les slugs n'ont que des `_` simples, la coupe est sans
ambiguïté dans les deux sens.

**La dimension est par colonne, donc par modèle.** `CatalogConfig::embedding_dim`
cesse d'être la source : elle vient de `embedder.dim()`, gravée dans le DDL
de *sa* colonne. Le service `"embedding_dim"` (`catalog.rs:2235`) rend celle
du modèle courant. Un index peut porter `granite_107m` en 384 et
`granite_278m` en 768 côte à côte.

### 2.3 Le cas dégénéré, sans rien déplacer

Un index existant a `embedding`, `_embed_hash`, `{table}_vec`. On ne les
renomme pas — un `RENAME COLUMN` est gratuit mais l'index HNSW ne suit pas, et
le reconstruire sur un gros index coûte ce qu'on veut précisément éviter.

À la place, **le nom d'une colonne n'est jamais dérivé : il est résolu depuis
la méta.** Chaque modèle enregistré dit comment il est stocké :

```
embedding_model:{slug}  →  {"name":"granite-278m","dim":768,"storage":"suffixed"}
embedding_model:{slug}  →  {"name":"bge-m3","dim":1024,"storage":"legacy"}
```

`storage: legacy` résout vers `embedding` / `_embed_hash` / `{table}_vec`
(et `{kb}_embedding` pour une KB) ; `suffixed` vers les noms du §2.2. Une
seule fonction fait la résolution :

```rust
fn vector_storage(&self, table: &VectorTable, slug: &str) -> VectorStorage
// VectorStorage { column, marker, index, dim }
```

et c'est **le seul endroit** qui connaît les deux conventions. Les huit
`format!` disparaissent derrière elle. Le garde-fou d'aujourd'hui est
exactement un index qui n'a qu'une entrée `legacy` — le cas dégénéré, sans
code spécial.

### 2.4 La méta : des clés préfixées, pas un JSON unique

`load_meta_by_prefix` existe et sert déjà à `vector_index_dropped:`
(`catalog.rs:1322`). Une clé par modèle :

- lisible sans parser une liste, ajoutable sans réécrire les autres ;
- `embedding_model` (singleton) reste **en lecture seule**, pour qu'une
  bibliothèque v5 qui tomberait sur la base sache au moins ce qu'elle a devant
  elle ; elle n'est plus jamais écrite.

Il n'y a **pas** de clé « modèle courant » : le courant est l'embarqueur avec
lequel le processus a ouvert le catalogue. Un index ne choisit pas, il
accueille.

### 2.5 La migration de schéma v6 — une seule chose, et pas celle qu'on croit

`migrate_scope_columns` (`catalog.rs:3195`) rejoue des blocs à colonnes
**fixes** sous un garde d'égalité de chaîne. Ajouter un modèle — N colonnes
dont le nom vient des données — n'y rentre pas, et ne doit pas y rentrer.

**v6 fait exactement une chose** : lire `embedding_model` = `nom:dim`, en
dériver le slug, écrire `embedding_model:{slug}` avec `storage: legacy`. Zéro
DDL sur les vecteurs. Un index qui n'avait pas de `embedding_model` (jamais
embarqué, ou factice seulement) n'a rien à inscrire.

**Ajouter un modèle est une opération hors version**, à la volée :

```
register_embedding_model(slug, name, dim):
  pour chaque table à vecteurs :
    alter_add_column(table, colonne, Vector(dim))        — absorbe « existe déjà »
    alter_add_column_default(table, marqueur, TEXT, "")  — idem
    create_vector_index(table, colonne, index)           — skip_if_exists
  upsert_meta("embedding_model:{slug}", …)
```

Idempotente — deux écrivains qui enregistrent le même modèle en même temps
font deux fois la même chose sans se gêner ; c'est le patron de
`migrate_scope_columns`, rendu paramétrique. `SCHEMA_VERSION` ne bouge pas.

## 3. Ce qui change, nœud par nœud

**`check_embedding_model` (`catalog.rs:1964`)** — devient
`ensure_embedding_model` : si le slug est connu, vérifier que la dimension
est la même (sinon `EmbeddingModelMismatch`, qui ne reste que pour ce cas) ;
sinon `register_embedding_model`. Un catalogue en lecture seule n'enregistre
rien, comme aujourd'hui. Les trois appelants (`:2471`, `:3962`, `:6544`) ne
changent pas.

**`ChunkRecordNode` et `KBChunkNode`** — **rien**, et c'est la mesure qui l'a
dit : un chunk n'a pas besoin de naître avec un marqueur par modèle. Une
colonne ajoutée par `ALTER … DEFAULT ''` vaut `''` pour les lignes d'avant et
`NULL` pour une insertion qui ne la nomme pas, et
`reclamer_chunks_sans_marqueur` compte déjà les deux comme « à faire ».
`compute_chunks` et ses douze tests restent intacts.

**`EmbedNode` (`record_nodes.rs:2265`)** — `embedding_col` cesse d'être un
défaut que personne ne surcharge : le nœud résout `(colonne, marqueur)` par
`vector_storage` pour le modèle courant. `embed_set` prend le marqueur en plus
de la colonne (`dialect.rs:1157`, `:1855` : le `_embed_hash` en dur dans le
`SET`). `embed_check_hashes` (`:1149`, `:1848`) prend le marqueur au lieu de
rendre `_embed_hash` fixe. `RecordVectors.dense` reste un `Option` — on
n'embarque qu'avec un modèle à la fois, et embarquer avec N modèles à
l'ingestion multiplierait le coût par N sans que personne l'ait demandé.

**`KBEmbedNode` (`record_nodes.rs:1365`)** — dérive `{kb}_embedding` de lui-même ;
passe par la même résolution.

**`embarquer_le_retard` (`catalog.rs:2465`)** — le marqueur qu'il passe à
`reclamer_chunks_sans_marqueur` est celui du modèle courant. Et, mesuré par
l'optimiseur (c85748da4) : au-delà de ~1 500 chunks à embarquer en 384 ou
~2 000 en 768, tomber l'index HNSW de **cette colonne** et le reconstruire
coûte moins que d'y insérer ligne à ligne. La passe le fait d'elle-même, par
colonne, avec le même drapeau de méta que le lot — un processus mort laisse
l'ouverture suivante reconstruire.
**C'est la migration** : ouvrir l'index avec le nouveau modèle, l'enregistrer,
et le retard vaut 100 % — le rattrapage existant fait le reste, chunk par
chunk, sans relire ni réécrire une ligne de chunk. `_embed_claim` reste
unique par ligne : deux migrations concurrentes vers deux modèles se
sérialiseraient sur les mêmes lignes. Acceptable, et dit.

**`VectorSearchNode` (`generic_search_nodes.rs:236`) et `Catalog::search`
(`catalog.rs:6651`)** — résolvent `(colonne, index)` pour le modèle courant et
les passent au backend. `SearchBackend::vector_search*`
(`search_backend.rs:169`, `:184`) gagne un paramètre `column`. La couverture
d'un modèle est un **retard** ordinaire, déjà compté par
`count_marqueur_manquant` et déjà dit par `Disponibilites` : chercher sur un
modèle à moitié embarqué rend ce qui est embarqué, comme aujourd'hui pendant
une ingestion.

**Le backend PostgreSQL (`postgres_search_backend.rs:214`, `:251`, `:269`)** —
écrit `embedding` en dur et ignore `index_name`. Il prend la colonne. C'est
aussi une **correction** : les KB sur pgvector, avec leur `{kb}_embedding`,
n'ont jamais été servies par ce chemin.

**`vector_indexes_of` / `bulk_vector_index` (`catalog.rs:1258`, `:1289`)** —
ne détruit et ne reconstruit que l'index du modèle courant : c'est le seul
qui reçoit des vecteurs pendant le lot. `rebuild_vector_index` (`:1310`) reçoit
la colonne au lieu du littéral. Le drapeau `vector_index_dropped:{table}`
devient `vector_index_dropped:{table}:{index}` — aujourd'hui, une seule valeur
par table, N index sur la même table en écraseraient N−1 et un processus mort
laisserait des index détruits sans trace.

**L'undo (`record_nodes.rs:2999`)** — remet à NULL le marqueur du modèle
courant, pas `_embed_hash` en dur. Le marqueur voyage dans le contexte d'undo
avec les uuids : l'undo n'a pas de catalogue sous la main pour le résoudre.

**Le DDL neuf (`schema.rs:363`, `:243`, `:301`, `:645`)** — une base neuve n'a
**aucune** colonne de vecteur à la création : elles arrivent avec le premier
`register_embedding_model`. La table `_Chunk` naît avec ses marqueurs de texte
et de découpe seulement. Le signal `vector` dans `EntityConfig` continue de
dire « cette entité se cherche par vecteur » — c'est lui qui décide si une
table reçoit les colonnes à l'enregistrement d'un modèle.

## 4. Ce qui n'est pas touché, et pourquoi

- **La vue par parent et la projection** (`group_by`, `_parent_uuid`,
  `PROJECT_GRAPH_CYPHER`) : aucune ne porte de vecteur. Seul le nom d'index
  passé à `QUERY_VECTOR_INDEX` change de source.
- **Le sparse** : dans un `SparseHandle` du blob store, clé par table, marqué
  par `_sparse_hash` — pas une colonne, pas couplé au dense. Un seul modèle
  sparse par index reste vrai. Le dire ici pour ne pas le redécouvrir.
- **Le démon** : un démon, un modèle, port 7878, `Identite.modele` — c'est
  suffisant pour dériver le slug. Migrer = relancer le démon avec le nouveau
  modèle, ouvrir, rattraper. Un démon multi-modèles est un autre chantier.
- **L'heuristique de taille** : le défaut `granite-278m` est posé depuis
  `bb538cb0d` (`MODELE_PAR_DEFAUT`) ; l'heuristique, elle, n'existe qu'en prose
  (`docs/optimiseur/6-septembre-2026-16h30/04`). Sa place est là où le dépôt
  est déjà compté — l'analyse de code, avant `Catalog::new` — pas dans le
  démon qui ne voit pas le corpus. Après ce chantier, qui le rend possible :
  deux modèles sur un index, c'est ce qu'il faut pour en changer.

## 5. Ce qui dirait que ça marche

1. **Deux modèles, un index.** Ingérer avec A ; ouvrir avec B ; B s'enregistre,
   le retard vaut 100 %, `embarquer_le_retard` le solde ; chercher avec B ;
   rouvrir avec A, chercher — les deux répondent, chacun sur sa colonne.
2. **Le cas dégénéré.** Une base v5 ouverte par v6 : une clé de méta écrite,
   **zéro** DDL, et la même recherche rend les mêmes résultats. C'est le test
   qui prouve que la migration ne coûte rien.
3. **Le conflit qui reste.** Même slug, dimension différente →
   `EmbeddingModelMismatch`, en le nommant.
4. **pgvector sur une KB** — cherchable, pour la première fois.
5. **Un nom obligatoire** : un `Embedder` sans `name()` ne compile pas.
6. **Le lot** : `bulk_vector_index` avec deux modèles enregistrés ne détruit
   qu'un index et le retrouve après un arrêt brutal.
7. **Un modèle absent refuse en le disant.** Index ouvert en lecture, ou
   modèle jamais enregistré : la recherche vectorielle rend une erreur —
   *« modèle `granite_107m` non disponible sur cet index ; disponibles :
   legacy=bge-m3, granite_278m »* — et jamais zéro résultat en silence. C'est
   la règle du dépôt : distinguer « ça n'existe pas » de « je ne te le montre
   pas ».

## 6. Ce qui reste à trancher

- ~~**`__` comme séparateur.**~~ Tranché : `__` sépare, et **un slug est
  refusé à l'enregistrement** s'il contient `__` ou un caractère hors
  `[a-z0-9_-]` — la coupe reste sans ambiguïté par construction, pas par
  convention. Reste à l'éprouver à l'exécution sur les deux moteurs.
- ~~**Retirer les colonnes d'un modèle.**~~ Tranché : pas d'`unregister`. Il
  détruit des vecteurs et personne ne l'a demandé.
- **Le relais `Arc<T>`** (`embedder.rs:117`) ne transmet pas `troncatures()` —
  un défaut à part, trouvé en chemin, à corriger dans le même passage sur le
  trait ou séparément.
