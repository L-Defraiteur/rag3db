# replace_text de l'atelier ne compte pas les occurrences qui se chevauchent

- **État** : ouvert (13 novembre 2026), pour archi.
- **Famille** : produit
- **Gravité** : réponse fausse, mineure — un remplacement dit « une seule occurrence » quand le texte en contient deux qui se chevauchent
- **Atteignable en service** : oui, à tout `replace_text` dont l'ancien texte peut se recouvrir lui-même (`aa` dans `aaa`, une ligne répétée)
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/daemon_host/atelier_fichiers.rs`, le compte des occurrences 525-527)
- **Pour** : rag3weaver archi (l'atelier qui lit et écrit, son auteur), lot d'après avec les six autres tickets de la relecture
- **Signalé par** : everything declarative, relecture de l'atelier sur origin/master 73c7f15e1 (13 novembre 2026), mis en ticket à la demande d'archi ; lignes de origin/master à cette date

## Ce que c'est

`replace_text` exige que l'ancien texte apparaisse exactement une fois. Le compte passe par une
recherche sans chevauchement (`matches`) : `"aa"` dans `"aaa"` compte 1. Le remplacement part
alors sur la première position, alors que l'ancien texte était ambigu.

## Le témoin à écrire

`replace_text` de `aa` dans un fichier `aaa` est refusé comme ambigu (deux positions), et
le fichier reste intact.

## Ce qui le fermerait

Compter les positions de départ (`match_indices` sur chaque décalage, ou une boucle
`find` qui avance d'un caractère) ; le témoin vert.
