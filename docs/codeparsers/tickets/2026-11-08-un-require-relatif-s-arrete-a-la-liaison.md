# Un `require` relatif s'arrête à la liaison au lieu de suivre le module

- **État** : en cours — correctif d96ed3264/a730a9b sur branche cx-codeparsers-require-2 ; décision de navigation et passe finale encore attendues
- **Gravité** : réponse fausse (un usage relié à la ligne du `require`, pas à la définition).
- **Atteignable en service** : oui, tout dépôt CommonJS dont un fichier lie une classe d'un autre fichier
  de l'arbre par `const X = require("./x.js")`.
- **Touche rag3weaver** : oui (résolution entre lots dans src/code.rs et pointeur codeparsers)
- **Famille** : produit

- **Pour** : session codeparsers
- **Signalé par** : session codeparsers ; reprise et preuves dans [le bilan du 11 novembre](../11-11-2026/04-bilan-navigation-et-provenance.md)

## Ce que c'est

`tools/wasm/src_js/connection.js:9` lie `const PreparedStatement = require("./prepared_statement.js")`
et l'emploie ligne 139. Notre arête CONSUMES (marque « fichier ») va à la ligne 9 du même fichier ;
typescript-language-server rend la classe, `tools/wasm/src_js/prepared_statement.js:10`. Un `import`
ES se suit jusqu'à son module ; un `require` relatif reste une variable du fichier.

Les 4 mentions (juge strict, arbre a72acfeef, `locales-mesure/verdicts-B.tsv` à luciepc) :

- `PreparedStatement`, `tools/wasm/src_js/connection.js:139` → `prepared_statement.js:10` ;
- `PreparedStatement`, `tools/wasm/src_js/sync/connection.js:110` → `sync/prepared_statement.js:10` ;
- `PreparedStatement`, `tools/nodejs_api/src_js/connection.js:236` → `prepared_statement.js:5` ;
- `QueryResult`, `tools/wasm/src_js/connection.js:158` → `query_result.js:8`.

Une cinquième de la même forme, `dispatcher` (`tools/wasm/src_js/index.js:25` → `dispatcher.js:64`).

## Deux faux voisins, notés ici

- `var entries = []` (`opfs-bridge.js:90`) : le serveur rend `lib.dom.asynciterable.d.ts` pour la
  mention de la ligne 91, sans doute un `.entries()` d'itérateur lu comme la variable.
- `m.charIdx` (`playground/test_multibyte.mjs:41`, un accès de champ dans un gabarit de chaîne) relié à
  la globale `let charIdx` de la ligne 13 : un champ ne devrait pas se relier à une variable.

## Ce qu'il faut pour le fermer

1. Un `const X = require("<chemin relatif>")` se traite comme `import X from "<chemin>"` : la mention de
   `X` va à l'export du module quand il est dans l'arbre ; sinon, la liaison reste la cible.
2. Un témoin à deux fichiers (`connection.js`, `prepared_statement.js`).
3. L'oracle sur les 4 mentions.

## Suite du lot et intégration — 11 novembre

Porteur : codeparsers. Le lot d96ed3264/a730a9b est conservé sur
cx-codeparsers-require-2, worktree /home/lucied/git_workspaces/cx-wt-codeparsers-require.
Preuves : 8 e2e et 286 tests producteur passés ; deux index A/B et complément
23 questions terminés. Détails et preuves brutes :
[04-bilan-navigation-et-provenance.md](../11-11-2026/04-bilan-navigation-et-provenance.md).

Reste à faire : décider avec l'orchestration le contrat de navigation
alias local/export, attribuer les cas encore incertains via le ticket de
l'oracle, puis reprendre sur la base finale convenue après R5, sous une
branche neuve. Jouer la passe complète réglementaire, dont PostgreSQL sur
la base privée cx_codeparsers_20261111_1698_test. Relire les quatre mentions
originelles dans les preuves avant toute clôture. Publier seulement après
verdict vert et tour de push explicite ; documenter le résultat utilisateur.
L'attente de base et la transmission du bilan ne ferment pas ce ticket.

### Préparation de la reprise sur master

Lecture du master distant 1b5cfd941 après fetch, sans rebase ni compilation :
80 commits sont arrivés depuis la branche courante. Depuis la base aece4b9f1,
aucun changement de code.rs ni records.rs ; la reprise n'annonce donc pas de
conflit textuel dans ces deux fichiers. Cela ne prouve pas la compatibilité :
les changements du catalogue, backend, indexation, recherche et dépendances
Lucivy imposent la passe sur l'ensemble intégré. run_e2e.sh a aussi changé.
La base finale reste celle à convenir après le relais/UI et ED, pas ce relevé.

Le diff global du lot comprend déjà une évolution historique de
scripts/oracle_lsp.py antérieure au complément : catégories liaison/réexportation.
L'affirmation « oracle inchangé » du bilan porte sur les campagnes figées et
le complément, tous épinglés au même hash, pas sur l'absence de diff de cet
outil dans tout le lot depuis aece4b9f1. Garder cette distinction en revue.

Aucun nouveau target créé : réutiliser target-codeparsers-clic avec le garde
RAG3WEAVER_TARGET_REPRENDRE=1 au point de reprise ; épingler la lib convenue,
pas courant. Demander le créneau après atterrissage de la base ED commune.
