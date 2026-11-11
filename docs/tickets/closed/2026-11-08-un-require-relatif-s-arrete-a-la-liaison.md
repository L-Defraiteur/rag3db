# Un `require` relatif s'arrête à la liaison au lieu de suivre le module

- **État** : corrigé — code d92c0f4b7/a730a9b ; passe intégrée 50ebbb760 relue ; contrat accepté et tour de push accordé par l’orchestration le 11 novembre 2026
- **Gravité** : réponse fausse (un usage relié à la ligne du `require`, pas à la définition).
- **Atteignable en service** : oui, tout dépôt CommonJS dont un fichier lie une classe d'un autre fichier
  de l'arbre par `const X = require("./x.js")`.
- **Touche rag3weaver** : oui (résolution entre lots dans src/code.rs et pointeur codeparsers)
- **Famille** : produit

- **Pour** : session codeparsers
- **Signalé par** : session codeparsers ; reprise et preuves dans [le bilan du 11 novembre](../../codeparsers/11-11-2026/04-bilan-navigation-et-provenance.md)

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
[04-bilan-navigation-et-provenance.md](../../codeparsers/11-11-2026/04-bilan-navigation-et-provenance.md).

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

### Passe intégrée autorisée après la base ED/archi

Base commune 68b9ede17c182487d3f823c2e5cc5906b62a4a46 ; rebase sans conflit
sur branche neuve cx-codeparsers-require-3, code relu à d92c0f4b7.
Diff produit : code.rs (résolution/export et liaison externe), tests e2e_code,
pointeur a730a9b ; aucun changement de records.rs ni du contrat IR des scopes.
Le diff oracle est historique (catégories liaison/réexportation), distinct de
la provenance de mention et du prédicat strict conservé dans le complément.

Lib convenue confirmée par l'optimiseur :
rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF.
Passe prévue : cinq jeux check_features ; lib entière large ; navigateurs ;
e2e_code, e2e_usages et e2e_postgres séparés, jeu large. Poste lourd, -j8,
CARGO_INCREMENTAL=0, target unique repris par garde, PostgreSQL privé ; aucun
nouveau GPU/service, LSP ou A/B. Sources figées pendant ces vérifications.
Les journaux seront sous ~/.cache/codeparsers-notes/cx-passe-require-3/ sur
luciepc ; seul leur verdict relu permet de demander le tour de push.

### Résultats de la passe intégrée

SHA joué 50ebbb7606c9dd7099c1441b782c29850e3c3c35 (code d92c0f4b7,
base 68b9ede17), lib 29a7 complète ci-dessus. Du 11 novembre 21:56:06 à
22:05:25 Paris, sous poste lourd : check_features VERT 5/5 ; lib large
2114 réussis, 0 échec, 64 ignorés ; navigateur 39/39 sans ignoré ; e2e_code
36/36, e2e_usages 19/19, e2e_postgres 34/34, tous sans ignoré. Chaque appel
e2e a aussi joué ses deux témoins préalables. Six étapes finies code=0.
Le target a culminé vers 26 Go puis est revenu à 25 Go ; sources propres
et HEAD exact relus après la passe.

Les 64 ignorés de la lib ne sont pas revendiqués comme validations : 39 sont
les navigateurs repris ensuite ; les 25 autres sont 16 sans motif imprimé,
un témoin hôte réel, trois llama-server, quatre mesures et un rouge connu
de checkpoint. Leurs noms et raisons sont conservés dans lib.log. Aucun
service supplémentaire ou banc à modèle n'a été lancé pour les masquer.
e2e_code laisse les résidus déjà déclarés pour cette suite dans
suites_qui_laissent_des_residus.txt ; usages et postgres n'en signalent pas.
Journaux complets et inventaire des étapes : cx-passe-require-3/{features.log,
lib.log,navigateur.log,code.log,usages.log,postgres.log,etat.log,provenance.txt}.

Relecture sans nouvelle question LSP de verdicts-B.tsv dans
require-default-v2-d96ed3264 : les quatre mentions originelles 139/110/236/158
sont strictement justes, marque import, vers leurs classes PreparedStatement
et QueryResult attendues. Cette preuve complète les critères de correction,
mais la décision de navigation et l'intégration restent ouvertes. Aucun push
master fait ni autorisé par ce constat ; bilan envoyé pour revue.

### Décision de navigation après revue

Contrat retenu par l'orchestration pour ce lot : CONSUMES exprime une dépendance
et navigation vers le symbole ou la valeur exportée d'un import relatif résolu,
même si le nom local diffère. Une instance vise sa zone d'export, jamais une
classe inventée ; absent/externe conserve la liaison marquée ; ambigu ne se
devine pas. Ce contrat n'est pas une équivalence avec textDocument/definition.
Les cinq usages alias restent strictement faux dans la comparaison LSP ;
aucun score global mélangé, aucun changement de prédicat strict. La mention
source reste distincte de sa cible ; ses pertes de provenance et le faux
platform demeurent dans leurs tickets ouverts, capture 23 séparée.

README et nouveautés explicitent ce contrat, sans nouveau code. La passe
sur 50ebbb760 est inchangée ; seule la documentation est complétée depuis.
Reste l'intégration sous tour explicite et la clôture avec ses preuves : le
présent ticket n'est pas fermé par l'accord de conception.

## Clôture

Les quatre mentions originelles sont strictement justes dans la seconde A/B ;
les huit témoins et les 286 tests producteur sont passés, ainsi que la passe
intégrée détaillée ci-dessus. Contrat de navigation accepté, README et nouveautés
alignés (1d70a8141), cinq strict-faux conservés. L'orchestration a relu les
journaux et donné le tour de push. Le défaut require est corrigé ; les suites
provenance et homonyme restent dans leurs deux tickets ouverts, sans transfert
implicite de leur réalisation dans cette clôture. Aucun filet à retirer.
