# État des lieux — ce que codeparsers sait dire d'un usage

3 octobre 2026. Premier temps du chantier « genre d'usage » (vision du jour,
`3-octobre-2026-20h16/01-vision-explorer-les-relations.md`, §2-A). Question :
pour chaque usage d'un symbole, l'analyseur sait-il dire **comment** on s'en
sert (appel, type, import, héritage…) et **à quelle ligne** ?

Lu sur codeparsers `6c9e4b1` (sous-module `extension/rag3weaver/codeparsers`,
dépôt `L-Defraiteur/codeparsers`) et rag3db `c5e4ef1f5`. Les cas réels
viennent d'une sonde qui passe de vrais fichiers du dépôt dans
`ProjectParser::parse_project` et imprime chaque référence et chaque relation.

## 1. En une phrase

L'analyseur garde la **ligne** de chaque usage, mais **aucun genre** : le
champ `context` est le texte de la ligne, et le type de relation (`CONSUMES`,
`INHERITS_FROM`, `IMPLEMENTS`) est **deviné après coup** par des recherches de
sous-chaînes dans ce texte. La relation résolue, elle, perd la ligne.

## 2. Ce qui sort, et ce qui se perd

| Étage | Porte la ligne ? | Porte un genre ? |
|---|---|---|
| `IdentifierReference` (une par occurrence) | oui, `line` + `column` | non : `kind` dit comment le nom a été **résolu** (`Import`, `LocalScope`, `Unknown`), pas comment il est **utilisé** |
| `ResolvedRelationship.metadata` | non | non : `context` = texte de la ligne (ou signature, ou rien) ; `clause` existe mais n'est **jamais** rempli |
| Relations d'héritage directes (`resolve_heritage_relations`) | non | par leur type seulement ; `metadata: None` |
| rag3weaver (`src/code.rs`) | jette tout | garde le type ; `MENTIONS.kind` = le type deviné |

Où le genre est déjà **connu puis jeté** au moment de l'extraction :
- Python distingue l'appel (`call`) de l'accès d'attribut, dans une clé
  interne qui n'est pas stockée ;
- Rust visite `type_identifier` à part (`visit_rust_type_refs`) ;
- les références tirées des signatures (types des paramètres et du retour)
  sont toutes des usages de type ;
- le trait d'un `impl Trait for X` est poussé à part ;
- les clauses d'héritage (`heritage_clauses`) sont lues dans l'AST.

## 3. Par langage

« Fiable » = lu sur un nœud de l'AST ; « deviné » = expression régulière ou
sous-chaîne.

| Langage | Appel | Type | Import | Héritage | Ligne |
|---|---|---|---|---|---|
| TS / JS | capté comme simple identifiant, non distingué | **manqué dans les corps** (`type_identifier` non visité) ; deviné sur les signatures (nom à majuscule) | regex ES, par fichier, sans ligne (sauf `import()`) | AST (`extends` / `implements`) | exacte pour les corps ; début du scope pour les signatures |
| Python | AST le sait, **le jette** ; chaque appel compte **deux fois** | annotations captées comme identifiants nus | regex, sans ligne, `is_local` toujours vrai | AST (superclasses, toutes en `Extends`) | exacte |
| Rust | non distingué | **AST** (`type_identifier`, `scoped_identifier`) | **aucun** : `use` n'est pas lu (regex TS) | `impl Trait for` : référence synthétique + regex sur la signature | exacte ; ligne de l'`impl` pour le trait |
| Go | non distingué | **manqué** : la visite des types existe mais n'est jamais appelée | aucun | inclusion de champ devinée par la forme du membre | exacte |
| C | non distingué | manqué (même code mort que Go) | aucun (`#include` ignoré) | — | exacte |
| C++ | non distingué | manqué | aucun | AST (`base_class_clause`), tout en `Extends` | exacte |
| C# | non distingué | capté (la grammaire nomme les types `identifier`), non distingué | aucun (`using` ignoré) | AST + **nom en `I` majuscule ⇒ interface** | exacte |

Vue, Svelte, Markdown, CSS, SCSS, HTML, texte brut : pas de références entre
symboles (regex de fichier ou découpe seule).

## 4. Cas réels tirés de ce dépôt

**Corpus d'`e2e_code`, Rust (`src/dataflow/port.rs`, `services.rs`)**
- `merge_port_values → take_or_clone` sort **huit fois**, une par ligne
  d'appel : les références locales ne sont pas dédoublonnées, les autres
  le sont par cible (la première ligne gagne). Le nombre de relations dépend
  donc de la façon dont le nom a été résolu.
