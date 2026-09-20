# Magic → rag3weaver : problèmes rencontrés et résolutions

État au 19 septembre 2026, 19 h 50, Europe/Paris. Base Git de la session :
`20a8f6ee8`. Les corrections décrites sont dans le répertoire de travail ; elles
ne constituent pas encore un commit ni une livraison du produit Magic.

Convention retenue pour ce dossier : `docs/DD-MM-YYYY/NN-sujet.md`.

## Résultat vérifié

Un petit corpus de test traverse maintenant l’écriture dans rag3db, la relecture
des listes et objets, puis un DAG Mermaid de recherche avec filtres imbriqués.
Le même test passe avec lucivy seul, le dense local seul et leur combinaison.
Le dense de ce test provient du vrai démon BGE-M3, en 1 024 dimensions.

Le problème de relecture attribué initialement aux tableaux était une différence
de forme du résultat de `Catalog.get()`, pas une disparition des capacités en base.
Les tests ont néanmoins révélé d’autres défauts réels dans le transport des types,
le compilateur de filtres et la copie des paramètres du parseur C++.

Cela valide le parcours ciblé. Cela ne signifie pas que toute la collection Magic
est déjà ingérée, que toutes les opérations de maintenance sont éprouvées, ou
que le MCP est installé. Voir [la validation et les travaux restants](02-validation-et-reprise.md).

## Périmètre et décisions

L’expérimentation Magic vit dans `experiments/mtga/`, dossier ignoré par Git.
Elle fournit déjà une API FastAPI de snapshots avec données structurées et JSON
Schema : cartes, collection, decks et registre de mécaniques. Le snapshot de
travail compte 27 071 entrées de cartes, 5 380 impressions possédées, 188 decks
et 430 entrées de glossaire. Ces nombres concernent la source, pas un index
rag3weaver intégral validé.

Choix demandés par Lucie :

- rag3db comme base de destination ; lucivy pour le plein texte ; dense local
  sur la deuxième carte dédiée, reliée à la TV ;
- conserver des identifiants stables et des payloads typés ;
- prévoir les recherches et leurs DAG avant de figer le mapping ;
- corriger les besoins génériques dans rag3weaver, réserver les connaissances
  propres à Arena à l’API ou à l’adaptateur Magic.

Le modèle métier retenu est `Card → Ability → Mechanic`. `Ability` désigne une
capacité native Arena avec son identifiant et son texte FR/EN ; `Mechanic` désigne
un concept partagé et sa définition. Le texte imprimé des capacités reste dans
le texte indexé de la carte. Les définitions complètes du glossaire ne sont pas
répétées systématiquement dans chaque carte. Une mention lexicale ne devient
pas une relation affirmant que la carte possède cette capacité.

Ce modèle est une décision de raccordement ; sa matérialisation complète sur
les données Magic reste à réaliser.

## Chemins d’accès : clarification

Lucie se souvenait correctement d’un accès plus explicite que l’API simplifiée :

| Niveau | Points d’entrée | Rôle |
|---|---|---|
| API simplifiée | `register_entity`, `ingest_entities`, `get` | Commodités de déclaration, écriture et lecture |
| Schéma explicite | `CatalogConfig`, `EntityDef`, `RelationDef` | Entités, champs, relations |
| Exécution composée | `GraphDefinition`, Mermaid, `DataflowRuntime` | Assemblage de nœuds d’ingestion et de recherche |
| Base | `DbConnection`, `DaemonConnection`, endpoint `/cypher` | Requêtes directes et lecture témoin |

Le catalogue utilise lui-même des DAG. Le chemin DAG n’est pas un second moteur
indépendant : certains nœuds consultent encore le catalogue comme service,
notamment pour les index, la cohérence et les filtres. Contourner `get()` est
possible ; contourner toute l’orchestration oblige à fournir ses services.

## 1. Types Json et Tags réduits à du texte

**Constat.** `FieldType::Json` et `FieldType::Tags` aboutissaient à
`ColumnType::Text`, donc à `STRING` dans rag3db. Un tableau JSON conservé comme
texte reste décodable, mais les opérateurs de liste ne peuvent pas directement
le traiter comme une liste native. Cela ne prouvait pas que rag3db était incapable
de stocker ces valeurs : le moteur possède déjà listes et `STRUCT`.

**Correction.** Ajout de types récursifs explicites `FieldType::List` et
`FieldType::Struct`, avec leur traduction en colonnes natives. Les anciens types
`Json` et `Tags` restent textuels : aucune conversion silencieuse des anciennes
bases. Les changements de type de colonnes existantes restent refusés par le
mécanisme de migration.

