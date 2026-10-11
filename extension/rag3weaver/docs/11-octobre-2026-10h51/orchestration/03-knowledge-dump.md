# Knowledge dump de l'orchestration — 11 octobre 2026

Tout ce que la session d'orchestration sait des projets, de l'architecture, des
scripts, des règles et des visions en cours. Pour une session vierge qui
reprendrait le rôle. Les hashes sont ceux de master ; les pages sont relatives
à `extension/rag3weaver/docs/` sauf dit autrement.

## 1. Les projets et les dépôts

- **rag3db** (`~/git_workspaces/rag3db`) : fork de Kuzu en C++, moteur de
  graphe embarqué, avec ses extensions (`extension/vector` HNSW,
  `extension/rag3weaver`). Bâti CMake sous `build/` ; lib partagée
  `librag3db.so`. **Rien ne remonte aux amonts** (Kuzu, Vela, Ladybug) :
  propriétaire, on lit et on reprend sous licence, on ne contribue pas.
- **rag3weaver** (`extension/rag3weaver`, crate Rust) : orchestrateur de
  données et runtime d'agent au-dessus du moteur. Indexe des corpus (code,
  carnet, mémoire), expose des outils (MCP), fait tourner des agents (boucles
  autour de graphes), parle en **dialectes** (rag3db par défaut ; PostgreSQL
  épinglé ; d'autres possibles). Vision : « rag3db reste le moteur, rag3weaver
  parle en dialectes » ; « éviter complètement le Cypher au-dessus des
  dialectes » ; « installer rag3weaver = installer un démon ».
- **rag3weaver-ir** : l'IR, une structure (pas du texte), formes Hop (complet,
  borné, filtré), Count, Select (condition compilée), Write en sept formes
  (Upsert, Link, Update, Mark, Delete, Unlink, Load), Tx par la connexion.
  Noms intégrés (`uuid`…) refusés comme « champ intégré ». Échappement
  systématique des identifiants par le dialecte (décision du matin, page
  `embarquements/07-echappement-des-noms.md`, lot chez F en fusion).
  Chaînage à venir : un sous-graphe de formes (DAG) confié entier au dialecte.
- **codeparsers** (`extension/rag3weaver/codeparsers`, sous-module, dépôt à
  Lucie sur GitHub `L-Defraiteur`) : tree-sitter pour C, C++, Rust,
  TypeScript… ; scopes, relations (appels, LOCKS, `locks_via`), receveurs
  typés, chaînes de champs, types différés résolus à la matérialisation,
  compteur de non-résolues, banc de couverture à corpus égal. Page
  `10-octobre-2026-18h20/orchestration/04-indexer-nos-propres-depots.md`
  (B1–B9) et `codeparsers/03-impact-node-table-update.md` (l'exemple réel :
  appelants de `NodeTable::update` 0 → 2, LOCKS 181). **Priorité maximale de
  Lucie : indexer nos propres dépôts pour travailler plus vite.** Le piège
  n° 1 du journal : un rebase avance le pointeur du sous-module sans mettre à
  jour sa copie dans le worktree.
