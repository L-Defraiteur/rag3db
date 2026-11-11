# Rapport de session — codeparsers (11 novembre 2026)

*Écrit par la session « codeparsers » de Claude Code, avant le passage des sessions de Lucie sur Codex.
Il s'adresse à un agent qui n'a aucune mémoire de nos échanges. Le savoir durable (comment le dépôt
marche, comment on mesure, les pièges) est dans [02-knowledge-dump.md](02-knowledge-dump.md).*

## Vocabulaire minimal

- **codeparsers** : la bibliothèque Rust qui lit le code source avec les grammaires tree-sitter et rend
  des *scopes* (fonctions, classes, variables, zones de fichier) et des *relations* entre eux. Elle vit
  dans son propre dépôt (`github.com/L-Defraiteur/codeparsers`) et entre dans rag3db comme **sous-module
  git** à `extension/rag3weaver/codeparsers`.
- **rag3weaver** : l'extension Rust de rag3db (`extension/rag3weaver/`) qui indexe un dépôt de code dans
  une base rag3db. `src/code.rs` y transforme la sortie de codeparsers en nœuds et en arêtes.
- **Le pointeur** : le commit de codeparsers que le dépôt rag3db enregistre pour son sous-module. Le
  faire avancer, c'est un « lot de pointeur ».
- **CONSUMES** : l'arête « ce scope utilise ce scope » (appel, lecture d'un nom). Chaque arête porte une
  **marque** `resolution` qui dit comment sa cible a été choisie : `fichier` (dans le même fichier),
  `import` (un import désigne le fichier de la cible), `nom` (le seul scope de ce nom, sans autre
  preuve), `type` (le type lu), `ambigu` (plusieurs cibles), et depuis ce lot `liaison` (voir plus bas).
- **c7** : la session d'orchestration, qui ordonne la file des pushes et tranche les questions de
  conception courantes. **Lucie** tranche les autres.
- **luciepc** : le second poste (accès `ssh lucied@luciepc`), où tournent les passes lourdes et les
  mesures. **Le portable** : le poste de Lucie, où est ce dépôt.
- **Une passe** : la suite de tests exigée avant un push (procédure `docs/procedures/push-d-un-lot.md`).
- **L'oracle** : `extension/rag3weaver/scripts/oracle_lsp.py`, qui juge nos arêtes en demandant à un
  vrai serveur de langage (clangd, rust-analyzer, pyright, typescript-language-server) où va chaque
  nom.

## Ce qui est sur master (rag3db et codeparsers)

| date | quoi | rag3db | codeparsers |
|---|---|---|---|
| 7 nov. | la visibilité C/C++ par `#include` (INCLUDES) | 9fd37ab1c | — |
| 7 nov. | un appel dans la garde d'un bras `match` Rust compte comme usage | f3a115887 | 672a0d5 |
| 7 nov. | ticket « la lib entière se fige ou rougit par intermittence » (fermé depuis par une autre session) | a6beebc60 | — |
| 7-8 nov. | la coloration du code par les grammaires (`syntax_highlighting::highlight`) ; le côté rag3weaver (routes `/highlight`) est le lot de la session hooks-essai | 667d5e35a | 2bfa28b |
| 8 nov. | oracle corrigé (indexation annoncée exigée), rust/python/ts rejugés | 904c34fff | — |
| 8 nov. | une locale JavaScript ne se résout que depuis sa portée (`visible_until_line`) | 43dd520b0 | c660893 |

**Le pointeur de master est c660893** (codeparsers master aussi).

## Le lot en cours : `require` (non poussé)

**Le défaut** (ticket ouvert `docs/codeparsers/tickets/2026-11-08-un-require-relatif-s-arrete-a-la-liaison.md`) :
en JavaScript CommonJS, `const PreparedStatement = require("./prepared_statement.js")` faisait de la
ligne de liaison une variable du fichier, et les mentions de `PreparedStatement` y allaient au lieu de
la classe définie dans `prepared_statement.js`.

**Les branches** :

- rag3db : `codeparsers/require-js`, tête **280b5a3f1** (sur master 4f54e473a). Elle porte le pointeur
  vers codeparsers **78888f5**, `src/code.rs` (la marque `liaison`, et `module_designe_fichier` qui ôte
  l'extension `.js`/`.ts` d'un import), le témoin e2e `an_external_module_binding_is_marked_liaison`
  dans `tests/e2e_code.rs`, et l'oracle (catégories « réexportation » et « liaison d'import » sur une
  zone).
