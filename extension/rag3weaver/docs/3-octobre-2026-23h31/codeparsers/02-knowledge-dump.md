# Session codeparsers — ce qu'on sait

Tenu à jour sur place. Le détail de chaque décision est dans
`../../3-octobre-2026-20h30/01` à `08` ; ici, l'essentiel et où le trouver.

## L'analyseur

codeparsers : tree-sitter 0.24, grammaires 0.23 (`tree-sitter-c-sharp`
épinglé `=0.23.1`). Par fichier, des `ScopeInfo` (fonctions, méthodes,
types, modules, tests) et leurs `IdentifierReference` ; un résolveur relie
une référence à un scope et produit les relations.

Ce qu'une référence porte :

- `usage: Option<UsageKind>` — appel, construction, type, héritage, import…
  Le genre est **deviné** par la syntaxe du site, la ligne est **gardée**.
- `qualifier`, `qualifier_type` (le type lu d'une variable, d'un champ, d'un
  retour déclaré), `qualifier_deferred: Option<DeferredType{FieldOf,
  ReturnOf}>` (résolu au second temps, quand tous les types du projet sont
  connus).
- Chaque relation porte `sites: Vec<UsageSite{usage, line}>`.

`ScopeInfo.test: Option<TestMark{role, certainty, marker, name}>` — sûr
quand la syntaxe le dit (`#[test]`, gtest, `describe`/`it`, `TestX(t
*testing.T)`), convention quand seul le nom et le fichier le disent
(pytest). Le tableau par langage : doc `…/05`.

## Ce qui est sûr, ce qui est deviné

| Sûr | Deviné |
|---|---|
| un appel sur une variable, un champ ou un retour **dont le type se lit** vise la méthode de ce type | un nom ambigu sans type lu : rendez-vous par le nom dans rag3weaver (`MENTIONS`, marqué « par le nom ») |
| le parent d'un scope : l'homonyme qui le contient (`ace6fd8`) | le genre d'usage d'un site rare (macros, `match`) |
| les imports Rust, Python, C++ et les bibliothèques externes | Go et C# n'ont pas encore leurs imports |
| la marque de test syntaxique | la marque de test par convention (pytest) |

## Les trous restants, par utilité

1. **Un appel sur une variable sans type lu** (`node.with_delai(t)`) ne
   devient jamais une relation directe : seul le rendez-vous par le nom le
   rattrape. Le trou le plus coûteux pour « tous les appels ».
2. **Un appel par chemin vers un type externe** (`Tokenizer::from_file`)
   prend rendez-vous avec le seul homonyme du projet. Vu par les liens ;
   même voie que les accès de champ (`code.rs`), autre condition.
3. Pas d'inférence à travers les génériques ni sur deux niveaux de champ ;
   une méthode de trait appelée par `dyn Trait` ne se résout que par
   `impact` et son départ supplémentaire (le trait).
4. Un conteneur garde les références de ses membres : `Chunker` apparaît à
   côté de ses méthodes quand les sites diffèrent ; la règle « au plus
   étroit » ne joue qu'à site égal.
5. Paramètres de fermeture et motifs de `match` pas toujours pris pour des
   locales.
6. Imports Go et C# ; chemin complet d'un import local (`crate::a::Foo`).

## Côté rag3weaver

**`usages`** (`src/dataflow/usage_nodes.rs`, gabarit `usages.mmd`) : un
pivot (`Symbol`, par `name`), ses définitions (`DEFINES`), ses usages —
arêtes directes, plus le rendez-vous pour un nom ambigu — groupés par genre,
avec fichier et ligne. Requêtes par lot : `UNWIND $uuids AS u MATCH (n:T
{_uuid: u})` (jointure par hachage) ; jamais `item.champ` (produit
cartésien, journal §6).

**`impact`** (`src/dataflow/neighborhood_nodes.rs`, gabarit `impact.mmd`) :
le voisinage par niveaux (1 à 3), `CONSUMES`, `INHERITS_FROM`, `IMPLEMENTS`
entrants. Un carrefour (degré > 50) est montré, pas traversé ; ses tests
sont relevés quand même (`through_hub`). Le départ supplémentaire : méthode
→ `impl` → trait → méthode du trait (`HAS_PARENT>`, `IMPLEMENTS>`,
`PARENT_OF>`), rendu à part. Regroupement par `test_role` : les tests qui
traversent d'un côté, le code de l'autre.

**La lecture du catalogue** (`src/dataflow/catalog_read.rs`) :
`read_catalog(catalog, pivot)`. Occupé (`try_lock` échoue) : une ligne,
jamais une attente. Jamais indexé (l'état **de l'entité**, pas l'état
global) : un refus qui dit quoi faire. En cours : le résultat, avec une
ligne d'état en tête. Prêt : « rien trouvé » est une vraie réponse.

**`graph_walk`** (branche `liens`) : le moteur commun — sens, motif, saut
nu, degrés. **`LinksNode`** : voir le rapport.

## Le banc des relations

`tests/e2e_banc_relations.rs` : vingt questions sur notre propre `src/`,
réponses relevées à la main dans le code (doc `…/08`). Notes actuelles
(après `bb98827af`) :

| Question | Rappel / précision |
|---|---|
| qui appelle | 1,00 / 0,88 |
| dépend de | 0,94 / 0,61 |
| relie (chemins) | 4 / 4 |
| tests qui traversent | 0,96 / 0,93 |

Le « dépend de » à 0,61 vient surtout des appels sans type (`text.len()`
relié à un `len` du projet), de conteneurs, et de noms de champ dans les
littéraux de struct. `SHORTEST` du moteur : 11 à 18 ms par paire, plan sans
produit cartésien mais avec un `SCAN_NODE_TABLE` (propriétés des nœuds du
chemin), qui croîtra avec la table. Signalé pour la session cœur C++.

## Ce qui a été essayé sans succès

- Écarter le rendez-vous de **tout** accès qualifié par une variable sans
  type, appels compris : de vrais appelants perdus (rappel de
  begin_snapshot à 0,75). Restreint aux non-appels.
- Les liens à **quatre sauts** sur un graphe de code : presque tout se
  relie par des utilitaires moyens, que le plafond de degré ne coupe pas.
- Un chemin de longueur variable (`*1..3`) pour le voisinage : il ne laisse
  ni dédoublonner, ni plafonner, ni couper au budget entre deux sauts ; la
  marche par lots le fait.
