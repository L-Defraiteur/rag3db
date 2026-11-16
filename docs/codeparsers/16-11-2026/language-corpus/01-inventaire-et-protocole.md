# Corpus publics : inventaire et protocole initial

État : inventaire statique, aucune compilation ni mesure. Ticket porteur :
`docs/codeparsers/tickets/2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md`.
Base parent `bef0b27cb3`, codeparsers `bd133850f89d6b12a3ca2b2f6ce238785ed37e3a`.
Ce document ne qualifie pas le correctif Rust7601, dont les validations restantes sont distinctes.

## Entrées réellement accessibles

Le chemin `parallel/project_parser.rs` → `parallel/parser_worker.rs` reconnaît :

| Langage | Extensions reconnues |
|---|---|
| TypeScript | `.ts`, `.tsx` |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` |
| Python | `.py`, `.pyi` |
| Rust | `.rs` |
| Go | `.go` |
| C | `.c` |
| C++ | `.h`, `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hxx` |
| C# | `.cs` |

Les extensions sont mises en minuscules. `.h` est routé C++, même dans un dépôt C.
`.mts` et `.cts` ne figurent pas dans cette table. TS et JS partagent le cache
`BaseScopeExtractionParser` initialisé avec `Language::Typescript` ; ne pas prétendre
qu'une grammaire JS dédiée est utilisée par cette entrée sans preuve supplémentaire.
L'enum universelle contient aussi Java/Kotlin/Ruby/PHP : ces noms ne sont pas une
preuve de routage ou d'extraction. Generic reste une façade heuristique séparée.

Les API spécialisées ne passent pas par ce routeur et ne fournissent pas toutes des scopes :

| Façade | Mécanisme lu | Sorties publiques principales | Limite |
|---|---|---|---|
| CSS | tree-sitter CSS | stylesheet, rules, at_rules, relationships | validité AST non exposée dans le résultat |
| SCSS | tree-sitter SCSS vendor | stylesheet, rules, at_rules, relationships | `.sass` ne prouve pas la prise en charge du dialecte indenté |
| Vue | expressions régulières | sfc, blocks, relationships | aucun AST Vue validé |
| Svelte | parser tree-sitter sans langage initialisé | component, blocks, relationships annoncés | `set_language` est commenté ; `parse(...).expect(...)`, exécution à qualifier séparément |
| Markdown | extraction documentaire | document, relationships | sous-parseurs TS/Python non initialisés ; ne pas supposer MDX ou code embarqué complet |
| HTML | stub explicite | `Err` | contenu ignoré, parsing déclaré non implémenté |

Sources : `src/{css,scss,vue,svelte,markdown,html}/*parser.rs` et leurs `types.rs`.
HTML est une absence déclarée de capacité, pas un fichier syntaxiquement invalide.
Svelte est une capacité annoncée dont le chemin lu est incomplet, pas un langage
qualifié. Le ticket du banc conserve ces constats ; leur éventuelle correction
exigera un témoin rouge et un lot distinct.

Dans l'index rag3weaver, un autre suffixe peut recevoir une étiquette de langage
puis `analyser_texte_brut` : un scope TexteBrut, `aucun_parseur=true`, `ast_valid=true`.
Cette combinaison signifie « aucune tentative de parsing », jamais « AST correct ».

## Versions et options à figer

Aucune section `[features]` dans le Cargo.toml de codeparsers ; les dépendances de
grammaires lues ne sont pas optionnelles. Le verrou autonome lu déclare : tree-sitter
0.27.1, TypeScript 0.23.2, JavaScript 0.25.0, Python 0.25.0, Rust 0.24.2, Go 0.25.0,
C 0.24.2, C++ 0.23.4, C# 0.23.5, CSS 0.25.0, HTML 0.23.2, SCSS 1.0.0 vendor,
Bash 0.25.1. Bash sert notamment au découpage shell ; ce n'est pas une neuvième
entrée du routeur de scopes. La version effective sera relue dans le verrou de
la compilation retenue, avec hash du verrou, du manifeste et du binaire.

Les options spécialisées sont enregistrées avec leurs valeurs effectives ; aucun
« défaut » implicite non traçable. Pour Markdown, fixer `parse_code_blocks=false`
pour la première passe documentaire ; sections/liens/blocs/tables/listes/front
matter sont par défaut activés. Pour Vue enregistrer `parse_directives`,
`parse_components`, `extract_composables` ; CSS `include_rules`, `extract_variables`,
`project_root` ; SCSS `max_nesting_depth`, `resolve_uses` ; Svelte `parse_reactives`,
`parse_stores`, `extract_components`. Leur présence ne démontre pas leur efficacité.

## Contrat proposé pour l'acquisition et la CLI

Banc possède acquisition et manifeste ; C7 sélection et coordination. Codeparsers
réserve `examples/language_corpus_probe.rs` ; ce fichier n'est pas encore livré.
Aucun script, test, programme ou installation provenant des dépôts téléchargés
n'est exécuté. Aucun accès aux services ou bases de Lucie.

Entrée proposée : JSONL UTF-8, un enregistrement par fichier, clés anglaises :
`repository_url`, `repository_commit`, `license_id`, `license_path`, `relative_path`,
`sha256`, `language`, `facade`, `stratum`, `size_bytes`. La racine locale de chaque
checkout est fournie séparément, pas incorporée comme identité pérenne. Les
identifiants de dépôt et la table des racines restent à arrêter avec Banc avant
implémentation. Préserver tous les candidats et exclusions dans un manifeste
annexe : aucun rejet silencieux.

Tirage : seed publiée, commit figé, chemins normalisés triés, strate production /
tests / documentation explicite. Déduplication par contenu documentée séparément
du nombre d'occurrences ; ni copies ni fixtures générées pour atteindre 2000.
Sous-effectifs, licences, provenance des fichiers générés et exclusions sont
publiés. Vérifier hash et taille avant analyse ; refuser sortie de racine et
symlink traversant une frontière. Le manifeste sélectionné lui-même est hashé.

Commande envisagée : `language_corpus_probe --manifest PATH --roots PATH --output PATH`.
Aucune option ici ne désigne une CLI déjà implémentée. La première passe est
strictement par fichier ; la résolution inter-fichiers aura sa recette bornée
par projet et sa propre mesure. Ne pas inventer un projet géant avec tous les dépôts.

## Mesures et interprétation

Chaque ligne résultat garde l'identité d'entrée, la façade réellement appelée,
les options, le hash du binaire/verrou et les diagnostics bruts disponibles.
Les dimensions suivantes sont indépendantes, pas un booléen « parseOK » :

- `routing_status` : `supported`, `unsupported_extension`, `unimplemented`, `raw_fallback`.
- `execution_status` : `completed`, `read_error`, `decode_error`, `parser_error`, `panic`, `timeout`.
- `ast_status` : `valid`, `invalid`, `not_exposed`, `not_attempted` ; conserver les erreurs/missing nodes seulement si observables.
- `capture_status` : `nonempty`, `empty`, `not_applicable` ; `coverage_status=unjudged` sans oracle annoté.
- `scopes`, `symbols`, `references`, `relations` : décomptes par type de sortie ; `null` avec raison si la façade ou la phase ne les produit pas.

Une relation documentaire ou CSS n'est pas une arête de résolution du graphe.
Les scopes ne sont pas automatiquement des symboles. Des captures absentes ne
sont établies que sur une attente annotée et sourcée ; zéro scope ne suffit pas.
Le nombre de fichiers sans erreurs AST n'est pas un taux de couverture sémantique.

`elapsed_ns` mesure l'appel, séparé de lecture/décodage/initialisation ; publier
la politique de réutilisation des parseurs et l'ordre des fichiers. Le pic RSS
est celui du processus (unité octets et source OS explicites), pas un pic par
fichier si le processus est réutilisé. Le pic cgroup du superviseur est une autre
mesure, avec son périmètre ; aucun des deux ne mesure le seul tas du parseur.
Une absence de métrique reste `null`, jamais zéro inventé.

Avant toute charge : source figée, revue du protocole d'entrée avec Banc,
réservation fraîche auprès de C7, budget, poste, target unique et pilote canonique
confiné. Aucun ancien GO Rust ne s'applique à cette campagne. Première erreur
inattendue : preuve conservée et diagnostic, pas de relance aveugle.

## Accord d'interface avec Banc

Identité retenue pour les acquisitions GitHub :
`repository_id=github.com/owner/repo@<full_commit>` ; URL canonique et SHA complet
restent présents dans chaque entrée. Pour une autre forge, arrêter d'abord sa
forme canonique plutôt qu'inventer une identité au téléchargement.
Le fichier `--roots` est un objet JSON :

```json
{"schema_version":1,"repositories":{"github.com/owner/repo@<full_commit>":"/absolute/checkout"}}
```

Ajouter `repository_id` aux champs JSONL proposés ci-dessus. `license_id` contient
une expression SPDX, y compris les choix multiples ; `license_path` pointe la
preuve relative. Licence inconnue : exclusion motivée, jamais MIT inférée.
`relative_path` est POSIX relatif, sans composant `..`, ni traversée de symlink.
Hash SHA-256 des octets bruts avant analyse, UTF-8 strict, aucun décodage lossy.
BOM et CRLF sont conservés : le lecteur ne normalise ni contenu ni positions.
Les octets non UTF-8 restent visibles dans l'inventaire des rejets motivés.

Vocabulaire initial fixé :

| `language` | `facade` | État de la cohorte |
|---|---|---|
| `typescript`, `javascript`, `python`, `rust`, `go`, `c`, `cpp`, `csharp` | `scope` | routeur de code |
| `css` | `css` | API spécialisée |
| `scss` | `scss` | API spécialisée |
| `sass` | `scss` | dialecte indenté à qualifier séparément |
| `vue` | `vue` | API spécialisée regex |
| `svelte` | `svelte` | limite d'initialisation connue |
| `markdown` | `markdown` | API documentaire |
| `mdx` | `markdown` | dialecte à qualifier séparément |
| `html` | `html` | absence de parsing déclarée |

Les cohortes de limites ne sont pas des langages qualifiés. Pas de sélection
Generic ou Bash dans cette première interface. `stratum` vaut `production`,
`test` ou `documentation` ; les cas ambigus reçoivent une règle de classement
publiée dans le manifeste annexe.

La déduplication globale par SHA est un indicateur de contenu unique, pas une
permission de supprimer un cas de routage. La sélection préserve au moins les
couples `(language, facade)` et les dialectes/strates annoncés : un même contenu
`.c`/`.h` ou `.ts`/`.tsx` peut déclencher une autre analyse. Toutes les occurrences
restent dans l'inventaire annexe. Publier séparément fichiers sélectionnés,
contenus uniques et sous-effectifs, sans gonfler les quotas par duplication.
