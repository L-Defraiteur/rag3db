# Un chemin vers une réexportation reste « par le nom »

- **État** : corrigé — codeparsers `77c8ea6` (le drapeau) et le suivi au rendez-vous (`code.rs`, 11 oct. 2026) ; témoin `e2e_usages::un_chemin_vers_une_reexportation_n_est_pas_par_le_nom`
- **Gravité** : réponse fausse (une arête sûre marquée « nom », donc tue par le filtre)
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (rendez-vous, Liens, impact)

## Ce que c'est

`crate::dataflow::DataflowEvent` : le chemin désigne dataflow/mod.rs, le type
est défini dans dataflow/runtime.rs et réexporté par `pub use
runtime::DataflowEvent`. La règle « le chemin désigne le fichier du
définisseur » ne conclut rien, l'arête reste « nom » : 1 982 arêtes « nom par
chemin de module » sur le code de rag3weaver (sonde `tests/sonde_marques.rs`,
4 octobre), en partie celles-ci.

## Décision (orchestration, 4 octobre)

Pas de « le module désigne son dossier » (une devinette). Le chemin désigne le
fichier du module ; si le nom n'y est pas défini, suivre les réexportations de
CE fichier — `pub use runtime::DataflowEvent`, `pub use runtime::*` (le nom
cherché dans ce module nommé seulement) ; `export … from` en TypeScript,
`__init__.py` en Python — sur un nombre borné de sauts. Marque « import ».
Sinon l'arête reste « nom ». Mesurée à part.

## Ce qu'il faut (estimation : environ un jour)

1. **codeparsers** (une heure) : `ImportReference` dit qu'un import est une
   réexportation (`pub use`, `export … from`, import d'un `__init__.py`).
2. **code.rs** (arbre principal, changement de schéma) : garder les
   réexportations d'un fichier d'un lot à l'autre — le fichier qui réexporte
   n'est pas forcément dans le lot de la mention. Proposition : un rendez-vous
   `MENTIONS` de genre `REEXPORTS` du scope de fichier vers le `Symbol` du nom
   (module source en propriété), que la matérialisation ne pose jamais en
   arête ; un `*` vers un symbole réservé du module.
3. **choose_target** : après l'import, si le module désigne un fichier sans
   définisseur, suivre ses `REEXPORTS` vers le module suivant (chemins
   relatifs résolus depuis le fichier qui réexporte), sauts bornés.
4. **Mesure** : sonde (classe « nom par chemin de module »), banc des
   relations, filtre dans Liens et impact.

## Témoin

Rejoué le 11 octobre (session codeparsers), sur master `48ae8e4b5` : avec
un homonyme dans le projet (`src/autre.rs` définit aussi `DataflowEvent`),
`use crate::dataflow::DataflowEvent; … DataflowEvent::neuf()` ne laisse
**aucune** arête vers `dataflow/runtime.rs` — pire que « nom » : le
rendez-vous s'abstient. Sans homonyme (le cas de la sonde B9 de la session
recherche), le seul définisseur suffit et l'appel paraît attribué : c'est le
repli « un seul définisseur », marqué « nom ». Le témoin est écrit (recette
en quatre fichiers, deux assertions : un lien, vers runtime.rs, pas « nom »),
gardé hors de la batterie tant qu'il est rouge ; il entre avec le correctif.

## Correction (11 octobre 2026)

La décision du 4 octobre, avec une forme plus légère que le rendez-vous
`REEXPORTS` proposé :
- **codeparsers `77c8ea6`** : `ImportReference.reexport` (un `use` public,
  `pub` ou `pub(crate)`), avec son module source.
- **code.rs** : le premier scope de fichier porte un rendez-vous vers chaque
  nom réexporté (`pub use runtime::X`, sans renommage), marqué
  `MENTIONS.reexport_from = "runtime"` — la mention d'import elle-même, pas
  un second rendez-vous entre les mêmes nœuds. À la matérialisation,
  `choose_target` reçoit les fichiers qui réexportent le nom ; quand l'import
  désigne l'un d'eux, le module source, résolu depuis le module qui
  réexporte (`runtime`, `self::`, `super::`, `crate::`), désigne le fichier
  suivant, sur trois sauts au plus. Un seul définisseur atteint : marque
  « import ».
Témoin (la recette en quatre fichiers, avec un homonyme) : rouge avant
(aucune arête), vert après (`dataflow/runtime.rs`, « import ») ; et le test
unitaire `un_import_suit_les_reexportations`.
**Reste** : `pub use x::*` (une réexportation de tout un module), le
renommage (`pub use a::X as Y`), `export … from` (TypeScript) et
`__init__.py` (Python).
