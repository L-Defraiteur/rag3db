# `cited_by` par une forme IR déclarée : ce qui manque à l'IR, et la pièce générique

*18 octobre 2026, mémoire longue, pour l'arbre principal (qui tient l'IR et le dialecte)
et l'orchestration. Règle de Lucie : personne n'écrit de Cypher, et un nœud Rust propre à
un outil est le mauvais remède ; tout doit se dire par le dialecte, l'IR ou un script
dans un nœud. Étude de lecture sur master 007b2c3e7, rien n'est codé.*

## La requête, en mots

Depuis `Memory`, par `CITE`, vers `Ref` : garder les Refs dont `genre ∈ $genres` (option)
et dont `valeur = $value` ou commence par `$value + '/'` ou `$value + '#'` ; filtrer
`Memory` par `state`, `kind` (listes, option), `updated_at >= $since` (option), et par un
sujet d'ancrage (`Memory -[:ANCHORED_TO]-> Subject {name: $subject}`, option) ; rendre par
fiche ses champs, les noms de ses sujets et les Refs qui ont répondu ; trier par
`coalesce(updated_at, created_at)` décroissant ; `limit`, `offset`, et le total avant la
limite. Un `q` plein texte limité à ces fiches, en plus.

## (1) Est-ce que ça s'écrit comme une forme IR déclarée aujourd'hui ? Non.

- **Pas de déclaration** : `Form`, `Select`, `Hop`, `Predicate`, `Column`, `Plan` ne
  dérivent que `Debug, Clone, PartialEq, Eq` (`ir/src/form.rs`, `plan.rs`) ; seuls
  `FilterCondition`, `Value` et `FieldType` se désérialisent. Une forme ne se construit
  qu'en Rust.
- **Pas de nœud qui exécute une forme** : aucun `FormNode` ; les seuls usagers de l'IR
  sont des nœuds Rust (`NeighborhoodNode`, `FetchRelatedNode`, `SelectRecordsNode`, les
  liens, les usages). Rien de tel dans `BASE_NODES` (`src/backend_code.rs:244-305`).
- La chaîne déclarée la plus proche (`SelectRecordsNode` → `FetchRelatedNode` →
  `IntersectResultsNode` → `PaginateNode`) ne sait ni filtrer par une traversée (Select
  refuse les jointures, `composable_results.rs:200-202` ; `FetchRelatedNode` n'a pas de
  filtre), ni rassembler les Refs par fiche, ni trier par date décroissante ;
  `PaginateNode` lit son `offset` d'un port `query` qu'un outil hors recherche n'a pas.

## (2) Ce qui manque à l'IR

Dans l'ordre où `cited_by` en a besoin ; tout vaut pour tous les gabarits.

| manque | où | note |
|---|---|---|
| **`Predicate::StartsWith{field,param}`** (et `Not`, `IsNull` pour finir la famille) | `ir/src/form.rs:181` (+ `names()`), rendu dans `rag3db_condition` / `postgres_condition` (`src/dialect.rs:1281, 1316`) | `FilterParser` a déjà `starts_with` (`filter.rs:517`), mais seulement en `Compiled` |
| **Un prédicat par traversée** : `Predicate::Through{relation, direction, end, filter}` → `EXISTS { (m)-[:R]->(x:End) WHERE … }` | `form.rs`, rendu au `select` du dialecte (`dialect.rs:1382`) | sert le filtre par Ref et le filtre par sujet ; un EXISTS, pas une jointure (pas de lignes multipliées) |
| **Un ordre qui a un sens et une expression** : `Vec<Order{expr, desc}>`, avec `coalesce(a, b)` comme expression permise | `form.rs` (`Select`, `Hop`), dialecte | aujourd'hui `order_by` est une liste de noms, croissante |
| **`offset`** (SKIP) sur `Select` et `Hop` | `form.rs`, `dialect.rs:1392` | |
| **Le total avant la limite** : un `Count::Matching` qui accepte le même prédicat (dont `Through`), dans le même `Plan` | `form.rs:146`, `plan.rs` | `Plan` a déjà plusieurs sorties |
| **Rassembler le lié par ligne** : une colonne `Column::Collect{relation, direction, end, fields, filter}` → `collect({…})` sous un `OPTIONAL MATCH` | `form.rs:55`, dialecte | les Refs qui ont répondu, et les noms des sujets d'ancrage, sont le même manque |
| **Un prédicat facultatif** : `Predicate::Optional(Box<Predicate>)`, retiré quand ses paramètres sont absents ou vides | `form.rs`, appliqué avant le rendu | un graphe n'a pas de `if` ; aujourd'hui un `in []` vide ne rend rien |
| **La déclaration** : `serde` sur `Form`, `Select`, `Hop`, `Count`, `Predicate`, `Column`, `Order`, `Plan`, avec `validate()` comme porte (identifiants, paramètres nommés, jamais de littéral) | `ir/src/form.rs`, `plan.rs` | la pièce qui rend tout le reste déclarable |

