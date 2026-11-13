# Le terminal sous Windows et macOS : à jouer quand une machine existe

- **État** : ouvert. Le plan de test est écrit, et rien n'est joué faute de machine (session tty, 13 nov. 2026).
- **Famille** : produit
- **Gravité** : inconnue (aucune réponse fausse connue, rien n'est vérifié sur ces systèmes).
- **Atteignable en service** : oui, pour qui lance `rag3weaver-tty` sous Windows ou macOS.
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`).

## Ce que c'est

Lucie veut que le terminal tourne partout (7 nov. 2026). Le backend crossterm le promet, mais rien n'a été bâti ni joué sous Windows ou macOS : aucun des deux postes n'en est un.

## La recette et le témoin

Le plan est dans `docs/tty/13-11-2026/04-la-mesure-a-travers-ssh.md`, § « Le plan de test pour Windows et macOS ». Le témoin est le harnais `extension/rag3weaver/tty/mesures/mesure.py`, joué depuis la machine vers luciepc puis en local par tunnel, avec une capture d'écran par terminal pour les glyphes.

## Ce qu'il faut pour le fermer

Une machine Windows et une machine macOS, le plan joué en entier, les chiffres et les captures rangés à côté de la page, et un ticket par défaut trouvé.