- `ServiceRegistry IMPLEMENTS is` : le mot « is » vient d'un commentaire de
  doc (`services.rs:15`, ramassé par la regex des zones hors scope) ; comme
  la signature du scope est `impl Default for ServiceRegistry`, l'heuristique
  `impl\s+\w+\s+for` fait de **toute** référence de ce scope une
  implémentation. *Correction du même soir* : cette arête-là passe par les
  références de niveau fichier, que rag3weaver n'active pas
  (`include_file_level_refs: false`) ; ce qui l'atteint, c'est
  `file_scope_01 → is`, un `CONSUMES` tiré du même commentaire. Les autres
  cas de ce paragraphe atteignent rag3weaver (sonde rejouée avec ses
  options : 235 relations sur ces deux fichiers, contre 260 avec les options
  par défaut).
- `PortValue IMPLEMENTS PortValue` : un `impl PortValue` sans trait relié à
  l'enum du même nom comme une implémentation.
- `new → count` (`port.rs:292`, `let count = data.len();`) : une variable
  locale prise pour la méthode `count` du même fichier.

**C++ (`src/include/binder/bound_use_database.h`)**
- `class BoundUseDatabase : public BoundDatabaseStatement` donne
  **trois** arêtes vers la base : `INHERITS_FROM` par la clause (sans
  contexte), `INHERITS_FROM` deviné sur la ligne `: BoundDatabaseStatement{…}`
  (l'appel du constructeur parent), et `CONSUMES` pour ce même appel.
- Le scope `rag3db` (un namespace) « consomme » `BoundUseDatabase` : les
  références des enfants remontent au namespace.

**Python (`scripts/test_backend_snapshot.py`)**
- `first = ingest(s1, [card(…), card(…), card(…)])` donne six `CONSUMES
  first → card` : chaque appel est vu comme appel **et** comme identifiant.

**JS (`ui/chat/app.js`)**
- `setBusy → busy` (une écriture, `busy = value`) et `setBusy → $`
  (des appels, `$("send")`) sont le même `CONSUMES`.

**Go et C#** (absents du dépôt, cas en ligne)
- Go : `var s Store` ne produit aucune référence à `Store` ; `t := NewStore()`
  donne bien `Use → NewStore`.
- C# : `class Item : Index` donne deux `INHERITS_FROM` (clause + devinette).

## 5. Un défaut trouvé en passant : C# panique dans rag3weaver

codeparsers verrouille `tree-sitter-c-sharp` 0.23.1 dans son propre
`Cargo.lock`, mais comme dépendance il demande `"0.23"`, et le `Cargo.lock`
de rag3weaver résout **0.23.5**. Cette version est une grammaire d'ABI 15,
que tree-sitter 0.24 refuse : `failed to set C# language: LanguageError
{ version: 15 }`, panique à l'ouverture de tout fichier `.cs`
(`c_sharp_scope_extraction_parser.rs:137`). Les tests de codeparsers passent
parce qu'ils utilisent leur verrou. Reproduit par la sonde, corrigé en la
rebâtissant avec 0.23.1. Correctif proposé : épingler `=0.23.1` dans le
`Cargo.toml` de codeparsers.

## 6. Ce que le second temps a fait (codeparsers `11ecd4f`)

Le genre et la ligne sortent désormais sur chaque relation d'usage
(`RelationshipMetadata.sites`). Ils ne corrigent aucune fausse arête, puisque
les types de relation ne changent pas, mais ils la rendent visible : avec les
options de rag3weaver, les deux `IMPLEMENTS` du corpus (`PortValue`,
`ServiceRegistry` vers eux-mêmes) portent un site `type` et non `inheritance`,
et les deux `INHERITS_FROM` devinés sur l'appel du constructeur parent en C++
portent un site `other`. C'est la contradiction que le lot suivant pourra
lever en préférant le genre lu sur l'AST à la devinette par sous-chaîne.

## 7. Ce que le second temps devait faire

Dans codeparsers seulement, sans changer les relations sorties ni leur
nombre :

1. un genre d'usage typé (`Call`, `Type`, `Import`, `Inheritance`, `Other`)
   posé **à l'extraction**, là où le nœud de l'AST le dit, sur
   `IdentifierReference` ;
2. ce genre et la ligne recopiés dans `RelationshipMetadata` (nouveaux
   champs optionnels), et la ligne posée sur les relations d'héritage ;
3. des tests par langage, écrits d'abord, qui vérifient le genre et la ligne
   de cas comme ceux du §4.

Ce qui reste hors du second temps, et à décider ailleurs :
- les fausses relations du §4 (« is », l'auto-implémentation, les doublons,
  les variables locales) : les corriger **change** le nombre de relations,
  donc les résultats d'`e2e_code` ; à proposer à part ;
- l'épinglage C# (§5) : petit, dans codeparsers, proposé comme commit séparé ;
- le rangement du genre en base (propriété sur l'arête ou relation
  « appelle ») : choix de Lucie ; le transport par `CodeRelation` se
  proposera à la session de l'arbre principal.
