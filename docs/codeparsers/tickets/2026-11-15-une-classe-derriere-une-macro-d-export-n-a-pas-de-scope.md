# Une classe derrière une macro d'export n'a pas de scope (la grammaire C++)

- **État** : ouvert (15 novembre 2026) — grammaire
- **Famille** : produit
- **Gravité** : basse. Les membres de la classe sont relevés ; le scope de la classe manque, et ses
  membres ont le parent d'au-dessus au lieu d'elle.
- **Atteignable en service** : oui, sur un fichier C++ où une macro seule sur sa ligne précède une classe
  (`FMT_CLASS_API\nclass FMT_API format_error : public std::runtime_error { … }`, `fmt/format.h` l. 956)
- **Touche rag3weaver** : oui, par codeparsers (l'extraction des scopes C++)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, à la fermeture du ticket de `fmt/format.h`

## Ce qu'on voit

tree-sitter-cpp 0.23.4 lit la macro comme le type d'une `function_definition`, et `class FMT_API
format_error : public std::runtime_error` comme un `qualified_identifier` plein d'`ERROR`. codeparsers
n'en fait plus une fonction anonyme (correctif du même jour), mais n'a pas de classe à extraire.

## Ce qu'il faudrait

Soit la grammaire (une macro seule sur sa ligne devant une déclaration), à signaler en amont de
tree-sitter-cpp ; soit, dans codeparsers, lire le texte de la tête (`class <nom>` après une macro) pour en
tirer la classe et son corps. La seconde voie est une lecture de texte sous un arbre fautif : à peser.
