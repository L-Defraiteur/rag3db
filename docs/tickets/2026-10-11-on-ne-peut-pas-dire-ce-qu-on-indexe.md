# On ne peut pas dire ce qu'on indexe : ni racines multiples, ni chemins inclus

- **État** : ouvert (11 octobre 2026) — trouvé par le premier index réel de rag3db (session codeparsers)
- **Gravité** : friction et coût (l'index prend tout le dépôt : temps, tampon, bruit dans les réponses)
- **Atteignable en service** : oui, pour tout dépôt dont on ne veut qu'une partie
- **Touche rag3weaver** : oui (`workspace` du manifeste, source de fichiers)

## Ce que c'est

L'orchestration demandait d'indexer `src/` et `test/` de rag3db. Ce n'est pas exprimable :
- `workspace.root` est une seule racine ;
- la source refuse les liens symboliques (« lien symbolique hors du projet », une garde voulue),
  donc un dossier de liens vers `src/` et `test/` ne marche pas ;
- `generated.dirs` exclut un nom de dossier **partout** : exclure `extension` aurait aussi
  retiré `src/extension/`.

Le dépôt entier a donc été indexé : 7 310 fichiers (62,3 Mo ; 4 177 code, 3 133 texte), dont
la documentation, les extensions et les outils, et un tampon estimé à 8 Gio sur 8.

## Ce qu'il faut pour le fermer

Dans `workspace`, au choix :
- `include` : une liste de chemins relatifs à la racine (seuls indexés) ;
- ou `roots` : plusieurs racines dans un même backend, chacune avec son préfixe ;
- ou des exclusions par chemin ancré (`/extension`) en plus des noms de dossier.
`estimate` dit ce qui est retenu par chacune.
