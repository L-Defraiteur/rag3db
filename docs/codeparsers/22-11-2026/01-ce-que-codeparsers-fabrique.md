# Ce que codeparsers fabrique, et ce que les gabarits devraient faire

22 novembre 2026, session codeparsers.

## La doctrine (Lucie, 22 novembre 2026, minuit)

> « Au tout début codeparsers était prévu pour ragforge, un projet un peu naïf ; maintenant plus aucun sens que le
> parseur aille prendre les devants sur la logique : il parse tel quel, donne la vérité ; rag3weaver range, puis
> sert les recherches, complexes ou non, par les gabarits. »

Une relation que codeparsers **fabrique**, par agrégation, recopie, rattachement par le nom ou genre deviné, est une
décision prise trop tôt, à la place d'une requête. On les retire lot par lot, après la v1, chacune avec son témoin,
et chacune remplacée par une requête de gabarit sur les seules relations lues.

## Le lot de cette nuit : une référence appartient au scope qui la contient

Le ticket de recherche [`2026-11-21-usages-rend-une-classe-comme-usage-de-type-d-une-fonction`](../../recherche/tickets/2026-11-21-usages-rend-une-classe-comme-usage-de-type-d-une-fonction.md)
remonte un symptôme : `usages copier_les_liens` rendait deux fois « class LinkRecordNode » comme usage de type. Il
a deux causes, toutes deux dans codeparsers, en Rust, Python et TypeScript :

1. **Une classe gardait les références du corps de ses méthodes.** Une classe, une interface ou un bloc `impl` lit
   ses références sur tout son nœud. Le filtre « au plus étroit » (`au_plus_etroit`, `parallel/parser_worker.rs`) ne
   valait que pour les namespaces et les modules. Il vaut maintenant aussi pour les classes et les interfaces, et
   compare **à l'octet** : un membre sur la ligne de l'en-tête ne prend pas à la classe ses références d'en-tête.
2. **Une seconde passe recopiait les références des méthodes vers la classe**, en usage `type`, vers toute classe de
   même nom du fichier, même hors de son empan : la `struct` d'un `impl`. Cette passe venait du TypeScript d'origine
   (`attach_class_field_type_references`, et sa copie Python). Elle est retirée.

Le témoin est `e2e_class_users::a_function_called_by_a_method_is_used_by_that_method_only`, en Rust, Python et
TypeScript : une fonction appelée par une méthode n'a que cette méthode pour usager, en appel, une fois.

Lucie le rappelle : « il y a intérêt des fois à rechercher toutes les références de la classe, mais c'est à faire par
gabarit, pas forcé dans le parsage ». Voir la dernière section.

## Les autres relations que codeparsers fabrique

Ce recensement a été fait par un sous-agent, en lecture du code, le 22 novembre. Les lignes citées sont celles de
cette date. Les chemins `relationship_resolver.rs`, `base_…` et `python_…` sont sous `codeparsers/src/`.

