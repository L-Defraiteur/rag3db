# La normalisation des noms de paquet est la même pour toutes les langues côté manifestes

- **État** : ouvert (14 novembre 2026)
- **Gravité** : imprécision latente (aucune dépendance Go n'arrive encore : rag3weaver ne lit pas go.mod)
- **Atteignable en service** : non tant que go.mod n'est pas lu
- **Touche rag3weaver** : oui (`manifestes.rs` devra passer les dépendances par langue)
- **Famille** : produit

- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, au passage des règles d'import de Go en données (`data/imports/go.toml`, codeparsers 00c24c2)


## Le constat

Le résolveur de codeparsers normalise les dépendances déclarées (`declared_dependencies`) et les paquets
du dépôt (`project_packages`) par `provenance::normaliser` : minuscules, `-` et `_` confondus, quelle que soit
la langue. Le côté import suit les variantes de sa langue (`data/imports/<langue>.toml`). Pour que les deux
côtés se comparent, Go déclare `variantes = ["tiret_souligne", "casse"]`, alors que les chemins de module Go
distinguent la casse (`github.com/Azure/azure-sdk-for-go` et `github.com/azure/…` sont deux modules).

## Ce qu'il faudrait

Normaliser chaque nom déclaré par les variantes **de la langue de son manifeste** (go.mod → go, Cargo.lock →
rust, package.json → javascript, pyproject/requirements → python), ce qui demande que rag3weaver passe les
dépendances par langue (`declared_dependencies: HashMap<langue, Vec<String>>` ou un champ par manifeste).
Go pourrait alors ne garder aucune variante.

## Pourquoi ce n'est pas fait

rag3weaver ne lit pas encore go.mod (`manifestes.rs` : Cargo.lock, package.json, pyproject.toml,
requirements.txt, vcpkg.json) : aucune dépendance Go n'arrive aujourd'hui, la collision de casse ne peut pas
se produire. Le ticket se ferme avec la lecture de go.mod.
