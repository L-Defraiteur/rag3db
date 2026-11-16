# Premier corpus public : résultats et limites

État : campagne FULL02b en cours, ne pas lire ce document comme un reçu de fin.
Le bilan chiffré complet sera ajouté après le verdict du superviseur.
Ticket parent : `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md`.

## Sources et périmètre réellement exécuté

Parent `384165760b7aef3eaa4fcf0c7ade8143d6bd88dc`, exemple/parser
`1b35cfb66ae160954981d7dd3a90f5ef88e95b01`, bibliothèque de production
inchangée depuis bd133850f89d6b12a3ca2b2f6ce238785ed37e3a.
ELF SHA `bff1cafb72e8c6662d2d595457b0ca6da4718686ae64b91ae1bde7a4e95565e2`.
Verrou autonome SHA `641726b51724320f1948e602062c5ef2bcf63e48ee3008eeeeb187fae9768e34`.
Aucun SDK, natif rag3db, modèle, base, ingestion ou programme de projet téléchargé.

Gel Banc : 24 dépôts, 32382 fichiers, dont19941 pour les huit langages de code ;
les autres sont des façades spécialisées et des cohortes de limites. Sources,
licences, sélection et sous-effectifs sont documentés dans le rapport Banc
60d7aeb979 et ses compléments. Manifeste sélection02 SHA
`96c2e771fefc4cd0e204975feb8d1c73b1a66815cb1d3a1edc1b9002b94b644c`,
racines SHA `98766ce80d550ec1f2ae1a75ba1f0394ac35b0101f4bf819855c0bb4bca495c6`.
Vérification miroir SHA8894a37a0626071c2206ce23629f4248807bd7eb952c5e064e8b23f5d68436a7.
Les hashes de chaque fichier sont contrôlés de nouveau par le worker avant appel.

Allocation corrective C7-CORPUS-384-CORRECTIVE-CWD-16nov,1800s à luciepc,
32GiB, memory.high=max, quota5CPU, jobs8. Portée validation-5r_yy729,
PID2788069, échéance monotone1189088.491307695 ; PQS found=[] après poste,
adoption du target de session par la vraie garde. Conversion Err spécialisé :
1test passé/0échec/0ignoré, compilation9.74s ; ELF normal compilé en0.37s.
Ce sont des vérifications du banc, pas une qualification sémantique de tous les fichiers.

Preuves réelles : `~/.cache/codeparsers-notes/language-corpus-02b/` à luciepc.
L'essai précédent02 reste ROUGE, arrêté avant Cargo sur cwd de lanceur sous
source ; ses preuves et la correction sont au ticket parent et dans la recette.
L'essai02b utilise exactement384/1b depuis le répertoire de preuves : les commits
portable ultérieurs5f et877 ne font pas partie du binaire mesuré.

## Oracle C7 : attentes antérieures aux résultats

Attentes09bb6acd3f ; fichiers retrouvés inchangés dans FULL02, sans reparse.
Les lignes5002 et8546 ont été copiées sur portable avec leurs identités/hashes.
Ces cas prouvent des faits locaux ; aucune résolution inter-fichiers et aucun
rappel global ne se déduisent de deux fichiers choisis pour leur syntaxe.