Ajout de `JsonSchemaMapping` : résolution des références locales, types primitifs,
listes, objets fixes et unions avec `null`. Le mapping expose aussi des chemins
filtrables, leurs types, opérateurs et scopes de tableaux d’objets. Les objets
ouverts sont signalés dans `opaque_paths` et conservés comme JSON textuel ; les
unions hétérogènes et références récursives ne sont pas devinées.

**Fichiers.** [config.rs](../../src/config.rs),
[dialect.rs](../../src/dialect.rs),
[schema.rs](../../src/schema.rs),
[json_schema.rs](../../src/json_schema.rs).

## 2. Listes vides et valeurs nulles : type perdu dans les paramètres

**Échec réel.** Une première ligne contenant `nums: []` faisait inférer
`STRING[]` au transport natif, alors que la colonne attendait `INT64[]` :

```text
Expression STRUCT_EXTRACT(item,nums) has data type STRING[]
but expected INT64[]. Implicit cast is not supported.
```

**Cause.** La conversion d’une liste s’appuyait sur son premier élément, avec
`STRING` en l’absence d’élément. Le même principe ne suffit pas pour les tableaux
d’objets dont les premières valeurs sont nulles.

**Piste écartée après essai.** Transmettre du JSON textuel puis le caster vers
`STRUCT[]` fonctionne sur des exemples simples, mais l’essai avec guillemets,
barres obliques inverses et retour à la ligne ne restituait pas exactement les
échappements attendus. Ce chemin n’a pas été retenu.

**Correction.** Annotation de type explicite à la frontière des paramètres :
`CypherValue::Typed`, son équivalent sur le fil du démon, puis construction des
valeurs natives avec les types des listes et des champs de structure. Les
valeurs des records et checkpoints restent ordinaires ; l’annotation est ajoutée
aux paramètres d’insertion et de mise à jour. Une validation précède l’entrée
FFI pour les paramètres annotés.

**Validation.** Le test réel relit des listes vides, listes nulles, nombres avec
premier élément nul et tableaux d’objets contenant des champs nuls.

**Fichiers.** [connection.rs](../../src/connection.rs),
[daemon/db.rs](../../src/daemon/db.rs),
[rag3db_connection.rs](../../src/rag3db_connection.rs),
[record_nodes.rs](../../src/dataflow/record_nodes.rs).

## 3. `Catalog.get()` : le tableau était sous `n`

**Symptôme.** Après une ingestion sans échec, le test ne trouvait pas
`abilities` dans la map renvoyée par `get()`.

**Comparaison décisive.** La lecture directe `RETURN n, n.abilities` rendait à la
fois le nœud complet et le tableau attendu. Les propriétés étaient présentes
sous la colonne `n`. `row_to_map` associait simplement les noms de colonnes aux
valeurs, donnant une forme `{"n": {"abilities": [...]}}`.

**Correction.** Déballer cette forme connue de nœud complet pour la lecture du
catalogue : une seule colonne `n`, une seule valeur map, avec les métadonnées de
nœud `_label` et `_uuid`. Une projection arbitraire d’objet n’est pas aplatie.

**Validation.** La comparaison des champs de chaque record source avec les
champs relus passe, y compris les tableaux. La seconde ingestion du même lot est
comptée inchangée.

**Fichier.** [catalog.rs](../../src/catalog.rs).

## 4. Fonctions de filtre de liste inexistantes

**Échec réel.** Le dialecte produisait `list_any_match(...)` et `list_all(...)`.
Le moteur interrogé répondait :

```text
Catalog exception: function LIST_ANY_MATCH does not exist.
```

**Correction.** Utiliser les quantificateurs natifs vérifiés :
`any(v IN liste WHERE prédicat)` et `all(v IN liste WHERE prédicat)`.
Les opérateurs `has_any`, `has_all`, `has_none` conservent des valeurs paramétrées.

**Validation.** La recherche exacte de `Blue` trouve les deux lignes attendues ;
`Blu` ne les trouve pas. Les tests de génération de clauses ont été actualisés.

**Fichiers.** [dialect.rs](../../src/dialect.rs),
[filter.rs](../../src/filter.rs).

## 5. Assertion C++ sur un paramètre dans un prédicat de tableau

**Échec réel.** Une fois la syntaxe du quantificateur corrigée, le filtre
paramétré déclenchait `KU_UNREACHABLE` dans
`ParsedParameterExpression::copy()`.

**Cause localisée.** Cette méthode ne savait pas copier une expression de
paramètre. Le chemin de traitement du prédicat de liste demande cette copie.

**Correction.** Construire une nouvelle `ParsedParameterExpression` conservant
le nom du paramètre, son nom brut et son alias. Recompilation du cœur partagé
utilisé par le démon de test.

