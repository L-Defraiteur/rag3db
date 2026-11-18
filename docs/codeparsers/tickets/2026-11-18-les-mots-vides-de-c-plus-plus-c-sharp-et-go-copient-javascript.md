# Les mots vides de C++, C# et Go copient ceux de JavaScript

- **État** : ouvert (18 novembre 2026)
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