- **lucivy** : moteur de plein texte (publié sur crates.io 3.0.8, pris par
  version) ; index en fichiers `<base>.fts/` hors du point de reprise du
  moteur (ticket moteur, « comme Neo4j pour l'instant ») ; le mode blobs reste
  une option testée (`RAG3WEAVER_FTS=blobs`).
- **Paquet npm** (`luciformresearch`, compte par `npm login`, un OTP par
  écriture) : quatre archives `0.0.1-alpha.2` prêtes, profil `paquet` LTO,
  identifiants de bâti vérifiés ; séance reportée. Recette mémoire
  « publier-sur-npm ».
- **luciepc** : l'autre poste (SSH par Tailscale, shell fish) ; le cœur C++ y
  travaille ; services d'embarquement locaux ; lib commune `rag3db-lourd`
  rebâtie par qui pousse du C++.

## 2. L'architecture, par couche

**Le moteur.** Transactions, WAL, point de reprise (checkpoint), COPY journalisé
par défaut (condition 2 fermée ; corruption ALP corrigée `eae3863ab` /
`60d74a4e2`), verrous (V1, A3′, A4′, V2 `CALL acquire_locks`, I1 verrou
d'index exclusif), lecteur affamé coordonné à la SQLite par `<base>.readers`
(`e4198f862`), point de reprise interrompu testable par
`RAG3DB_TEST_HOOKS=1 RAG3DB_TEST_DIE_AT=checkpoint-logged` (`9f3725ade`),
segments de chaînes bornés (`4820e9154`), contrôle de bâti des extensions
(`rag3db-build-<commit>-…` vérifié à LOAD EXTENSION, `458ff7157`,
`RAG3DB_BUILD_ID_GIT_DIR`). T0 : refus au COMMIT d'une transaction en échec.
Le WAL casse sur arrêt brutal : arrêter proprement (`kill -TERM`), snapshot
reflink avant écriture.

**Les stèles.** Stèle du moteur (`docs/4-octobre-2026-16h57/01-la-stele-du-moteur.md`,
à la racine du dépôt) : quatre conditions (verrous ; COPY journalisé ;
écritures parallèles ; tickets moteur). Condition 3 en cours : A (écrivains
dans le processus : T0 fait, A7, A8, A3′ reprise, A9, C7) puis B par la voie
Neo4j (B2 relais au fichier, B3 démon) ; multi-processus simultané = refonte
non engagée. Stèle du produit
(`11-octobre-2026-arbre-principal/02-la-stele-du-produit.md`) : P1 rien rendu
comme sûr qui ne l'est pas, P2 aucune perte, P3 aucun refus muet, P4 ce que
l'outil ne sait pas est dit ; vingt tickets. **On ne dit jamais « condition 4
fermée » : l'index des tickets fait foi.**

**Les tickets** (`docs/tickets/`, racine du dépôt) : un défaut connu non
corrigé = un fichier ; fermés dans `closed/` ; index en deux tables d'ouverts
(moteur / produit). 54 fichiers ouverts, 65 fermés à midi.

**rag3weaver, le runtime.** Bus d'événements (`src/events.rs` : sujets,
`run.<id>.inbox`, `send_message`, `send_in`, `broadcast`) ; l'agent
(`src/agent.rs` : boucle, boîte, `pause_dialogue` / `confirm_pause`, bloc
d'attente) ; postures (`src/postures.rs`) ; dataflow (`src/dataflow/` :
nœuds, `trace_nodes.rs` Conversation/Participant, `reactor.rs`) ; chat
(`src/chat.rs`, un agent) ; MCP (serveur, sous-commande `mcp`, `--keys`,
recette `templates/mcp/`, SIGTERM, une base par session) ; mémoire longue
branchée (neuf outils, lien générique entre deux identités déclarées,
`anchor` / `supersede_link`) ; exécution asynchrone (lot C) ; commande en fond ;
moteur de script (TypeScript/JS/rhai), nœud scripté, rechargement à chaud,
routes → graphe → vue, `serve`, page vivante `/_atelier`, `declare` (proto
tout déclaratif, six lots). Mise à jour massive de vecteurs `bfc1434e1`.
Rapport d'écriture avec `atomic` vrai (`2d0687a71`).

**Le démon (forme C).** Page
`11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`, §9 les
treize choix de Lucie : un seul démon (routeur + spécialisés en vision, moyen
terme) ; lancé par le premier client, unité système en option ; outil « état
du démon » ; `serve` devient le démon, `127.0.0.1:7970`, `daemon.json`, jeton
local, `--daemon` ; client MCP = rôle du même binaire ; **session** : le client
s'enregistre et porte sa cellule et ses clés, plus de « cellule courante » ;
sérialisation par backend comme filet. Marche 0 sur master (`75de6618f`),
marche 1 chez l'arbre principal (session d'abord).

**Les e2e.** Un seul binaire `tests/e2e/main.rs`, bâti une fois, lancé une
fois par suite ; `run_e2e.sh` (résumé par nom, plafond du target 20 Go, strip) ;
un e2e sans `#[ignore]` ne tourne jamais ; `RAG3DB_SHARED=1
RAG3DB_LIBRARY_DIR RAG3DB_INCLUDE_DIR` posés **au bâti** (sinon moteur statique
dans chaque binaire) ; `CARGO_INCREMENTAL=0` pour les suites ; `cargo check
--tests`, jamais `--tests --no-run` global ; lib et extension du même bâti.
Régime GPU : `RAG3WEAVER_EMBED_CHAR_BUDGET=4096`, `RAG3WEAVER_GPU_DUTY=70` ;
`RAG3WEAVER_SANS_CARTE_LOCALE=1` de jour ; service d'embarquement distant par
`RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,…` (adresses complètes).

