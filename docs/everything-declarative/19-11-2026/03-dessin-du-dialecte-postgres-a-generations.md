# Dessin : le dialecte postgres à générations (2.11)

Session everything declarative, 19 novembre 2026, 16 h 15. Entrée 2.11 de la passation du 16 novembre ; ticket
`2026-11-14-le-dialecte-postgres-n-a-pas-la-forme-a-generations.md`. Cette page est à relire par c7 avant le code.

## Ce que la passation sous-estime

La passation et le ticket décrivent le manque comme quatre méthodes de dialecte à écrire en SQL :
- `sait_les_generations` ;
- `condition_visible_sur` ;
- `create_generation_table`, `merge_generation` et `latest_generation_rank`.

Ces méthodes ne suffisent pas. L'écrivain à générations ne passe pas tout entier par le dialecte : il écrit lui-même
du Cypher brut, dans lequel il colle la clause de visibilité que rend le dialecte. Passer `sait_les_generations` à vrai
sur postgres enverrait ce Cypher à PostgreSQL. Aujourd'hui, ces chemins sont seulement gardés par
`ok_or("ce dialecte ne sait pas les générations")`.

## L'inventaire (master `55dfce1271`)

**Les méthodes du dialecte** (`src/dialect.rs`), aujourd'hui à leur défaut `None` ou `false` sur `PostgresDialect` :

