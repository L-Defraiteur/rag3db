# Les mots vides de C++, C# et Go copient ceux de JavaScript

- **État** : corrigé (19 novembre 2026) — les mots vides et les noms intégrés des huit langages viennent d'une seule source, `data/stop_words/` (pointeur codeparsers b67ad75, branche rag3db `codeparsers/stop-words-locals`)
- **Famille** : produit
- **Gravité** : basse. Un nom du projet qui est un mot-clé de JavaScript (`of`, `function`, `undefined`,
  `typeof`, `from`, `export`…) n'est pas relevé en C++, en C# ni en Go : ses appels ne sont reliés à rien.
- **Atteignable en service** : rarement, un nom qui serait un mot-clé de JavaScript
- **Touche rag3weaver** : oui, par codeparsers (les références des scopes C++, C# et Go)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, au lot des mots vides de Python et de C

## Constat

`CPP_STOP_WORDS` (`cpp_scope_extraction_parser.rs`), `CSHARP_STOP_WORDS` et `GO_STOP_WORDS` commencent par la
liste de JavaScript (`const`, `let`, `var`, `function`, `undefined`, `typeof`, `instanceof`, `in`, `of`…), puis
ajoutent les mots du langage. Les noms intégrés de Python (`BUILTIN_IDENTIFIERS` de
`python_scope_extraction_parser.rs`) sont ceux de JavaScript (`Number`, `Map`, `Promise`, `Symbol`, `console`) :
une classe Python nommée `Symbol` ou `Map` n'est jamais relevée.

## Ce qu'il faudrait

Le même geste que pour Rust, Python et C : des données générées depuis le langage (`data/stop_words/generate.py`),
la grammaire tree-sitter pour C++, C# et Go ; pour les noms intégrés de Python, le module `builtins` de CPython.
Un témoin rouge par langue, puis la sonde de marque sur un corpus de chaque langue.

## Un seul lot, tous les langages (Lucie, 19 novembre 2026, par c7)

Règle permanente : un lot prend tous les langages d'un coup et vise du code commun. Ce ticket se ferme donc en **un
seul lot** qui couvre les huit langages du routeur (TypeScript, JavaScript, Python, Rust, C, C++, C#, Go) :

- une seule source pour les mots vides et les noms intégrés : `data/stop_words/<langue>.toml` produits par le même
  `generate.py`, lu par `standard::stop_words_of` pour toutes les langues ; plus aucune constante
  `*_STOP_WORDS` ni `BUILTIN_IDENTIFIERS` copiée par grammaire ;
- la recette par langue réduite à sa table : les mots réservés de chaque grammaire tree-sitter (comme C), les
  mots-clés de CPython et son module `builtins` pour Python ; TypeScript et JavaScript gardent leurs propres mots ;
- un témoin rouge par langue dans un seul fichier, et la sonde de marque sur un corpus de chaque langue (au moins
  rag3weaver/src, le moteur C++, un corpus Python et un corpus TypeScript/JavaScript).

## Fermé

19 novembre 2026, session codeparsers. Un seul lot pour tous les langages : `data/stop_words/generate.py` écrit un
fichier par langage (Rust, Python, C, C++, C#, Go, et JavaScript que TypeScript partage), avec ses mots vides
(`names`, tirés de la grammaire tree-sitter ou des mots-clés du langage) et ses noms intégrés (`builtins` : le module
`builtins` de Python, le `builtin.go` de Go 1.27.4, des listes à la main sans les noms de JavaScript). Le parseur de
base les lit pour tous les langages ; les douze listes copiées par grammaire sont retirées.

Témoin `tests/stop_words_every_language.rs` (codeparsers) : rouge sur 828f855 (`of`, `function` en C++, C# et Go ;
`Symbol`, `Map` en C# et Python), vert sur la tête. Suite codeparsers 75 lots sur 75. Sonde de marque en release
contre 828f855 : rag3weaver/src, `tools/python_api` et l'UI inchangés ; sur le moteur, +81 arêtes vers
`namespace function`, dont 79 fausses — elles ont révélé le défaut des locales C et C++
(`2026-11-19-une-locale-c-ou-cpp-se-relie-a-un-scope-homonyme.md`), corrigé dans la même pile.
