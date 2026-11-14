# Un import Go ne produit aucune bibliothèque

- **État** : ouvert (14 novembre 2026)
- **Gravité** : manque de rappel : aucun nom importé d'un fichier Go n'a de bibliothèque, donc ni
  provenance ni arête `USES_LIBRARY`
- **Atteignable en service** : oui, sur tout dépôt Go indexé
- **Touche rag3weaver** : oui (l'index de code ; aucune ligne de rag3weaver à changer)
- **Famille** : produit

- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, au témoin de bout en bout de `go.mod` (lot de la provenance par
  langue) : l'analyse d'un `main.go` qui importe `github.com/Azure/azure-sdk-for-go/sdk/azcore/policy`
  rend `libraries = []`

## Ce que c'est

`GoScopeExtractionParser` appelle `BaseScopeExtractionParser::extract_structured_imports`, dont les
expressions régulières sont celles de JavaScript (`import X from '…'`, `require('…')`). Un import Go
(`import "fmt"`, `import ( alias "chemin" … )`) n'y correspond pas : aucune `ImportReference`, donc aucune
bibliothèque externe.

## Ce qu'il faudrait

Une extraction Go par l'arbre tree-sitter (`import_declaration` → `import_spec` : `path`, `name` pour
l'alias, `_` et `.`), le nom lié étant le dernier élément du chemin (ou l'alias), comme le fait la
résolution Go (`go_import_resolver.rs`). Témoin rouge d'abord : une bibliothèque par import, sa provenance
par `go.mod` (`plus_long_prefixe_declare`, casse gardée).

## Ce qui est déjà prêt

La règle Go (`data/imports/go.toml`), le plus long préfixe déclaré, la table par langue (casse gardée) et
la lecture de `go.mod` par rag3weaver : il ne manque que les imports.
