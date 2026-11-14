# Un import Go ne produit aucune bibliothèque

- **État** : corrigé (14 novembre 2026) — codeparsers 6ba8478 (`imports-go`) : les imports Go se lisent sur l'arbre ; témoin `tests/imports_go.rs` (codeparsers) et `manifestes::tests::go_l_analyse_garde_la_casse_du_module_requis` (rag3weaver), rouges sous cf90e1c
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

## La correction

`GoScopeExtractionParser::extract_go_imports` lit chaque `import_spec` (seul ou en bloc) : `source` = le
chemin, `imported` = le nom lié (l'alias, sinon le dernier élément ; `/vN` renvoie à l'élément d'avant,
`gopkg.in/yaml.v3` donne `yaml` ; un élément qui n'est pas un identifiant Go ne lie rien), `_` un
`SideEffect`, `.` sans nom lié. Corrigé dans le lot qui l'a vu (la provenance par langue) : `fmt` est
builtin, `github.com/Azure/azure-sdk-for-go/sdk/azcore/policy` est `dependency` par `go.mod`, le même chemin
sous une autre casse est `inconnue`.
