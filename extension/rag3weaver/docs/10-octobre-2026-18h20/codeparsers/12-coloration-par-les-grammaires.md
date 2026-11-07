# La coloration du code par les grammaires tree-sitter (conception)

*codeparsers, 7 novembre 2026, relue et acceptée par c7 le même soir, à la demande de c7 (ticket 14f4e7274, le code coloré du chat de
hooks-essai). Décision : la forme générale est la coloration par les grammaires que codeparsers porte
déjà, pour que l'index, l'IDE et le terminal aient les mêmes couleurs ; highlight.js dans la page n'est
que le repli. Cette page dit ce qui existe et ce qui manque, avant le code.*

## Ce qui existe

Chaque crate de grammaire que codeparsers dépend livre sa requête de coloration
(`queries/highlights.scm`), exposée en constante Rust :

| langue | crate | constante | seule ? |
|---|---|---|---|
| Rust | tree-sitter-rust 0.24.2 | `HIGHLIGHTS_QUERY` | oui |
| Python | tree-sitter-python 0.25.0 | `HIGHLIGHTS_QUERY` | oui |
| Go | tree-sitter-go 0.25.0 | `HIGHLIGHTS_QUERY` | oui |
| C# | tree-sitter-c-sharp 0.23.5 | `HIGHLIGHTS_QUERY` | oui |
| C | tree-sitter-c 0.24.2 | `HIGHLIGHT_QUERY` | oui |
| C++ | tree-sitter-cpp 0.23.4 | `HIGHLIGHT_QUERY` | **non** : ne couvre que le propre du C++ ; se lit **après celle de C** (déjà portée) |
| TypeScript, TSX | tree-sitter-typescript 0.23.2 | `HIGHLIGHTS_QUERY` | **non** : se lit **après celle de JavaScript**, que codeparsers ne dépend pas |
| Bash | tree-sitter-bash 0.25.1 | `HIGHLIGHT_QUERY` | oui |
| CSS | tree-sitter-css 0.25.0 | `HIGHLIGHTS_QUERY` | oui |
| HTML | tree-sitter-html 0.23.2 | `HIGHLIGHTS_QUERY` | oui (ses injections, script et style, plus tard) |

Les quatre langues demandées (Rust, C++, TypeScript, Python) sont couvertes, à deux ajouts près.

## Ce qui manque

1. **La requête de JavaScript**, pour TypeScript : ajouter la dépendance `tree-sitter-javascript`
   (la même famille de versions que tree-sitter-typescript 0.23) pour sa seule constante, ou
   embarquer son `highlights.scm` ; la dépendance est plus simple à suivre.
2. **La concaténation** : C++ = C puis C++ ; TypeScript = JavaScript puis TypeScript (l'ordre de
   l'éditeur de référence, `inherits`).
3. **Le moteur de coloration.** `tree-sitter-highlight` (le crate officiel) n'est pas encore une
   dépendance, et sa version doit suivre celle de tree-sitter (0.27 chez nous) ; si elle n'existe
   pas, une fonction à nous suffit, sans injections ni locales au premier pas :
   un `Query` par langue (compilé une fois), un `QueryCursor` sur l'arbre, les captures triées par
   position, et pour un recouvrement, **la première capture d'un motif plus ancien l'emporte** (la
   règle de tree-sitter-highlight : les motifs du début du fichier sont les plus précis).
4. **Les sortes.** Les noms de capture des requêtes (`@function.method`, `@type.builtin`,
   `@string.special`…) se ramènent à une liste fermée par leur premier segment :
   `keyword`, `type`, `function`, `string`, `comment`, `number`, `constant`, `variable`, `property`,
   `operator`, `punctuation`, `attribute`, `label`, `module` ; un nom inconnu tombe dans `other`, et
   la liste se dit dans la réponse.

## La forme

- **codeparsers** : `highlight(texte, langue) -> Vec<(usize, usize, Sorte)>`, les décalages en
  **octets** du texte UTF-8 ; une langue sans requête rend `None` (le repli de la page, dit).
- **rag3weaver** : l'hôte l'expose par une **route** `POST /highlight {text, language}` (décidé par
  c7 : c'est l'écran qui colore, pas l'agent ; un outil viendra si un graphe en a besoin). Il rend
  **les deux décalages**, `[[debut, fin, sorte, debut_utf16, fin_utf16], …]` : les octets UTF-8 pour
  le terminal, les unités UTF-16 pour la page, comme les surlignages de la recherche. La page ne
  convertit rien. Deux portes, une seule fonction de l'hôte derrière, qui n'ouvre aucun backend
  (hooks-essai) : `POST /ide/highlight` sous le cookie de l'IDE pour la page, `POST /highlight` sous
  le jeton porteur pour le terminal de tty.
- **Les classes** de la page et de tty suivent les sortes, une à une : `hl-keyword`, `hl-type`,
  `hl-function`, `hl-string`, `hl-comment`, `hl-number`, `hl-constant`, `hl-variable`, `hl-property`,
  `hl-operator`, `hl-punctuation`, `hl-attribute`, `hl-label`, `hl-module`, `hl-other`.
- **Le chat et tty** l'habillent (hooks-essai, tty) ; highlight.js reste le repli quand l'hôte ne
  répond pas ou qu'une langue manque.

## Les témoins

- Un par langue (Rust, C++, TypeScript, Python), sur un extrait du dépôt : quelques spans attendus
  (un mot-clé, un type, un appel, une chaîne, un commentaire), pas l'égalité de toute la liste.
- C++ : un mot-clé du C (`return`) et un du C++ (`template`) sont colorés, preuve de la
  concaténation ; TypeScript : un mot-clé de JavaScript (`const`) et une annotation de type.
- Un texte qui ne se lit pas (une erreur de syntaxe) se colore quand même là où l'arbre tient.

## L'ordre

1. Cette page, relue par c7 et hooks-essai.
2. codeparsers : la dépendance JavaScript, la fonction et ses témoins, sur une branche.
3. rag3weaver : le pointeur, et la route ou l'outil de l'hôte, en lot à part dans la file.
