# Les champs des méthodes CSharp confondent noms et types

- **État** : ouvert — défauts de faits observés sur l'oracle C7, pas de correction dans le banc
- **Famille** : produit
- **Gravité** : un AST valide produit des noms de paramètres et un retour erronés
- **Atteignable en service** : oui, extraction CSharp du worker de production ; aucune ingestion testée ici
- **Touche rag3weaver** : oui, faits de codeparsers utilisés par la résolution
- **Pour** : codeparsers, lot séparé avec témoins minimaux
- **Signalé par** : oracle C7 posé avant résultats09bb6acd3f et campagne FULL02b séquence5002

## Preuve et source

Roslyn `c6dae7dd4f51cef1680c6b23ab614394d6989a69`,
`src/Analyzers/CSharp/Analyzers/ForEachCast/CSharpForEachCastDiagnosticAnalyzer.cs`,
SHA `23327a8f8df833f8b5e1941b06ff7e5cae4126f2592772adfe3196324edeab41`.
Parser1b35cfb (bibliothèque bd13385), parent384165760b ; ELFbff1cafb72e8c6662d2d595457b0ca6da4718686ae64b91ae1bde7a4e95565e2.
Ligne résultat copiée avant modification sur portable :
`~/.cache/codeparsers-notes/language-corpus-02b/oracle/csharp-row-5002.json`.
ASTvalid, aucune omission de l'inventaire compact.

La méthode GetForEachInfo, lignes26–31, porte réellement le retour tuple
`(CommonConversion conversion, ITypeSymbol? collectionElementType)`.
L'IR renvoie return_type="GetForEachInfo". Ses deux paramètres s'appellent
semanticModel et node ; l'IR les nomme SemanticModel et CommonForEachStatementSyntax,
soit leurs types, pourtant lus correctement dans le champ type.
Il ne s'agit ni d'un échec AST ni d'une absence de sérialisation de l'outil.

Dans `c_sharp_scope_extraction_parser.rs`, l'extraction du retour prend le premier
nœud d'une liste de genres qui omet tuple_type et inclut identifier. L'extraction
d'un nom de paramètre prend le premier identifier enfant, sans distinguer le type
et le nom par les champs de la grammaire. Ce mécanisme est cohérent avec les deux
erreurs observées ; un correctif devra vérifier les champs réels de cette grammaire.

## Témoin minimal à qualifier et fin attendue

Un fichier contenant `class C { (int x, string? y) F(Model m, Node n) => default; }`
suffit comme candidat minimal, sans compiler ni exécuter le projet public.
Observer le rouge sur les noms m/n et le retour tuple, puis corriger et conserver
les cas de types prédéfinis/génériques/qualifiés/nullable. Le cas réduit n'a pas
encore été joué. Une correction de compteurs ne corrige pas ces faits sémantiques.
Parent : `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md`.