- codeparsers : `require-js`, tête **78888f5** (sur c660893).
- **Cargo.lock** : rien à changer, codeparsers 78888f5 n'ajoute aucune dépendance (vérifié par
  `git diff c660893 78888f5 -- Cargo.toml Cargo.lock`, vide).

**Ce que fait le lot** :

1. codeparsers lit `const X = require("m")` comme `import X from "m"`, et `const { a, b: c } =
   require("m")` comme des imports nommés ; la ligne de liaison ne fait plus de scope variable.
2. Une ligne d'import (`import … from`, `const … = require(…)`) ne compte plus comme une mention des
   noms qu'elle lie.
3. Un import qui ne se résout pas dans le projet (module externe comme `require("path")`, ou fichier
   absent) garde une cible : la **zone de fichier** (`file_scope_NN`, le texte entre deux scopes) qui
   contient sa ligne d'import, avec la marque **`liaison`** (« liaison de module externe »). Décision de
   c7 du 8 novembre : pas de trou de navigation, mais l'arête dit qu'elle s'arrête à la liaison.
4. En mode « entre fichiers » de codeparsers seulement (pas celui de rag3weaver, voir plus bas) : un
   import par défaut va à l'export par défaut du module (`module.exports = X`, `export default X`), et
   une instance exportée (`module.exports = new FS()`) va à la zone qui l'écrit, pas à la classe.

**Les tests** : codeparsers, 46 binaires, 283 tests verts, dont `tests/require_js.rs` (10 témoins) ;
rag3weaver, les 33 tests unitaires `code::` et le témoin e2e de la marque `liaison` verts (luciepc,
8 nov.). **La passe complète de push n'a pas été jouée** sur cette branche.