| Source et construction | Résultat effectivement conservé |
|---|---|
| C# classe CSharpForEachCastDiagnosticAnalyzer | Class15–32, ancrage16 inclus ; positions byte et ASTvalide |
| C# base générique | Extends avec AbstractForEachCastDiagnosticAnalyzer et ses deux arguments |
| C# attribut | DiagnosticAnalyzer, propriétaire de classe, ligne15 |
| C# propriété | SyntaxFacts, membre de cette classe, ligne20, typeISyntaxFacts |
| C# GetSyntaxKinds | Method23–24, propriétaire correct, retourImmutableArray<SyntaxKind> |
| C# GetForEachInfo | Method26–31 et propriétaire présents ; retour et noms de paramètres erronés |
| C# imports | Sept using5–11, bindings distincts ; pas quatorze malgré représentation au fichier et au scope |
| C# namespace | Namespace nommé, ligne13, distinct de la classe |
| Go Format | type_alias46, distinct de Class/Interface ; int sous-jacent non exposé comme fait structuré |
| Go has | Method108, parentFormat, paramètre f2:Format, retourbool |
| Go mayBe / mayOnlyBe / mustNotBe | Method109/110/111, parentFormat, paramètresf2:Format |
| Go String | Method117–132, parentFormat, retourstring |
| Go import strings | Binding ligne7 ; aucun lien Join présumé |
| Go bloc const | La source49–106 est conservée dans un Module47–107 ; pas de faits Constant exposés |

C# GetForEachInfo porte un tuple source avec ITypeSymbol nullable, mais l'IR
renvoie le nom de la méthode comme return_type. Les paramètres semanticModel/node
sont nommés comme leurs types SemanticModel/CommonForEachStatementSyntax.
Défaut ticketé dans `2026-11-16-les-champs-des-methodes-csharp-confondent-noms-et-types.md` ;
aucun correctif appliqué durant la campagne. ASTvalide et captures présentes
n'empêchent donc pas des faits sémantiques faux.

Les modificateurs C# internal/sealed et la distinction des receveurs Go valeur /
pointeur ne sont **pas conservés par cet outil**. Ils ne sont pas déclarés absents
de l'IR : les receveurs figurent notamment dans la signature de production, omise
par le compact. Le jugement de ces dimensions reste indéterminé sur cet artefact.
Le type Go sous-jacent et les constantes sont des limites de faits structurés,
à distinguer d'une perte du texte source ou d'une perte après ingestion.

Preuves oracle : ligneC# SHA69eb6604fc30d9f151b233693b811f429ee1ecf407fd6994e4ea4a7659ba93ce ;
ligneGo SHA5c0875331adc028178b0d42384af6bf59909dc950665b379eabb0b8f9c3b25dd.
Les compteurs `reported_total_scopes=0` sont conservés séparément des5/75 scopes
récursifs rendus ; le zéro n'est pas interprété comme une absence de captures.

## Pannes isolées et reclassement

Le worker a avorté sur OpenSSL ssl/s3_lib.c par stack overflow ; ticket enfant
`2026-11-16-un-worker-deborde-sa-pile-sur-un-fichier-openssl.md`. La localisation
parsing/collecte du banc n'est pas encore établie. Aucun retry du cas.
Des échéances10s ont été observées sur Roslyn T_1247520.cs et RustAnalyzer
crates/hir/src/lib.rs ; elles bornent lecture/hash/initialisation/transport/appel,
pas seulement CPU du parseur. Ne pas appeler leur SIGKILL un OOM démontré.

L'annexe indépendante Bancb33c7d5d2c identifie584 fichiers générés upb,
329C/255Cpp, SHA8e673337e1361122167b4647d94ac02fcdd2563ee6bb31a8680e711bc8d6a18a.
Le gel reste intact. Le bilan publiera les strates originales et amendées ;
« hors de cette annexe » ne signifie pas « exhaustivement non généré ».

## Ce que cette passe ne prouve pas

AST sans erreur, présence de scopes et absence de panique sont trois dimensions
différentes. La résolution de projets et les relations du graphe n'ont pas été
jouées ; les relations CSS/documentaires ne les remplacent pas. HTML/Svelte sont
refusés comme chemins incomplets connus, pas déclarés réussis. Les façades sans
validité AST publique restent not_exposed, pas valides par défaut. Les fichiers
sans capture ne sont jugés fautifs qu'après lecture d'une attente sourcée.
L'oracle indépendant Banc109faits/12sources, publié905bf17f36 avant consultation
des résultats, complète les six autres langages ; jugement en cours, pas encore livré.
