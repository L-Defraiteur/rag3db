# Premier échantillon indépendant : attentes avant les résultats

Ticket : `docs/codeparsers/tickets/2026-11-16-verifier-la-couverture-sur-des-corpus-publics-multilangages.md`.
État : deux sources publiques sélectionnées lues par C7, attentes posées avant
la campagne. Aucune extraction jouée ni couverture déclarée par cette page.
Manifeste `selection-01/selected.jsonl` SHA
`6ab2b309e0365974d8ee59d6d1c442e7b2d78963659bfd20f225d83a764053b2`.

Un nom dans un commentaire ne suffit pas. Vérifier sa définition localisée et,
lorsque le contrat de sortie l'expose, son type, son propriétaire et ses liens.
Une méthode peut être portée par un scope ou un membre : ne pas imposer une
seconde représentation pour obtenir un compteur. La ligne d'ancrage doit être
incluse dans la plage source ; elle n'est pas forcément son début, notamment
avec les attributs et commentaires. Aucune résolution inter-fichiers prouvée
par ces seules attentes locales.

## C# : classe générique héritée et membres d'expression

Dépôt `dotnet/roslyn`, commit `c6dae7dd4f51cef1680c6b23ab614394d6989a69`.
Chemin : `src/Analyzers/CSharp/Analyzers/ForEachCast/CSharpForEachCastDiagnosticAnalyzer.cs`.
SHA256 : `23327a8f8df833f8b5e1941b06ff7e5cae4126f2592772adfe3196324edeab41`.
1340 octets, BOM UTF-8 présent et conservé ; strate production.

| Construction visible dans la source | Ancrage | Vérification attendue |
|---|---:|---|
| Classe `CSharpForEachCastDiagnosticAnalyzer` | 16 | définition de classe, modificateurs internal/sealed |
| Base `AbstractForEachCastDiagnosticAnalyzer` | 16–18 | héritage générique ; arguments SyntaxKind et CommonForEachStatementSyntax |
| Attribut `DiagnosticAnalyzer` | 15 | attribut rattaché à la classe si exposé par l'IR |
| Propriété `SyntaxFacts` | 20–21 | membre de cette classe, expression, type ISyntaxFacts |
| Méthode `GetSyntaxKinds` | 23–24 | définition, retour ImmutableArray<SyntaxKind>, corps d'expression |
| Méthode `GetForEachInfo` | 26–31 | définition, deux paramètres, retour tuple avec type nullable |
| Sept directives using | 5–11 | références d'import distinctes ; aucune résolution externe présumée |
| Namespace déclaré dans le fichier | 13 | portée file-scoped, distincte d'une classe englobante |

Ce fichier exerce aussi une expression de collection moderne. Un AST invalide
doit être conservé avec ses diagnostics ; ne pas transformer automatiquement
l'absence d'une construction dans un AST invalide en défaut du résolveur.

## Go : type nommé primitif et receveurs valeur/pointeur

Dépôt `golang/go`, commit `3c6d0928959aa02598fa073f2747a1b563d3a852`.
Chemin : `src/archive/tar/format.go`.
SHA256 : `fc7d2a2584ba8cefef8e2fcc39395bdd73a1d25907d99975684bc6bf4555bf6e`.
11963 octets, strate production.

| Construction visible dans la source | Ancrage | Vérification attendue |
|---|---:|---|
| Type nommé `Format` sous-jacent int | 46 | distinguer déclaration de type, struct et interface |
| Méthode `has` | 108 | receveur valeur Format, paramètre f2 et retour bool |
| Méthode `mayBe` | 109 | receveur pointeur *Format, définition distincte |
| Méthode `mayOnlyBe` | 110 | receveur pointeur *Format |
| Méthode `mustNotBe` | 111 | receveur pointeur *Format |
| Méthode `String` | 117–132 | receveur Format, retour string, corps complet |
| Import `strings` | 7 | import ; lien Join à juger séparément de son existence |
| Bloc const avec iota et valeurs implicites | 49–106 | constructions à inventorier, sans inventer des scopes si le contrat ne les produit pas |

Le contrôle des receveurs vérifie une association, pas seulement la présence de
cinq noms de fonctions. Un type nommé manquant doit être signalé comme limite
d'extraction ou défaut selon le contrat relu, sans affirmer une perte en base
puisque ce banc ne fait aucune ingestion.

## Jugement à produire après mesure

Pour chaque ligne : fait observé dans l'inventaire compact, attendu absent,
non exposé par cette API, ou indéterminé avec motif. Garder les sorties et les
diagnostics ; créer un ticket fils pour toute lacune confirmée avec un exemple
minimal. Cet échantillon de deux fichiers ne donne aucun taux de rappel global.
