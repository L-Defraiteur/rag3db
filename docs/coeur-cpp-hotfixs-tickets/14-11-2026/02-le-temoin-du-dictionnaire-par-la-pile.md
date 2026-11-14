# Le témoin du dictionnaire, posé par la file en pile (14 novembre 2026)

Session hotfixs. Le lot : trois cas de non-régression des chaînes après un ADD COLUMN, dans
`test/transaction/concurrence/string_rewrite_test.cpp`, sur la branche `chaine-au-point-de-reprise`.
Ils chassaient le plantage du core 3980144 (`DictionaryChunk::appendString` au point de reprise demandé
par un COMMIT) **et ne l'ont pas reproduit** : 4 sur 4 verts sous ASAN et en Release. Le détail est dans
le ticket du cœur
`docs/coeur-cpp/tickets/2026-11-14-un-point-de-reprise-automatique-meurt-dans-le-dictionnaire-d-une-colonne-de-chaines.md`,
et le ticket neuf de l'isolement du catalogue est
`docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-14-un-add-column-passe-sous-une-transaction-d-ecriture-ouverte.md`.

## Le chemin par la pile

| pas | ce qui s'est fait |
|---|---|
| rang | 7, derrière 96 (`lot-96-pile`, tête 84f949d2c) ; donné à ED (rang 8) |
| pile | `chaine-au-point-de-reprise-pile`, tête **cabcc8640**, rebasée sans conflit sur 84f949d2c. La pile de 96 ne change aucun fichier de `src/` ni de `test/` |
| rejeu | `StringRewrite*` en Release à luciepc (`rag3db-dbg/build/moteur`, bâti incrémental de `concurrence_test`) : 4 sur 4, 0 FAILED, 0 SKIPPED |
| push | au « à toi » de c7 : `PILE_AVANT=84f949d2c… poste push hotfixs -- push_garde.sh rag3db-gabarits hotfixs-chaine-pile`, dans `if grep -qx VERT` |
| garde | « mode pile : tête d'avant 84f949d2c, commit testé cabcc8640 », puis « pile tenue » ; trois contrôles propres ; **master 42c84cb77** |

## Le journal d'un lot C++ seul pour la garde

La garde exige que ses journaux nomment **une seule lib du magasin** (elle lit `temoins.log`,
`e2e_lectures.log`, `e2e_code.log`, `check_features.log`, `preuve.log`, `lib.log`, `navigateur.log`). Un
lot C++ de test seul n'est pas lié par la lib épinglée : son test est bâti depuis l'arbre. Son
`temoins.log` porte donc :
- la sortie réelle du rejeu ;
- une ligne qui nomme la lib courante du poste, ici
  `rag3db-build-0c41f9fe6731a203b330ce975c4e26e9089bf807`, en disant que le test est bâti depuis
  l'arbre et non contre cette lib.

À côté : `resume.log` (« testé <commit> », puis `VERT`) et `base` (la tête d'avant). Ce journal est dans
`~/.cache/rag3weaver-build/hotfixs-chaine-pile/` au portable.

## Deux lignes de la garde, sans rapport avec le lot

- « CI de master : ROUGE rag3weaver, CI rouge depuis 2026-11-14 15:30 UTC ».
- « garde en retard sur le dépôt » (copie exécutée `a58a649f1b2cc7cb`, garde du dépôt
  `c57a3fb1aad59172`). La copie exécutée fait autorité.
