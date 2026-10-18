# La cible clean efface la grammaire Cypher, suivie par git, et le bâti suivant ne la régénère pas

- **État** : ouvert (18 octobre 2026, seconde session cœur C++, à la demande de l'orchestration).
- **Gravité** : blocage du bâti. Après un `clean`, rien ne compile tant qu'on ne restaure pas les
  fichiers à la main. Rien n'est perdu, puisque git a les fichiers.
- **Atteignable en service** : non (bâti du moteur seulement).
- **Touche rag3weaver** : non, sauf qu'un bâti de la lib commune ou de l'extension vector après un
  `clean` tombe de la même façon.

## Ce que c'est

`cmake --build <dir> --target clean` efface quatre fichiers suivis par git :

- `third_party/antlr4_cypher/cypher_lexer.cpp`
- `third_party/antlr4_cypher/cypher_parser.cpp`
- `third_party/antlr4_cypher/include/cypher_lexer.h`
- `third_party/antlr4_cypher/include/cypher_parser.h`

Le bâti suivant ne les régénère pas. Ninja s'arrête sur :

```
FAILED: [code=1] third_party/antlr4_cypher/CMakeFiles/antlr4_cypher.dir/cypher_lexer.cpp.o
cc1plus: fatal error: …/third_party/antlr4_cypher/cypher_lexer.cpp : Aucun fichier ou dossier de ce nom
```

Il était au 176e objet sur 1 200. Toutes les cibles de test manquent ensuite.

On l'a vu le 18 octobre sur luciepc. Le rejeu de la rétention sur E10 (87a362e30) demandait un
bâti neuf, parce qu'E10 ajoute des méthodes virtuelles à `Table`, `NodeGroup` et
`ChunkedNodeGroup`. Le `clean` a cassé le bâti, et la liste est tombée rouge sans avoir rien
testé. Le push, lui, est resté dans son `if grep -qx "verdict VERT"`.

## Recette

Dans un worktree propre, avec un dossier de bâti configuré par défaut
(`AUTO_UPDATE_GRAMMAR=ON`) :

```
cmake --build build/moteur --target clean
git status --short third_party        # D des quatre fichiers ci-dessus
cmake --build build/moteur -j 8       # FAILED sur cypher_lexer.cpp
```

## La cause, lue dans le code

L'enchaînement n'a pas été rejoué étape par étape.

- `CMakeLists.txt:306` : `option(AUTO_UPDATE_GRAMMAR … TRUE)`. L'option est active par défaut.
- `third_party/antlr4_cypher/CMakeLists.txt` : sous cette option, un `add_custom_command`
  déclare les quatre fichiers comme `OUTPUT`, **dans l'arbre des sources**
  (`${CMAKE_CURRENT_SOURCE_DIR}/…`), avec `DEPENDS src/antlr4/Cypher.g4` et
  `scripts/antlr4/generate_grammar.cmake`. Pour CMake et ninja, ils deviennent des produits du
  bâti, que `clean` efface.
- `scripts/antlr4/generate_grammar.cmake` sort tout de suite (`return()`) quand le hachage de
  `Cypher.g4` et de `keywords.txt` égale celui de `scripts/antlr4/hash.md5` (suivi par git). Le
  but est qu'un bâti neuf n'exige pas Java. Après un `clean`, ninja relance la commande, puisque
  les sorties manquent, mais la grammaire n'a pas changé : le script sort sans rien écrire, et la
  compilation trouve les fichiers absents.

La commande promet donc des sorties qu'elle ne produit pas quand rien n'a changé.

## Pour le fermer

Deux formes, à trancher par le cœur.

1. **Ne plus déclarer ces fichiers en `OUTPUT`.** La grammaire générée est un fichier source
   suivi, et la régénération resterait une action explicite : une cible `update_grammar`
   (`add_custom_target`), lancée à la main ou par un contrôle de la CI qui compare
   `hash.md5`. C'est la forme la plus simple : `clean` ne touche plus l'arbre des sources.
2. **Régénérer vraiment quand les sorties manquent.** `generate_grammar.cmake` ne sortirait tôt
   que si les quatre fichiers existent. Sinon, il régénère. Cela exige Java et le téléchargement
   d'`antlr4.jar` après chaque `clean`, ce que le commentaire du script veut éviter. Et un
   `clean` réécrirait quand même des fichiers suivis.

Un autre défaut va dans le même sens : une commande de bâti qui écrit dans l'arbre des sources
rend aussi le worktree sale pour git.

**Le témoin**, à écrire : un contrôle de la CI ou un script qui fait `clean` sur un bâti
configuré, puis exige `git status --porcelain --untracked-files=no` vide.

**En attendant**, après tout `--target clean`, faire
`git checkout -- third_party/antlr4_cypher` avant de bâtir.
