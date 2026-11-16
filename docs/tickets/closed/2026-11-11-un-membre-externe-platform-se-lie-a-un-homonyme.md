# Un membre externe platform se lie à un homonyme du projet

- **État** : fermé (16 novembre 2026) — deux causes, rag3weaver et codeparsers `edfc72d` ; voir « La correction »
- **Gravité** : réponse fausse (usage relié au mauvais fichier)
- **Atteignable en service** : oui, résolution JS par le seul nom
- **Touche rag3weaver** : oui (rendez-vous de `src/code.rs`) et codeparsers (origine des références)
- **Famille** : produit

- **Pour** : session codeparsers
- **Signalé par** : session codeparsers ; reprise et preuves dans [le bilan du 11 novembre](../11-11-2026/04-bilan-navigation-et-provenance.md)

## Preuve

À luciepc, `~/.cache/codeparsers-notes/require-mesure-2/verdicts-ajoutees.tsv` :
le scope « Database close », mention `platform` dans
`tools/nodejs_api/test/test_database.js:377`, vise
`tools/nodejs_api/install.js:9`, avec la marque `nom`.
L'oracle TypeScript strict désigne le membre du type Node process dans
`@types/node/process.d.ts:1765` : verdict faux.
Arbre indexé a72acfeef, lot 0a20764f5/codeparsers78888f5,
lib épinglée 9bae60786 (identifiant complet dans le rapport de reprise).

Ce défaut est distinct de l'instance FS exportée : la propagation de l'export
par défaut ne justifie aucun changement de politique des modules externes.
La nouvelle A/B doit dire si cette arête reste présente ; aucun verdict nouveau
n'est encore revendiqué.

## Pour le fermer

Isoler une fixture de membre d'un module externe avec un homonyme local ;
poser le rouge et déterminer si l'origine d'import ou le receveur se perd.
Mesurer une règle générique qui conserve cette provenance jusqu'au rendez-vous,
avec l'oracle strict, sans promouvoir une absence de réponse en gain.

## La correction (16 novembre 2026)

Le rouge est vu sur les vrais fichiers du dépôt (`tests/e2e_usages.rs`,
`an_external_member_is_not_a_project_homonym_in_the_real_files` : `install.js`, `test/test_database.js`,
`src_js/rag3db_native.js`). La forme réduite (`an_external_member_is_not_a_project_homonym`) passait déjà :
deux causes, qu'elle n'avait pas.

1. **Le qualificatif vient d'un import d'une bibliothèque extérieure** : `const process = require("process")`,
   puis `process.platform`. L'accès de champ par un qualificatif importé garde son rendez-vous, et le seul
   définisseur concluait par le nom. Désormais (rag3weaver, la collecte du rendez-vous), une référence dont le
   qualificatif vient d'un import (`via_qualifier`) d'une bibliothèque du fichier **de provenance sûre**
   (`builtin`, `dependency`) ne prend pas de rendez-vous : le membre est celui de la bibliothèque. La première
   forme, sans la provenance, ôtait 65 liens Python justes (`rag3db.Connection(db)` : le paquet du projet, que
   `pyproject.toml` déclare ; provenance `project`).
2. **Au niveau du fichier, la référence lue sur le texte perdait son qualificatif** : `process.platform` en tête
   de `rag3db_native.js` sortait comme le nom nu `platform`. Codeparsers (`edfc72d`) donne le nom qui précède
   `.`, `?.` ou `->` pour qualificatif ; la règle des accès de champ s'applique alors comme dans une fonction.
   Témoin : `codeparsers/tests/file_scope_members.rs`, vu rouge puis vert.

La sonde (`tests/sonde_marque_nom.rs`, corpus figé de rag3db-garde ; avant = master, codeparsers `7702f03`) :

| corpus | « nom » avant | après | couples partis |
|---|---|---|---|
| rag3weaver/src | 9 064 | 9 059 | 8 : `.lines()`, `OpenOptions::append` (std), des champs de `page.js` lus au niveau du fichier, reliés à des fonctions homonymes |
| src/ du moteur | 2 654 | 2 653 | 1 (`count`) |
| tools/nodejs_api | 348 | 324 | 26 : `path.join` (vers un `join` du projet), `process.env`, `process.platform` |
| tools/python_api | 256 | 256 | 0 |

Aucun couple ne naît ; ceux qui partent sont tous faux à la lecture.