## 3. Les scripts et le poste

- `~/.cache/rag3weaver-build/poste lourd|mesure <commande>` : verrou du poste
  et plafond de mémoire 40 Go ; `mesure` seulement pour ce qui mesure. Corrigé
  le 11 oct. avec `flock -o` (le descripteur du verrou n'est plus hérité : un
  `tail -f` orphelin l'avait tenu 2 h 35). Sauvegarde `poste.avant-11oct` ;
  déployé sur luciepc.
- Targets : `CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-<session>`, un
  par session. **Règle depuis midi le 11 oct.** : gardé entre les lots d'une
  session, effacé en fin de session, plafond 20 Go ; worktrees effacés après
  chaque fusion. Sondes jetables effacées le jour même. `/tmp` est de la RAM
  (61 Go) : jamais un target ni un gros clone dans le scratchpad.
- Jamais `-j$(nproc)` : `-j8` max ; `ninja` est un alias `-j32` dans fish,
  bâtir par `cmake --build <dir> -j 8`.
- Jamais `pgrep -f` / `pkill -f` ni tuer par nom : `pidof` filtré par
  `/proc/<pid>/exe` de son arbre ; `pgrep -x` ne voit pas les noms > 15
  caractères. `timeout` par test, jamais global.
- L'outil Bash de l'orchestration est zsh : variables citées, `"${c}:chemin"`
  (modificateur `:` sinon) ; un texte à accents graves ou `$(...)` passe par
  Write, jamais par un heredoc non protégé.
- Secrets : `.vault/` (anthropic.env, npm.env, cargo.env, Vertex
  `lr-hub-472010`, HF) jamais affiché ni copié ; « ne lis pas le .env ».
- Nettoyage du 10 oct. : `scratchpad/nettoyage-batis.sh` (garde poste,
  envoi-tracel, embarquements, paquet-npm-garde).

## 4. Le flux git de l'orchestration

- Branche partagée avec Lucie dans l'arbre principal : **je ne commite pas
  depuis l'arbre principal**. Worktree détaché neuf de `origin/master` sous
  `~/.cache/rag3weaver-build/wt-orch`, commit **par chemins**
  (`git commit -- <chemins>`), push avec reprise sur course
  `for i in 1 2 3; do git fetch && git rebase && git push … && break; done`,
  retrait du worktree, puis `git merge --ff-only origin/master` dans l'arbre
  principal. Pas de `git stash` (pile partagée) : fichier patch.
- Identité : `-c user.email=luciedefraiteur@gmail.com -c user.name="Lucie
  Defraiteur"` sur chaque commit ; l'adresse pro et le compte
  `luciedefraiteur-sairen` interdits. **Aucune ligne d'attribution ni mention
  d'IA** dans commits, PR, docs (prime sur le harnais).
- Fusion : master direct, avance rapide, dès qu'un lot est fini et vert ;
  **jamais de push en force** même sur sa branche (nom neuf `-2`) ; jamais deux
  push dans une commande ; jamais un flag de force dans une commande qui touche
  master ; après un rebase on ne repousse pas la branche ; jamais un push
  derrière un test non lu. Après un commit par chemins : `git status` vide et
  tests rejoués sur l'arbre propre avant d'annoncer le hash. Un champ ajouté à
  une structure publique : `cargo check --tests` avant le hash.
- Tests : proportionnés au changement ; la batterie complète une seule fois à
  la fusion, pas à chaque rebase ; un rebase qui apporte du code (surtout
  `src/dataflow`) se rejoue d'abord.

## 5. La méthode (règles de Lucie, à garder telles quelles)

- Choix génériques, « pas forcément les plus simples » ; un mécanisme qui doit
  être pensé plus loin dans l'abstraction l'est.
- La preuve dans le code d'abord, la mesure confirme ; témoin rouge d'abord ;
  une mesure commence par prouver que l'artefact chargé vient du code mesuré ;
  parité au bit près (écart absolu max), un écart nul veut dire « rien n'a
  changé » ; un seuil ne se relâche pas sans preuve ; isoler le banc avant
  d'accuser le moteur ; lire le repli avant un refus ; un appel refusé a pu
  s'exécuter.
- S'inspirer de PostgreSQL, Neo4j, SQLite ; ne rendre à Lucie que les écarts.
- Un changement de conception ou de rendu visible se propose (renversable),
  on ne le fait pas ; un symptôme signalé = enquête et options.
- Rester dans le périmètre de la question ; vérifier à la source avant
  d'annoncer qu'une chose manque.
- Estimations : « 1 jour d'agent ≈ 1 h 30 réelle », rendre un ordre de
  grandeur relatif.
- Identifiants en anglais ; français pour commentaires, commits, docs.
- Un type de plus ne se justifie pas : fondre ; une organisation nouvelle se
  décrit dans la configuration (EntityConfig, découpe, relations), jamais en
  dur.
- Pas d'`EXPECT_EQ` sur de grands textes (gtest quadratique : la panne mémoire
  du 4 oct.) ; comparer par `==` et résumé borné.