**La mesure A/B** (finie le 9 novembre à 02 h 31, luciepc, lib
`rag3db-build-9bae60786b05ec1463c60d2630a4de4444bc2cfd-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`,
sans `RAG3WEAVER_MOTEUR_ANCIEN`) : la base de code « code-lourd » (l'arbre rag3db a72acfeef) indexée deux
fois, A = master 4f54e473a (codeparsers c660893), B = la branche (0a20764f5, codeparsers 78888f5). Résultats
dans `~/.cache/codeparsers-notes/require-mesure-2/resume.txt` à luciepc.

- Sur les 298 mentions TypeScript/JavaScript de l'échantillon de référence : A 324 arêtes, **279 justes**,
  10 fausses, 12 liaisons d'import ; B 300 arêtes, **275 justes**, **2 fausses**, 0 liaison d'import.
- Arêtes comparées par (mention, cible) : **82 ôtées** (51 liaisons d'import, 14 fausses, **17 justes**) ;
  **426 ajoutées** (**415 marquées `liaison`**, que l'oracle rend « muettes » parce que le serveur ne
  désigne rien pour ces noms de modules externes ; 7 justes ; 2 fausses ; 1 réexportation ; 1 muette).
- Les CONSUMES d'un mentionneur TS/JS passent de 1 381 à 1 878 (+36 %), presque tout par les liaisons.
- La durée de l'index B (3 594 s contre 1 172 pour A) **ne vaut rien** : le fstrim hebdomadaire de
  luciepc prenait le disque pendant B.

**Ma lecture** : le lot ôte les faux et les liaisons d'import qu'il visait, mais il perd 17 justes
(des réexportations `module.exports = { Connection, … }` dans des `index.js`, et des liaisons de
modules externes ou générés qui étaient la seule définition dans l'arbre) et ajoute 415 arêtes
`liaison` qu'aucun oracle ne juge. **La découverte qui explique les justes perdus** : rag3weaver
appelle codeparsers en mode « fichier seul » (`resolve_cross_file: Some(false)`, `src/code.rs` vers la
ligne 1041) et fait les liens entre fichiers dans son propre « rendez-vous » de `code.rs`, qui ne relie
une mention qu'aux scopes **du même nom**. L'export par défaut d'un autre nom que l'alias, et l'instance
exportée, ne lui sont pas visibles ; le point 4 ci-dessus ne sert donc pas l'index.

**Pourquoi rien n'est poussé** : la mesure s'est finie la nuit ; c7 devait donner son « à toi » au
matin, et le bilan (17 justes perdus, +36 % d'arêtes TS/JS par les liaisons) demande une décision
avant le push : garder les liaisons telles quelles, les restreindre, ou d'abord faire le second lot
(ci-dessous).

## Le ticket `import_binding`

c7 a demandé (8 nov.) de **compter avant de décider** : une sorte de scope nouvelle, `import_binding`,
donnerait à chaque nom importé (y compris `import` ES externe) sa propre cible. Il faut d'abord compter
sur code-lourd combien de noms importés cela ferait (TS/JS, et Python, Rust, Go si la règle vaut pour
tous) et ce que cela pèse en scopes et en vecteurs, puis écrire le ticket avec le chiffre ; on décide
ensuite, avec Lucie si l'index grossit de plus de quelques pour cent. **Le compte n'est pas fait** ; le ticket
est écrit (11 nov.) : `docs/codeparsers/tickets/2026-11-11-une-mention-d-un-import-es-externe-n-a-pas-de-cible-a-elle.md`.

## L'arbre de Lucie : le sous-module apparaît modifié

Dans `~/git_workspaces/rag3db` (l'arbre de Lucie, le portable), `git status` montre
` M extension/rag3weaver/codeparsers`. L'arbre (HEAD 3c0ec28b5) enregistre le pointeur **c660893**, mais
le sous-module est resté extrait à **672a0d5** (le pointeur du 7 novembre). L'arbre a avancé sans
`git submodule update`. Rien n'est perdu (672a0d5 est un ancêtre de c660893). Pour le remettre d'accord :
`git -C ~/git_workspaces/rag3db submodule update extension/rag3weaver/codeparsers`. **Les sessions ne
touchent pas cet arbre** ; c'est à Lucie de le faire, ou à une session sur son mot.

## Mes tickets ouverts (`docs/codeparsers/tickets/`)

- `docs/codeparsers/tickets/2026-11-08-un-require-relatif-s-arrete-a-la-liaison.md` — le lot en cours ci-dessus.
- `docs/codeparsers/tickets/2026-11-07-la-marque-nom-relie-un-appel-de-methode-standard-a-un-homonyme.md` — `split_off`,
  `is_zero` reliés par le nom à un homonyme du dépôt quand le receveur est un type de la bibliothèque
  standard.
- `docs/codeparsers/tickets/2026-10-25-une-definition-cpp-se-lie-a-sa-declaration-d-en-tete.md` — C++ : une définition reliée à sa
  propre déclaration d'en-tête.
- `docs/codeparsers/tickets/2026-10-25-fmt-format-h-une-macro-lue-comme-un-corps-de-fonction-depuis-tree-sitter-0-27.md` — recul
  accepté par Lucie avec tree-sitter 0.27.
- `docs/codeparsers/tickets/2026-10-18-des-scopes-sans-defined-in-dans-une-reindexation-a-neuf.md` — des scopes sans arête vers
  leur fichier après une réindexation à neuf.
- `docs/codeparsers/tickets/2026-10-11-un-fichier-rust-s-analyse-plus-lentement-a-la-ligne.md` — coût d'analyse d'un fichier Rust.
- `docs/codeparsers/tickets/2026-10-11-new-est-un-mot-vide-en-rust.md` — `new` traité comme un mot vide.
- `docs/codeparsers/tickets/2026-10-04-des-receveurs-restent-sans-type.md` (en cours) et
  `docs/codeparsers/tickets/2026-10-04-une-arete-ne-dit-pas-comment-elle-a-ete-resolue.md` (en cours) — la lecture des types de
  receveurs et les marques ; partagés avec d'autres sessions.

Ils sont dans `docs/codeparsers/tickets/` depuis le 11 novembre, avec un dixième ouvert le même jour :
`docs/codeparsers/tickets/2026-11-11-une-mention-d-un-import-es-externe-n-a-pas-de-cible-a-elle.md` (la sorte
`import_binding`, le compte à faire).

## Ce qui reste sur disque (rien n'a été supprimé)

**luciepc** :

- `~/.cache/codeparsers-notes/` (66 Gio, 464 entrées) : scripts de passe et de mesure
  (`branche-push-sm.sh`, `push-final.sh`, `rejeu-court.sh`, `suite-de-passe.sh`, `mesure-require-2.sh`,
  `mesure-locales.sh`, `lib-figement.sh`), résultats (`require-mesure-2/`, `locales-mesure/`,
  `rejuge-lsp/`, `figement/`, `garde-mesure/`), l'oracle et son arbre (`oracle/`, dont `oracle/arbre`, une
  copie de rag3db à a72acfeef, et `oracle/ts5`, un TypeScript 5). **Attention** :
  `hors-arbre-18-oct/` et `archive-mtga-18-oct/` contiennent les données du projet Magic de Lucie
  (`experiments/mtga`, environ 24 Gio, sans doute la seule copie) : **ne jamais les supprimer ni les
  déplacer**.
- Worktrees : `~/git_workspaces/rag3db-brancher` (branche locale `local-require-js`, 0a20764f5),
  `rag3db-mesure-garde` (détaché, 0a20764f5), `rag3db-figement` (détaché, d845744a2),
  `codeparsers-garde` (672a0d5), `rag3db-temoin-inc` (c2ef8a941, un fichier modifié, ancien).
- Targets cargo : `~/.cache/rag3weaver-build/target-codeparsers-clic` (15 Gio),
  `target-figement` (3,5 Gio).

**Le portable** :

- Worktrees : `~/git_workspaces/rag3db-garde` (mes branches rag3db), `~/git_workspaces/codeparsers-garde`
  (branche `require-js`, 78888f5).
- Target : `~/.cache/rag3weaver-build/target-codeparsers-local` (5,9 Gio).

**Branches distantes anciennes** (rag3db `codeparsers/*` et codeparsers), toutes atterries ou
remplacées : `includes`…`includes-4`, `tree-sitter-027`…`-4`, `garde-pointeur`…`-3`, `locales-js`…`-4`,
`oracle-clangd`, `oracle-lsp`, `outline`, `docs-graine-lsp`, `allegement-boucle-langages`, `rejuge-lsp`.
Leur suppression est une décision de Lucie (pas de `git push --delete` sans elle).

## Reprendre

Un seul travail n'est pas fini : **le lot `require`**. Reprendre de ses branches, pas de master.

### Le lot `require`, sur le portable

- **rag3db** : `~/git_workspaces/rag3db-garde` (un worktree de ce dépôt). La branche à reprendre est
  `codeparsers/require-js`, dernier commit **280b5a3f1** (« outils(oracle_lsp): les catégories
  « réexportation » … »), poussée sur `origin`. *Le worktree peut être extrait sur une autre branche
  (celle de ce rapport) : faire d'abord le `checkout` ci-dessous.*
- **codeparsers** : `~/git_workspaces/codeparsers-garde` (un clone de codeparsers), branche `require-js`,
  dernier commit **78888f5**, propre, poussée.
- **Target** : `~/.cache/rag3weaver-build/target-codeparsers-local` (5,9 Gio, celui des tests de
  codeparsers seul).
- **Fait** : le code (codeparsers et `code.rs`), les témoins verts, la mesure A/B.
- **Reste** : la décision (voir « Les trois prochains pas »), la passe complète, le push.

Commandes (shell **zsh** ; `poste lourd` est le verrou des travaux lourds du poste, voir le dump) :

```
cd ~/git_workspaces/rag3db-garde
git fetch origin && git checkout codeparsers/require-js && git reset --hard origin/codeparsers/require-js
git submodule update extension/rag3weaver/codeparsers          # le sous-module à 78888f5
git -C extension/rag3weaver/codeparsers log -1 --format=%h      # doit dire 78888f5
# Les tests de codeparsers seuls :
cd ~/git_workspaces/codeparsers-garde && git checkout require-js
CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-codeparsers-local ~/.cache/rag3weaver-build/poste lourd timeout 3000 nice cargo test -j 8
```

### Le même lot, à luciepc (où se jouent la passe et le push)

- **Worktree** : `~/git_workspaces/rag3db-brancher`, branche locale `local-require-js`, dernier commit
  **0a20764f5** ; il lui manque le commit de l'oracle 280b5a3f1 (le `reset` ci-dessous l'apporte).
  Sous-module extrait à 78888f5, égal au pointeur. Propre.
- **Target** : `~/.cache/rag3weaver-build/target-codeparsers-clic` (15 Gio), déjà bâti pour cette
  branche (binaires de test sous `debug/deps/rag3weaver-*`).
- **La lib du moteur** à épingler (jamais `courant`) : la dernière ordinaire annoncée par
  l'orchestration ; le 9 novembre c'était
  `rag3db-build-9bae60786b05ec1463c60d2630a4de4444bc2cfd-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`,
  sans `RAG3WEAVER_MOTEUR_ANCIEN`. Vérifier avec l'orchestration avant de lancer.

La passe puis le push, à la main (la procédure complète est `docs/procedures/push-d-un-lot.md`) :

```
ssh lucied@luciepc
cd ~/git_workspaces/rag3db-brancher
git fetch origin && git reset --hard origin/codeparsers/require-js && git rebase origin/master
git submodule update extension/rag3weaver/codeparsers
cd extension/rag3weaver
export RAG3DB_LIB=<l'identifiant complet de la lib> RAG3DB_ROOT=$HOME/git_workspaces/rag3db-brancher
export CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/target-codeparsers-clic CARGO_INCREMENTAL=0
export RAG3WEAVER_PG="postgres://rag3weaver:rag3weaver@127.0.0.1:5433/rag3weaver_test"
export RAG3WEAVER_PAQUETS_REACT=$HOME/.cache/codeparsers-notes/paquets-react
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880 RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70
F=rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm
P=~/.cache/rag3weaver-build/poste
$P lourd timeout 3600 bash scripts/check_features.sh
$P lourd timeout 3600 bash scripts/cargo_magasin.sh test -j8 --features $F --lib
FEATURES=$F $P lourd timeout 3600 bash scripts/temoins_navigateur.sh
for s in e2e_code e2e_usages e2e_postgres; do $P lourd timeout 3600 ./run_e2e.sh --test e2e $s; done
```

Tout vert (aucun `FAILED`, aucun « running 0 tests ») et le « à toi » de l'orchestration reçu : le
dernier pas sous `poste push` (fetch, rebase, et s'il apporte du code, rejouer avant ; puis le
pointeur de codeparsers sur son master, puis le push) :

```
git -C extension/rag3weaver/codeparsers push origin 78888f53b2415ee615a3b4e8f299a35e66dff33c:refs/heads/master
$P push codeparsers -- bash -c 'git fetch -q origin && git rebase -q origin/master && git push -q origin HEAD:master && echo POUSSÉ $(git rev-parse --short HEAD)'
git fetch origin && git status --short && git log -1 --format=%h origin/master   # les trois contrôles
```

Le script `~/.cache/codeparsers-notes/push-final.sh` (luciepc) fait ce dernier pas pour le lot des
locales ; il est à adapter (branche, sous-module attendu) s'il sert de modèle.

### Les autres worktrees de luciepc : finis, rien à reprendre

- `~/git_workspaces/rag3db-mesure-garde` (détaché, 0a20764f5, sous-module 78888f5) : l'arbre de la mesure
  A/B ; il a servi, rien n'y attend.
- `~/git_workspaces/rag3db-figement` (détaché, d845744a2) : l'arbre du guetteur de figement du
  7 novembre ; fini.
- `~/git_workspaces/codeparsers-garde` (672a0d5) : un ancien clone de codeparsers ; rien n'y attend.
- `~/git_workspaces/rag3db-temoin-inc` (c2ef8a941, 19 oct., sous-module à 7c5ff80 alors que le pointeur
  dit 5949214, d'où ` M extension/rag3weaver/codeparsers`) : un vieux témoin d'INCLUDES, atterri depuis.

## Ce qui est bloqué, et sur quoi

- Le lot `require` : sur la décision de c7 (ou de Lucie) au vu de la mesure, puis sur son « à toi ».
- Le ticket `import_binding` : sur le compte, à faire (un lourd à luciepc, une indexation).

## Les trois prochains pas

1. **Trancher le lot `require`** avec c7 : pousser tel quel (liaisons comprises), ou restreindre la
   marque `liaison` (par exemple aux seuls noms dont aucun autre scope ne porte le nom), ou attendre le
   second lot. Puis la passe complète (`docs/procedures/push-d-un-lot.md`, `e2e_code` et `e2e_postgres`
   puisque `code.rs` change) et le push.
2. **Le second lot** : faire passer l'export par défaut de chaque fichier JS/TS (un nom, ou « instance
   de X ») de codeparsers jusqu'au rendez-vous de `code.rs`, qui le lit quand l'import est par défaut.
   Il rendrait les 17 justes perdus et corrigerait l'instance (`FS`).
3. **Le compte pour `import_binding`**, puis le ticket avec les chiffres.

## Reprise du 11 novembre — exports par défaut

Le lot require initial reste différé. État courant et preuves dans
[03-reprise-exports-par-defaut.md](03-reprise-exports-par-defaut.md) : branche
`cx-codeparsers-require-2`, code d96ed3264, sous-module a730a9b ; huit e2e ciblés
et 286 tests unitaires/intégration du parseur verts. La passe complète et une
nouvelle A/B sur ces hashes restent à faire. La première A/B e101/e48 est
conservée séparément et ne valide pas la nouvelle implémentation.

## Livraison du lot require et exports — état après revue finale

L'état de reprise ci-dessus est historique. Le lot initial et son complément
exports sont réunis sur cx-codeparsers-require-3 ; code d92c0f4b7, producteur
a730a9b publié sur origin/cx-codeparsers-default-metadata. Passe intégrée sur
50ebbb760/base68b9/lib29a7 relue : features5/5, lib2114 réussis/64 ignorés
non revendiqués (39 repris navigateur), navigateur39, code36, usages19,
PostgreSQL privé34, sans échec ni ignoré dans ces étapes séparées.
Le contrat navigation vers le symbole/valeur exportée est accepté ; les cinq
strict-faux restent dans le bilan, sans score global. Tour de push accordé.
Ticket corrigé déplacé vers
`docs/tickets/closed/2026-11-08-un-require-relatif-s-arrete-a-la-liaison.md`.
Les actions non livrées restent aux tickets codeparsers de provenance oracle,
homonyme platform, import_binding et receveurs/résolution ; aucune nouvelle
campagne LSP ni capture n'est lancée par cette livraison.

Les worktrees et branches locales du lot, le target-codeparsers-clic et la base
PG privée sont à supprimer immédiatement après les trois contrôles du push.
Les branches distantes et les preuves sous ~/.cache/codeparsers-notes/ restent
conservées ; ne pas effacer les données des deux A/B avec le target. Le résultat
effectif du nettoyage est transmis à l'orchestration après exécution.

## Livraison et nettoyage effectifs

Master reçoit 2e910930dbb9684fdfeb1f387fc2a430c9604880 sous poste push après
verdict relu. Trois contrôles lus : status propre, aucun rebase en cours,
origin/master égal à HEAD. Le sous-module a730a9ba89ef233423bb7063f85181de26d98ce0
est accessible sur origin/cx-codeparsers-default-metadata.

Sur luciepc : worktree de passe cx-wt-codeparsers-require, worktrees des deux
mesures (cx-wt-codeparsers-mesure-v2, rag3db-require-default-mesure) et ancien
rag3db-brancher supprimés ; branches locales -2/-3/-4 et require-temoins retirées.
Target unique target-codeparsers-clic et base PG privée
cx_codeparsers_20261111_1698_test supprimés. Sur le portable : worktrees require,
témoins et métadonnées, branches locales require et variantes, ainsi que les
branches locales du sous-module associées, supprimés. Les branches distantes
restent intactes. Mesures, bases A/B, binaires figés et journaux sont conservés
sous ~/.cache/codeparsers-notes/, hors du target supprimé ; derniers journaux
du wrapper copiés dans cx-passe-require-3/wrapper-final.

Incident de nettoyage : codeparsers-garde distant était un worktree rattaché
au sous-module de rag3db-brancher. Le retrait du parent invalidait son lien Git.
Il a été rétabli par init/fetch/reset --mixed vers le commit historique
672a0d5 ; status propre après rétablissement. Aucune comparaison initiale des
fichiers, références ou writers ne permet de conclure à leur identité avant/après. Ce clone ancien est conservé,
ainsi que la trace dans cx-passe-require-3/reparation-ancien-codeparsers-garde.log.
Rectification : cet incident reste ouvert dans
`docs/codeparsers/tickets/2026-11-11-le-nettoyage-du-parent-invalide-un-worktree-de-sous-module.md` ;
revue et garde de nettoyage restent à faire. Pas de nouvelle intervention dans
le garde ni de suppression des restes avant revue. Les suites oracle/provenance
et homonyme restent distinctes, ouvertes également.

## Garde de nettoyage validée pour livraison

Garde Python et procédure commune du lot 91c386388 : revue acceptée, 14 témoins
passés aux deux postes et rejoués par l'orchestration. Ticket d'incident
`docs/codeparsers/tickets/2026-11-11-le-nettoyage-du-parent-invalide-un-worktree-de-sous-module.md`
reste ouvert : garde livrée, état historique de l'index/refs/reflogs de l'ancien
garde non certifié. Pas de nouvelle intervention dans ce dépôt ni GC/prune.
Cleanup des worktrees propres à ce suivi contrôlé par la garde elle-même,
archives et rapports hors cible conservés dans
`~/.cache/codeparsers-notes/cx-garde-nettoyage-livraison/` sur le portable.
