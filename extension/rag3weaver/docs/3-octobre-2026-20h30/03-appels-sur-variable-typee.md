# Les appels sur une variable dont le type se lit

3 octobre 2026, codeparsers `9a1fae3`. Le trou le plus coûteux pour « tous
les appels d'un symbole » : `node.with_delai(t)` ne devenait jamais une
relation, parce que le qualificatif `node` n'est pas un scope. Le résolveur
abandonnait l'appel.

## Ce qui a été décidé, et pourquoi

| Décision | Pourquoi |
|---|---|
| Le type d'une variable se lit dans **trois sources seulement** : son annotation, un paramètre typé, un initialiseur constructeur (`Type::new(…)` et ses cousins `default`, `new_*`, `with_*`, `from*` ; littéral de struct ; `new Type()` ; `Type()` en Python) | Cadrage de l'orchestration : aucune inférence à travers un appel. `let n = make();` ne dit rien ; une relation manquante vaut mieux qu'une fausse. |
| `Self` vaut le type de l'`impl` qui le contient | Sans cela, `let mut graph = Self::new(); graph.connect(…)` (graph.rs) perdait sa relation : le correctif l'avait d'abord fait perdre, la mesure l'a montré. |
| `Box`, `Arc`, `Rc` se déballent d'un niveau ; tout autre générique garde son nom (`Option<Node>` est une `Option`) | On appelle à travers les trois premiers les méthodes du type enveloppé ; pas à travers `Option`. |
| Un nom lié à deux types différents dans le même scope est écarté | Ne pas choisir entre deux lectures. |
| Le résolveur ne retient que les méthodes du type lu ; s'il y a plusieurs types de ce nom, tous dans d'autres fichiers, il s'abstient | Même règle que pour les homonymes : ne pas deviner. |
| Une variable typée garde son accès qualifié même quand elle est un paramètre exclu | Sans cela, le cas (b) n'existait pas : `fn f(n: &Node) { n.run() }` perdait la référence dès l'extraction. |
| Paramètres de fermeture et motifs de `match`, `if let`, `while let` sont des locales | Demandé avec le lot ; c'était le reste des fausses arêtes du lot précédent. |
| Python ne relève plus l'attribut d'un `a.b` une seconde fois sans qualificatif ; `self` et `cls` gardent le leur | La copie sans qualificatif se résolvait au premier homonyme du fichier : `n.run()` allait vers `Other.run`. Les appels `self.m()` ne tenaient qu'à cette copie ; le résolveur connaît `self`. |

## La mesure, sur `src/dataflow` (contre `f0faa82`)

Régénérable : `scripts/aretes_retirees.sh f0faa82 9a1fae3 src/dataflow`.

| | Nombre |
|---|---|
| Appels de méthode devenus des relations | **547** (487 sur la ligne de l'appel, 60 en chaîne sur plusieurs lignes : `ctx` puis `.service::<…>()`) |
| Appels qui changent de cible, pour la méthode du type déclaré | 24 (`node = node.with_delai(t)` dans les fabriques de nœuds) |
| Relations retirées | 365 : locales de motif et de fermeture (`Some(pos)`, `\|s\|`), et méthodes de la bibliothèque standard (`.clone()`, `.len()`, `.is_empty()`) qu'un homonyme du projet captait |
| Références qualifiées par une variable simple qui ont un type lisible | 4 387 sur 13 221 (33 %) |

**Le taux de faux.** 25 relations neuves tirées au hasard (graine 7), relues
à la main contre la source : toutes visent la méthode du type déclaré de la
variable (`ctx.set_output` → `NodeContext`, `store.load_execution` → le trait
du store, `tool.instantiate`, `r.schema`…). Aucune fausse dans l'échantillon.
Par construction, la cible a pour parent le type lu ; le risque restant est
une lecture de type fausse, ce que l'échantillon ne montre pas.

**Ce qui reste sans type.** Les qualificatifs les plus fréquents sans type
sont des chemins (`crate`, `std`, `serde_json`, `CypherValue`), qui ne sont
pas des variables, puis des paramètres de fermeture et des liaisons de motif
(`v`, `e`, `s`, `p`, `rec`) dont le type demande de l'inférence — hors du
cadrage.

## Pour rag3weaver

La voie des rendez-vous (`MENTIONS`, dans `code.rs`) relie un appel par nom,
sans regarder le qualificatif : un `x.run()` vers un `run` unique s'y
matérialisait déjà. Le type lu (`IdentifierReference.qualifier_type`) pourra
y servir à départager les homonymes ; c'est un changement de `code.rs`, à
proposer à la session de l'arbre principal après sa reprise du transport.