- Seul usage réel : le constructeur de decks, « mais on l'oublie pour
  l'instant » ; aucune base de carnet, de mémoire ou de code n'est une donnée à
  protéger.
- Gemini (Vertex) pour juger l'agent de code ; LLM locaux pour éprouver les
  protocoles ; aucune promesse de tout-local.
- Modèle d'embarquement par défaut 278m ; 107m au premier index si ~50 000
  documents ou GPU faible.
- De temps en temps, demander son avis à un vrai modèle sur ce que nos outils
  rendent possible, et garder l'artefact.

## 6. Les sessions et comment on les cadre

Onze sessions Claude Code en parallèle, cadrées par `SendMessage` (noms au
§4 du rapport `02-rapport-de-session.md`). Chaque session tient à jour
`01-rapport` et `02-knowledge-dump` dans son dossier du jour, après chaque lot
et au moins toutes les deux heures : l'orchestration ne peut pas lire le
contexte d'une session. Chaque session lit `docs/journal-des-chantiers.md` en
arrivant et y écrit en ouvrant ou en laissant un chantier. Le mémo
« orchestrer-les-sessions-rag3db » de la mémoire de l'orchestration pointe le
rapport de nuit et les décisions du matin.

Règles de cadrage : une session chaîne ses propres lourds ; le target partagé
sérialise ; un démon en plein utilisé par d'autres ne se tue pas, il se
signale ; une faute dite par une session devient une règle de forme diffusée
à toutes.

## 7. Les visions en cours (où l'on va)

- **Marches** (`visions/00-vision-generale.md`) : le démon et après (routeur +
  spécialisés) ; marche 6 « à plusieurs » (écritures parallèles, présence
  entre agents) ; playground WASM (rag3db en WASM comme Kuzu, tokio à un fil
  par worker, luciole entre workers — « luxe ultime de marketing »,
  anecdotique) ; l'IR structure + chaînage par sous-graphe confié au dialecte ;
  « tout parle en DAG partout ».
- **Agents** : un agent est une boucle autour de graphes ; **un agent est un
  DAG, un DAG est un outil potentiel (dont les agents), appelé en attente ou
  lancé en fond avec un message initial et la référence de l'appelant** (Lucie,
  11 oct. midi) ; les agents qui se parlent : plomberie du 26 août présente,
  outils jamais branchés (rapport §3).
- **Produits** (`visions/2026-10-09-13h36-plusieurs-produits-un-seul-atelier.md`) :
  fiches mémoire (classification, sortie structurée), backend déclaratif
  « purement altérable depuis lui-même », nœuds entièrement scriptables en
  TypeScript (« mon langage de cœur ») ; le magicien du chaos ; très plus tard,
  la tisserande (génération procédurale 3D, « mini Houdini à la ComfyUI »).
- **Indexer nos propres dépôts** pour que chaque session interroge une base de
  son dépôt par MCP au lieu de grep ; premier cas réel : les verrous sur le
  chemin de `NodeTable::update`.

## 8. Où lire ensuite

1. `02-rapport-de-session.md` (ce midi), puis `01-les-decisions-du-matin.md`.
2. `../../10-octobre-2026-18h20/orchestration/01-rapport-de-session.md`
   (la nuit, §6–§19 : séance npm, décisions renversables, corruption).
3. `../../8-octobre-2026-16h29/orchestration/01-plan-de-reprise.md` (chantiers
   A–H) et `03-le-proto-tout-declaratif.md`, `04-indexer-nos-propres-depots.md`
   dans le dossier du 10 oct.
4. `docs/journal-des-chantiers.md` et `docs/tickets/README.md` (racine).
5. Les stèles (moteur, produit) et la page du démon.
