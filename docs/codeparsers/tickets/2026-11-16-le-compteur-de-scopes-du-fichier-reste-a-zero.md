# Le compteur de scopes du fichier reste à zéro

- **État** : ouvert — incohérence observée sur les résultats de worker, contrat exact à qualifier avant correction
- **Famille** : produit
- **Gravité** : métadonnée de comptage ambiguë ; les faits extraits sont présents
- **Atteignable en service** : via ScopeFileAnalysis des workers ; effet des consommateurs à vérifier
- **Touche rag3weaver** : potentiellement, dépendance codeparsers ; le banc dérive un compte séparé
- **Pour** : codeparsers
- **Signalé par** : FULL02b oracle CSharp/Go, source384165760b/parser1b35cfb

Sur la ligne CSharp5002, total_scopes=0 mais5scopes IR sont rendus ; sur la ligne
Go8546, total_scopes=0 et75scopes récursifs sont rendus. Les identités/hashes et
artefacts sont au rapport03-rapport-premiere-campagne.md, aucun reparse.
ProjectParser calcule aussi un agrégat à partir de scopes.len : vérifier si le
contrat de fichier attend les seuls scopes de tête ou tous les descendants.
Ne pas mélanger ces deux granularités pour corriger le seul nombre affiché.

Le banc conserve le compteur déclaré et son décompte dérivé ; aucun zéro n'est
interprété comme absence de parsing/capture. Témoin minimal à écrire et jouer :
parse_file sur un fichier à une fonction, puis un fichier avec enfants ; vérifier
les garanties du compteur public et ses consommateurs. Aucun témoin réduit joué
ni correction annoncée. La référence32382fichiers reste figée.
Suite de la référence close `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md`.

## Relu le 18 novembre (session codeparsers)

Confirmé par lecture de codeparsers bd13385 : les extracteurs C, C#, C++, Go et Rust posent `total_scopes: 0` en dur (`c_scope_extraction_parser.rs:151`, `c_sharp_…:181`, `cpp_…:173`, `go_…:159`, `rust_…:175`), alors que Python (`python_…:100`) et l'extracteur de base (`base_…:330`) prennent `scopes.len()`. Le défaut est donc par langage, pas par fichier. rag3weaver ne lit pas ce champ (aucune occurrence sous `src/`) ; `ProjectParser` refait sa somme par `scopes.len()`. Gravité faible.

Le parent `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md` est clos et reste, avec le rapport de la campagne, sur la branche `cx-codeparsers-language-corpus` (non portée : son pointeur 1b35cfb, un outil du banc, ne va pas sur master).