| méthode | qui l'appelle |
|---|---|
| `sait_les_generations` | l'ouverture d'une base neuve (l'estampille `…/generations`) |
| `condition_visible_sur` | `generations.rs:183` et les sites bruts ci-dessous |
| `condition_sur_predicat` | `generations.rs:185`, la pile d'une vue (2b′) |
| `create_generation_table`, `merge_generation`, `latest_generation_rank` | `catalog.rs` (~10625, ~10758) |
| `read_generation` | le chargeur des vues (2b′) |

Le rendu SQL du prédicat `Visible` existe déjà (`postgres_condition_sur`, `dialect.rs:1474`). Les deux méthodes de
condition n'ont qu'à l'appeler, comme `Rag3dbDialect` appelle `rag3db_condition_sur`.

**Le Cypher brut de l'écrivain à générations** :

| site | fonction | forme |
|---|---|---|
| `code_sync.rs:455` | `decide_methods_defined_elsewhere` | lecture filtrée (`IN`, `=`, visible) |
| `code_sync.rs:541` | `ecrire_la_table` | lecture filtrée, visible OU `_jusqu_a = $w` |
| `code_sync.rs:620` | `ecrire_la_table` | `SET _jusqu_a` par uuids |
| `code_sync.rs:679` | `completer_les_naissances` | lecture filtrée, visible |
| `code_sync.rs:763` | `recopier_les_aretes_entrantes` | recopie d'arêtes (`MATCH …-[r]->… CREATE`) |
| `code_sync.rs:780` | `recopier_les_aretes_entrantes` | recopie de `CONSUMED_BY` |
| `code_sync.rs:830` | la déliaison des arêtes | `detach_edges_returning` + `replacen(" DELETE r", …)` sur le texte rendu |
| `code_sync.rs:1565` | `synchroniser_par_generations` | lecture des fichiers visibles |
| `code_sync.rs:1712` | `lignes_visibles_des_fichiers` | lecture filtrée, visible |
| `catalog.rs:~10684` | le contrôle d'une base à générations | `WHERE NOT (visible) … LIMIT 1` |
| `catalog.rs:~10784` | les comptes d'une génération | `SET` sur `_generation` par `_key` |

Le site 830 retouche à la main le texte d'une requête rendue par le dialecte. Il casserait sur tout dialecte qui
n'écrit pas ` DELETE r`.

## Le dessin proposé

**Une forme de l'IR par site, rendue par chaque dialecte.** C'est le sens du projet : « une forme se traduit, elle ne
s'hérite pas en Cypher » (`SchemaDialect::hop`). L'IR a presque tout :

1. **Les lectures filtrées** (455, 541, 679, 1565, 1712) deviennent des `Select` : un filtre `AllOf` d'`Equals`, d'`In`
   et de `Visible`, avec les colonnes rendues. Il manque un **`Predicate::Not`** pour 541 (« visible OU `_jusqu_a` = w »
   s'écrit avec `AnyOf`) et pour `catalog.rs:~10684`. Je l'ajoute : rendu `NOT (…)` sur les deux dialectes.
2. **Le `SET _jusqu_a` par uuids** (620) devient `Write::Mark`, qui existe et se rend déjà sur les deux dialectes.
3. **La recopie d'arêtes** (763, 780) devient une forme neuve, `Write::CopyEdges { relation, from, to, side, filter }`.
   Elle recopie les arêtes d'une ligne ancienne vers sa remplaçante, pour des paires `(ancienne, neuve)` données en
   paramètre. Rag3db : le Cypher d'aujourd'hui. Postgres : `INSERT INTO rel(from_uuid, to_uuid, …) SELECT … FROM rel
   JOIN unnest($paires)`.
4. **La déliaison visible** (830) : `Write::Detach` gagne un filtre sur l'autre bout (`end_filter: Option<Predicate>`),
   et `detach_edges_returning` le rend lui-même. Plus de `replacen`.
5. **Les comptes d'une génération** (`catalog.rs:~10784`) : une méthode de dialecte de plus,
   `set_generation_counts`, comme `merge_generation`, car `_generation` est une table interne.
6. **Les méthodes du dialecte postgres** : `sait_les_generations`, les deux conditions (par `postgres_condition_sur`),
   et la table `_generation` dans le schéma interne (`internal_table`), avec
   `INSERT … ON CONFLICT (_key) DO NOTHING`, `SELECT max(rang)` et la lecture d'une génération.
7. **Les trois colonnes** (`_couche`, `_depuis`, `_jusqu_a`) par `alter_add_column_default`, à vérifier sur `Scope` et
   `File`. Et les index postgres sur `(_couche, _depuis, _jusqu_a)`, sans lesquels chaque lecture visible balaie la
   table.

**L'ordre des lots** : chacun sur rag3db d'abord, à comportement égal, puis postgres.
- **Lot A**, rag3db seul : les sites 1 à 5 passent par l'IR. Aucun changement de comportement : les témoins de
  l'écrivain (`e2e_code_sync`, la lib) restent verts, et chaque forme neuve a son témoin de rendu.
- **Lot B** : postgres sait les générations (6 et 7).

## Les témoins

- **Lot A** : un témoin de rendu par forme neuve (`Predicate::Not`, `Write::CopyEdges`, le `end_filter` de `Detach`),
  sur les deux dialectes, comme les témoins de rendu existants. Puis les suites de l'écrivain inchangées :
  `e2e_code_sync` (exigée par `code_sync.rs` et `catalog.rs`) et la lib.
- **Lot B, rouge d'abord** :
  - une base de code neuve sur postgres porte l'estampille `code-scope-v1/content/generations`, et non
    `…/content` (c'est le « comment le voir » du ticket) ;
  - puis les témoins de l'écrivain qui ne lisent pas de Cypher brut, joués sur postgres. Le serveur est là :
    `rag3weaver-pg` tourne à lucie-tablette, et `run_e2e.sh` ajoute la feature quand le port répond.
- **Passe** : `SUITE_DU_TEMOIN=e2e_postgres SUITES=e2e_code`, et `e2e_code_sync` par les chemins.

## Ce que je demande à c7

1. **La forme par l'IR** (proposée) plutôt qu'une méthode de dialecte par site. Les méthodes iraient plus vite pour
   postgres seul, mais garderaient le Cypher brut dans l'écrivain et le `replacen`.
2. **Deux lots**, A puis B, A étant un refactor à comportement égal. Ou un seul lot, plus gros.
3. **L'estimation** passe de 2 jours (passation) à environ 3 jours d'agent : environ 1,5 pour A et 1,5 pour B.
