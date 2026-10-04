# Un chemin vers une réexportation reste « par le nom »

- **État** : ouvert — décidé, en attente d'un créneau de l'arbre principal (code.rs)
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

Aucun test encore ; la sonde en donne le volume.