**Validation.** Le même filtre paramétré s’exécute ensuite dans les deux tests
d’intégration ; aucun collage des valeurs utilisateur dans le texte Cypher.

**Fichier.** [parsed_parameter_expression.h](../../../../src/include/parser/expression/parsed_parameter_expression.h).

## 6. Conditions sur un même élément et valeurs nulles

**Besoin.** « Une capacité a le libellé Flying et un niveau ≥ 3 » doit exclure
une ligne où Flying a le niveau 1 et Ward le niveau 3.

**Ajout.** `FilterCondition::Nested` porte le chemin du tableau et une condition
entière évaluée sous un seul quantificateur. `FilterCondition::Path` désigne un
chemin local explicite ; le sens historique de `Field.key = "Entité.champ"`
reste réservé aux jointures.

**Échec intermédiaire.** Le test incluait encore une ligne où le niveau de Flying
était nul. Le prédicat inconnu devait être traité comme non satisfait dans cette
recherche existentielle.

**Correction.** `COALESCE(prédicat, false)` à l’intérieur du quantificateur et
`COALESCE(..., false)` sur son résultat. Le compilateur valide les composants du
chemin. La source du DAG vérifie les branches structurées contre les types
déclarés et refuse un filtre mal formé avant de lancer les signaux.

**Validation.** Le record où les deux critères sont sur la même capacité est
le seul retenu. Les records aux critères répartis, au tableau vide, au tableau
nul et au niveau nul sont exclus. Un chemin traversant un tableau sans `nested`
fait échouer le DAG, au lieu d’élargir les résultats.

**Fichiers.** [filter.rs](../../src/filter.rs),
[json_schema.rs](../../src/json_schema.rs),
[generic_search_nodes.rs](../../src/dataflow/generic_search_nodes.rs).

## 7. Port de sortie du DAG déjà consommé

**Symptôme.** Le graphe s’exécutait, mais le test ne pouvait pas relire
`resolve.results` dans sa sortie finale.

**Cause.** Le runtime libère un port lorsque son dernier consommateur l’a pris.
Dans ce template, `RenderResultsNode` consomme les résultats de `resolve` et les
réexpose. Le port intermédiaire n’est donc pas la sortie terminale.

**Correction.** Le template déclare `render.results`, qui reste une liste
structurée. `render.meta` permet de récupérer les métadonnées propagées. Il ne
faut pas confondre ce port avec `render.text`, le rendu textuel.

**Validation.** Le test instancie le vrai template, construit le DAG et lit
`render.results`. Il vérifie le même UUID attendu dans les trois configurations
lucivy, dense et hybride, avec `limit = 1`.

**Fichiers.** [search_structured.mmd](../../templates/tools/search_structured.mmd),
[structured_payloads.rs](../../tests/structured_payloads.rs).

## 8. Incidents d’environnement et de compilation

| Incident | Diagnostic / résolution |
|---|---|
| Feature `daemon` seule ne compilant pas | `EmbedDaemon::identite` appelait un helper Burn absent sans feature Burn. Compilation conditionnelle ; précision déclarée `unknown` sans Burn. |
| Header `rag3db.hpp` introuvable | Utiliser le répertoire du header amalgamé du build natif, pas seulement `src/include`. |
| Avertissement COPY `ESCAPED_NEWLINES` | Le cœur du build `lecteurs` était ancien. Les derniers tests utilisent `build/lecteurs-csv/src/librag3db.so`. |
| Reconstruction de `rag3db` seule | Cette cible produit la bibliothèque statique. Il faut aussi la cible `rag3db_shared` pour le démon chargé dynamiquement. |
| Tests de serveurs refusés dans le sandbox | Les tests d’ouverture de ports échouaient avec `Operation not permitted`. Relance autorisée hors sandbox : suite unitaire verte. |
| Test d’origine interprétant `/tmp` comme dépôt | Échec observé dans le contexte sandbox ; non reproduit lors de la relance complète hors sandbox. Pas de correction de ce module. |

## GPU utilisé

Le démon déjà actif sur `127.0.0.1:7878` déclarait `bge-m3`, 1 024 dimensions,
`factice: false`, précision `Flex32`. Son environnement déclarait le modèle
BGE-M3 et le régime `confort`. Ses descripteurs ouverts comprenaient
`/dev/dri/renderD130`, correspondant à PCI `0000:07:00.0`, la carte de la TV selon
le repérage local existant. Il ouvrait aussi le périphérique intégré ; aucun
profilage d’exécution par noyau n’a été réalisé.

La documentation locale associe cette carte à `gpu:1` (deuxième carte dédiée).
Ce numéro ne doit pas être confondu avec `drm/card1`. Le test a réutilisé ce démon,
sans téléchargement de modèle ni fournisseur distant.
