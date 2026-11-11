# Un membre externe platform se lie à un homonyme du projet

- **État** : ouvert — observé dans l'A/B require du 9 novembre, relecture du 11 ; mesure du correctif des exports en cours
- **Gravité** : réponse fausse (usage relié au mauvais fichier)
- **Atteignable en service** : oui, résolution JS par le seul nom
- **Touche rag3weaver** : oui (rendez-vous de `src/code.rs`) et codeparsers (origine des références)
- **Famille** : produit

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