Les paramètres restent des **noms** dans la forme et voyagent en `QueryParam`
(`value.rs:131`) : aucune valeur n'est jamais écrite dans le Cypher (seule exception
connue : `EdgeExclusion`, des identifiants, `dialect.rs:1436`).

## La pièce générique : un nœud qui exécute une forme déclarée

**`PlanNode`** (le nom est à trancher avec l'arbre principal), dans `src/dataflow/`, ajouté à
`BASE_NODES` :

- **config** : `plan` (un `Plan` déclaré, en JSON dans le `.mmd` ou un fichier du
  gabarit), `params` (`{nom: "$param_d_outil"}`, par la substitution `$x` qui existe déjà,
  `graph_tool.rs:336-365`, ou depuis un port), `format` (`rows` | `results`) ;
- **contrôle** : `Plan::validate()` à la création du nœud, puis la politique du
  backend (seules ses entités et relations déclarées) ;
- **exécution** : `Catalog::run_plan` en lecture (`catalog.rs:9465`, la variante des
  lectures d'outil), par la vue de lecture quand elle suffit ;
- **sorties** : `rows` (JSON, une ligne par fiche avec ses colonnes `Collect`), `total`
  (le `Count` du plan), et `results` (le port des résultats), pour que `RenderResultsNode`
  rende le markdown ou le JSON (`format=markdown|json`, déjà là) et
  qu'`IntersectResultsNode` borne une recherche plein texte (`q`) aux fiches rendues.

Avec lui, `cited_by` est **un `.mmd` sans Rust propre** : un `PlanNode` (la forme
ci-dessus), un `RenderResultsNode`, et pour `q` un `KBQuerySourceNode` →
`BM25SearchNode` → `IntersectResultsNode`.

## (3) Le nœud de script peut-il composer le reste ?

En partie. `src/script` (rhai, TypeScript par quickjs, JavaScript) est **JSON vers JSON,
sans catalogue ni connexion ni IR** (`script/mod.rs:8-11`), et les nœuds scriptés d'un
gabarit (`nodes/`) aussi ; `RhaiNode` n'a que `rhai_scripts` et `rhai_limits`. Un script
peut donc **mettre en forme** (regrouper, nommer « la partie qui a répondu », rendre un
markdown propre) après le `PlanNode`, mais ne peut **rien lire** : il ne remplace aucun
manque du (2).

## Le partage proposé

- **Arbre principal** : les extensions de l'IR et leur rendu par les deux dialectes
  (`StartsWith`, `Through`, `Order`, `offset`, `Count` sur le même prédicat, `Collect`,
  `Optional`, `serde` + `validate`), avec leurs témoins de rendu.
- **Mémoire longue** : `PlanNode` dans `BASE_NODES`, l'outil `cited_by.mmd` du gabarit
  `memory`, les genres `path` et `symbol` par `seed` (lot à part, déjà écrit), et les
  témoins demandés : un par genre, un `cited_by` sur un dossier (préfixe, tri, total,
  `format=json`).
- Un seul jeu de types entre les deux : la forme JSON d'un `Plan` se fixe d'abord, par un
  exemple écrit ensemble (la forme de `cited_by`), avant le code.