| # | passe | où | ce qu'elle fabrique | langages | qui en dépend | requête de gabarit qui la remplacerait |
|---|---|---|---|---|---|---|
| 1 | `include_child_refs` | `relationship_resolver.rs:689-697`, `:947-958` | `CONSUMES` de la classe vers ce que ses méthodes utilisent | tous | personne : rag3weaver la coupe (`code.rs:1477`) | `(c)-[:PARENT_OF*0..2]->(m)-[:CONSUMES]->(x)` |
| 2 | `include_file_level_refs` | `relationship_resolver.rs:679-687` | à chaque scope, les `CONSUMES` du scope de fichier | tous | personne : rag3weaver la coupe (`code.rs:1476`) | `(s)-[:DEFINED_IN]->(f)<-[:DEFINED_IN]-(fs)` puis les `CONSUMES` de `fs` |
| 3 | imports d'un conteneur sur tout son nœud | `base_…:582-583`, `:3053`, `:3097`, `rust_…:480`, `:502` | `au_plus_etroit` ne taille pas `import_references` : une classe, un `mod tests` ou un namespace « utilise » (`USES_LIBRARY`) une bibliothèque que seuls ses membres utilisent | tous les conteneurs | `USES_LIBRARY` ; le paramètre `relation` de grep et search | recalculer `import_references` depuis les références restantes ; `(c)-[:PARENT_OF*0..2]->(m)-[:USES_LIBRARY]->(l)` |
| 4 | `ensure_import_references_tracked` | `base_…:3107-3167` | une référence `Other` trouvée par recherche de mot dans le texte, pas lue dans l'arbre | tous sauf Python | des `CONSUMES` et `MENTIONS` de genre « other » | aucune : un nom que l'arbre rate est un défaut de couverture du parseur |
| 5 | une fonction garde les références de ses fermetures | `base_…:724` | la référence de la fermeture, en double sur la fonction | tous | personne : rag3weaver replie déjà les fermetures (`fold_lambdas`) | le repli à l'ingestion, ou `(f)-[:PARENT_OF*1..]->(l {scope_type:'lambda'})-[:CONSUMES]->(x)` |
| 6 | borne d'un `TypeVar` | `python_…:1529-1553`, `:1574` | là où `T` sert, une référence `Type` vers la borne de `T` | Python | personne nommément | `(f)-[:CONSUMES]->(t {name:'T'})-[:CONSUMES]->(b)` |
| 7 | relations inverses (`CONSUMED_BY`, `HAS_PARENT`, `DECORATED_BY`) | `relationship_resolver.rs:1438-1466`, défaut `include_inverse` | des miroirs, sans aucun fait de plus | tous | `CONSUMED_BY` : `relation` de search et grep ; `HAS_PARENT` : usages, impact, `impact_fichier` | `include_inverse: false`, et les gabarits écrivent `<CONSUMES`, `<PARENT_OF` (la syntaxe de pas les accepte) |
| 8 | `PARENT_OF` posé par le nom, hors de l'empan | `relationship_resolver.rs:1121-1152` (repli sur le premier homonyme, `:1136`) | une contenance qui ne contient rien : la méthode Go vers sa struct, `Foo::bar` C++ hors classe | Go, C++ | les pas `HAS_PARENT>` d'usages et d'impact | `PARENT_OF` seulement si l'empan contient l'enfant ; `parent_name` reste un champ, rejoint par un rendez-vous de rag3weaver dans le même fichier |
| 9 | rattachement au premier homonyme du fichier | `base_…:3083`, `python_…:1558`, `relationship_resolver.rs:615-626`, `:791-806` | un `CONSUMES` vers un homonyme choisi, sans marque | tous | `impact` et `callees` suivent ces arêtes (`resolution = fichier`) | à plusieurs candidats, rien ou `by_name`, puis le rendez-vous à un seul définisseur |
| 10 | genre d'héritage deviné sur le texte | `detect_relationship_type_by_name`, `relationship_resolver.rs:1668-1751` | `INHERITS_FROM` / `IMPLEMENTS` choisis sur des sous-chaînes (« extends », `impl…for`, deux-points) | tous | les rendez-vous de rag3weaver (`code.rs:2138`) | l'espèce lue sur la clause (`HeritageClauseClause`) ; le gabarit lit `(c)-[:INHERITS_FROM\|IMPLEMENTS]->(b)` |
| 11 | `site_vers` : un « other » requalifié en « call » | `relationship_resolver.rs:1763-1769` | un genre d'usage déduit de la cible | tous | `usages` groupe et filtre par `usage` | `CASE WHEN r.usage='other' AND t.scope_type IN ['function','method'] THEN 'call' ELSE r.usage END` |
| 12 | verrous par appel (frontière) | `parallel/parser_worker.rs:240-287` | `Lock` / `SharedLock` vers un champ, depuis `self.x.lock()` et le type déclaré du champ | Rust, C++ | `LOCKS`, `impact` et `callees` | à l'ingestion, depuis `field_types` et la mention qualifiée de `lock`, si `MENTIONS` garde le qualificatif (non vérifié) |

**Ce qui est une lecture légitime, hors de cette liste** :
- les types de retour et de paramètres lus dans la signature, et la première passe
  d'`attach_class_field_type_references` (les types des champs) ;
- l'en-tête `impl Trait for Type`, lu à sa ligne ;
- les clauses d'héritage ;
- les `#include` et `using` rattachés au scope le plus étroit ;
- `DECORATES`, `DEFINED_IN` et les prototypes C de niveau fichier.

## La classe et ses membres, par gabarit

**Ce qu'il faut pouvoir demander** : « tout ce que la classe et ses méthodes utilisent », et « qui utilise la classe
ou ses méthodes ». Après ce lot, **aucun outil exposé ne le fait** :
- `usages` ne part que des définitions du nom ;
- le `also_path` d'`impact` n'atteint que la méthode de base de même nom ;
- `callees` n'a pas d'`also_path`.

**Le moteur sait déjà le faire.** `NeighborhoodNode` accepte `also_path` avec `also_same` vide. Deux gabarits
suffiraient :
- `callees` avec `also_path='PARENT_OF>', also_same=''` ;
- `impact` avec ce même `also_path`.

Voici les mêmes requêtes en Cypher, sur les seules relations lues :

```cypher
// ce que la classe et ses méthodes utilisent
MATCH (c:Scope {name:$n})-[:PARENT_OF*0..2]->(m)-[r:CONSUMES|INHERITS_FROM|IMPLEMENTS|USES_LIBRARY]->(x)
RETURN DISTINCT m.name, type(r), x.name, r.line
// qui utilise la classe ou ses méthodes
MATCH (c:Scope {name:$n})-[:PARENT_OF*0..2]->(m)<-[:CONSUMES|INHERITS_FROM|IMPLEMENTS]-(u)
WHERE NOT (c)-[:PARENT_OF*1..]->(u)
RETURN DISTINCT u.name, u.file_path, m.name
```

Ticket : [`2026-11-22-la-classe-et-ses-membres-ne-se-demandent-pas-ensemble`](../tickets/2026-11-22-la-classe-et-ses-membres-ne-se-demandent-pas-ensemble.md).

**Deux limites** :
- `check_also_path` refuse un dernier pas répété. Pour des classes imbriquées, il faut donc écrire
  `PARENT_OF>+|PARENT_OF>?`.
- En Go et pour le C++ hors classe, ce chemin repose sur le `PARENT_OF` posé par le nom (l'entrée 8).
