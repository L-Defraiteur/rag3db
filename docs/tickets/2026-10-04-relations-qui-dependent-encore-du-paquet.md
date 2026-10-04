# 474 relations dépendent encore de la taille du paquet

- **État** : en cours (session codeparsers)
- **Gravité** : réponse fausse (le graphe dépend du découpage)
- **Atteignable en service** : oui (première indexation, synchronisation)
- **Touche rag3weaver** : oui

## Ce que c'est

Remesure du dépôt entier par la session embarquements, à corpus identique :
305 357 relations par paquets de 64, 304 883 à 512. La sortie de codeparsers
n'en est pas la cause : en fichier seul, un appel et 85 paquets y rendent
les mêmes 371 289 relations, une à une. L'écart naît dans
`code::analyze_in_project` ou après.

## Témoin

`tests/sonde_liste_analyse.rs::relations_selon_le_paquet` (branche rag3db
`liens`, à jouer) : relations et rendez-vous en attente seulement d'un côté,
par type et par extension.

## Pistes

Les `HAS_PARENT` de l'analyseur, `project_files` (Python seulement), le
repli des fermetures, `USES_LIBRARY` hors Rust.

## Pour le fermer

La sonde, puis le correctif, puis le test d'égalité des graphes de l'arbre
principal étendu au dépôt entier.
