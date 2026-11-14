# Du nom importé au paquet (conception, 14 novembre 2026)

*Session codeparsers, à la demande de Lucie relayée par l'orchestration (« ya ça aussi dans d'autres
langages ? faut qu'il prévoie d'avance », « et alias aussi non ? »). L'import pointé de Python
(`docs/codeparsers/tickets/2026-11-14-un-import-python-pointe-ne-produit-aucune-bibliotheque.md`) n'est pas
un correctif Python : c'est le premier cas d'une règle générale — partout, **le nom importé diffère du nom
que le manifeste déclare**. Cette page dit la mécanique proposée, **en données**, et la forme du fichier de
règles, à valider avant de la figer.*

## Les cas

| langue | nom importé | nom de paquet (manifeste) | ce qui le dit |
|---|---|---|---|
| Python | `requests.adapters` | `requests` | le premier segment |
| Python | `from . import x`, `from .m import f` | — | relatif : `project` |
| Python | `yaml` | `PyYAML` | **rien** dans le nom : table déclarée |
| JS/TS | `lodash/fp` | `lodash` | le premier segment |
| JS/TS | `@scope/pkg/sub` | `@scope/pkg` | deux segments quand le premier commence par `@` |
| JS/TS | `node:path`, `path` | — | `builtin` (le préfixe `node:`, la liste des modules de Node) |
| JS/TS | `@/components/x` | — | alias de résolution du dépôt (`tsconfig.json` `paths`/`baseUrl`, `resolve.alias` de vite/webpack) : `project` |
| Rust | `serde_json::Value` | `serde_json` ou `serde-json` | le premier segment, tiret et souligné échangés |
| Rust | `crate::`, `self::`, `super::` | — | `project` |
| Rust | `foo::x`, renommé dans Cargo.toml (`foo = { package = "bar" }`) | `bar` | alias de résolution du dépôt (`package = …`, `[patch]`) |
| Go | `github.com/a/b/c/d` | `github.com/a/b` | le **plus long préfixe** déclaré par `go.mod` ; `replace` vers un dossier : `project` |
| Go | `net/http` | — | `builtin` (premier élément sans point) |
| PHP | `Symfony\Component\HttpFoundation\Request` | `symfony/http-foundation` | la table PSR-4 de `composer.json` (vers `src/` : `project` ; vers `vendor/` : dépendance) |
| Ruby | `active_support` | `activesupport` | **rien** dans le nom : table déclarée |
| Java/Kotlin | `com.google.common.collect` | `guava` | **rien** dans le nom : table déclarée |
| C/C++ | `<boost/asio.hpp>`, `<fmt/format.h>` | `boost`, `fmt` (vcpkg, conan) | le premier dossier, parfois ; sinon table déclarée |

## Deux sortes d'alias

