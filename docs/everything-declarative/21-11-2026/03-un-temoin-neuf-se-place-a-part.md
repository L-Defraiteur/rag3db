# Un témoin neuf se place à part, pas devant un voisin déjà pris

21 novembre 2026, nuit, session everything declarative. Une règle tirée de deux reposes rouges, à la demande de c7.

## Ce qui s'est passé

Cette nuit, trois lots d'ED ont chacun ajouté un témoin dans le module de tests de `src/backend.rs` :
`options` facultatif, le nom exact par le nom, et `token-count`. Tous les trois étaient insérés **devant le même test**
(`a_short_variable_has_no_chunk_and_keeps_its_full_text`), pris comme ancre. Chaque lot était juste seul. Mais au
pousseur, celui qui atterrissait le second heurtait le premier sur ces lignes, et sa repose était rouge :
`ed-exact-name-by-name-2` à 4 h 02, puis `ed-token-count`, repris avant son dépôt. Deux reprises, deux attestations
rejouées, un lot reporté à l'hôte suivant.

Les conflits ne portaient que sur des ajouts. Les résoudre était sûr, mais chaque résolution coûte une passe.

## La règle

**Un témoin neuf se place à un endroit qu'aucun autre lot en vol n'a pris.** En pratique, au choix :
- **à la fin du module de tests** du fichier, ou dans un module de tests propre au lot (`mod <sujet>_tests` en bas du
  fichier, ou un fichier à part) ;
- **à côté des témoins de son propre sujet**, pas devant un test sans rapport, pris comme ancre commode ;
- **jamais devant le même voisin que le lot d'avant de la même session** : une session qui enchaîne des lots dans un
  même fichier les écarte d'elle-même.

Le même principe vaut pour les autres lieux où tout le monde ajoute une ligne : la tête de `docs/nouveautes.md` (là,
le conflit est inévitable et se résout en gardant les deux lignes, la plus récente en tête) et les listes comptées (les
nœuds fournis, les nœuds d'outil), dont le compte se relit après toute repose.

## Comment le vérifier avant un dépôt

`git diff origin/master...HEAD -- <fichier>` montre où le lot insère. Si l'ancre est un test qu'un autre lot en file
touche aussi, déplacer le témoin avant de déposer.

## Quand le conflit coupe dans des corps de fonctions : repartir du fichier de master

Une leçon de la même nuit (4 h 0x). Pour reprendre deux lots sur master, j'ai résolu leurs conflits en gardant les
deux côtés. Mais les témoins en conflit commençaient par les mêmes lignes de fixture
(`let tools = …`, `fixture_code_avec(…)`), et git a découpé ses blocs **au milieu des corps** des fonctions. Garder les
deux côtés a laissé une accolade ouverte, et la compilation a échoué sur les deux branches. C'était avant tout dépôt,
mais la passe était perdue et le lot a manqué l'hôte du matin.

**La règle** : garder les deux côtés ne vaut que pour des ajouts **entiers**, des lignes de nouveautés ou des items
d'une liste. Quand un bloc de conflit commence ou finit au milieu d'une fonction, on ne le résout pas en place : on
repart du fichier de master et on y réapplique les changements du lot, à la main ou par un script, en plaçant chaque
témoin en fin de module. Puis on vérifie que le fichier compile avant toute passe longue : `cargo check` sur la cible
de test, ou au moins l'équilibre des accolades comparé à celui de master.
