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