1. **L'alias du nom lié** (`import numpy as np`, `import { x as y } from "m"`, `use foo as bar`) : le nom
   local change, pas la bibliothèque. La provenance se juge sur **le nom importé** (la source de
   l'import), jamais sur l'alias ; la liaison porte déjà le nom lié. Un cas par langage au témoin, pour
   prouver qu'un alias ne fait pas une `inconnue`.
2. **L'alias de résolution**, qui ment sur la provenance : `paths`/`baseUrl` de `tsconfig.json`,
   `resolve.alias` de vite/webpack, `replace` de `go.mod`, `package = …` et `[patch]` de `Cargo.toml`, la
   table PSR-4 de `composer.json`. Ces tables **existent déjà dans le dépôt** : `rag3weaver::manifestes`
   les lit comme des manifestes de plus, et ce qu'elles déclarent passe **avant** la normalisation par le
   nom. Aucune table à la main pour ce que le dépôt déclare déjà.

## L'ordre de résolution, pour un import

1. **La forme locale** de la langue (relatif, `crate::`) → `project`.
2. **La bibliothèque standard** (le fichier de règles de la langue) → `builtin`.
3. **Les alias de résolution du dépôt** (lus par `rag3weaver::manifestes`) → `project`, ou le nom du paquet
   qu'ils désignent, jugé ensuite comme en 4.
4. **La règle de normalisation de la langue** : le nom importé donne une liste de noms de paquet candidats,
   dans l'ordre ; le premier qui est un paquet du dépôt → `project`, ou une dépendance déclarée →
   `dependency`.
5. **La table déclarée du dépôt** (le manifeste du backend rag3weaver : `nom importé → paquet`, par
   préfixe) → `dependency` ou `project`, pour Java, Ruby, C++, et ce qui ne se déduit pas.
6. Sinon **`inconnue`**, honnêtement ; `/etat` les compte.

## La forme proposée du fichier de règles (à valider)

Un fichier par langue, livré avec codeparsers (`data/imports/<langue>.toml`), **compilé comme défaut**
(`include_str!`, pour qu'un consommateur sans dossier de données ait les mêmes règles) et **remplaçable à
l'exécution** par une option du résolveur (`import_rules_dir`, que rag3weaver passe), pour une mise à jour
sans recompiler. TOML parce qu'il se relit comme un `Cargo.toml` ; le crate `toml` est déjà dans le
`Cargo.lock` de rag3weaver (une dépendance transitive), donc aucun paquet de plus. Le même fichier porte
les builtins (`docs/codeparsers/tickets/2026-11-14-les-listes-de-builtins-par-langage-sont-figees-dans-le-code.md`) :

```toml
# data/imports/python.toml
langue = "python"
extensions = ["py", "pyi"]

[builtin]
source = "sys.stdlib_module_names, Python 3.14.9"
noms = ["abc", "argparse", "os", "sys", …]

[local]
prefixes = ["."]

[nom_de_paquet]
separateur = "."      # requests.adapters → requests
garder = 1
variantes = ["tiret_souligne", "casse"]   # PyYAML ↔ pyyaml (ne suffit pas pour yaml : table déclarée)
```

```toml
# data/imports/javascript.toml (servi aussi à TypeScript)
langue = "javascript"
extensions = ["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"]

[builtin]
source = "les modules de Node"
prefixes = ["node:"]
noms = ["assert", "buffer", "fs", "path", …]

[local]
prefixes = [".", "/"]

[nom_de_paquet]
separateur = "/"
garder = 1
garder_si_le_premier_commence_par = { "@" = 2 }   # @scope/pkg/sub → @scope/pkg
```

```toml
# data/imports/rust.toml
langue = "rust"
extensions = ["rs"]

[builtin]
noms = ["std", "core", "alloc", "proc_macro", "test"]

[local]
prefixes = ["crate::", "self::", "super::"]

[nom_de_paquet]
separateur = "::"
garder = 1
variantes = ["tiret_souligne"]   # serde_json ↔ serde-json
```

```toml
# data/imports/go.toml
langue = "go"
extensions = ["go"]

[builtin]
regle = "premier_segment_sans_point"   # une règle de la chaîne d'outils, pas une liste

[nom_de_paquet]
regle = "plus_long_prefixe_declare"     # le module de go.mod
```

```toml
# data/imports/java.toml (et ruby.toml, cpp.toml : le nom ne dit rien)
langue = "java"
extensions = ["java", "kt"]

[nom_de_paquet]
regle = "aucune"   # seule la table déclarée du dépôt tranche
```

Les règles nommées (`premier_segment_sans_point`, `plus_long_prefixe_declare`, `aucune`) sont une **liste
fermée** dans le code ; les données choisissent parmi elles. Ce qui varie d'une langue à l'autre
(séparateur, nombre de segments, variantes, listes) est dans le fichier.

**La table déclarée du dépôt**, dans le manifeste du backend de code (rag3weaver), par préfixe :

```json
"imports": {
  "com.google.common": "guava",
  "active_support": "activesupport",
  "yaml": "PyYAML"
}
```

## Les lots

1. **L'import pointé** (Python) : le nom lié de `import a.b` est `a`, ce qui attache l'import au scope ;
   rouge d'abord (le témoin `codeparsers/tests/import_python_pointe.rs`).
2. **Le fichier de règles** et le résolveur qui le lit (Python, JS scope, Rust tiret au moins, rouge
   d'abord), les builtins y déménagent, les résolveurs morts de `src/import_resolution/` partent.
3. **Les alias de résolution** dans `rag3weaver::manifestes` (tsconfig, vite/webpack, go.mod `replace`,
   Cargo `package`/`[patch]`, composer PSR-4), et **la table déclarée** du manifeste du backend.
4. **`/etat`** compte les `inconnue` restantes.
