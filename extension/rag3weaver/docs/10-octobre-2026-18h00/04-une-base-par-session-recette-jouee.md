# Une base par session — la recette, jouée

11 octobre 2026, 02 h, session mémoire longue. Demandée **écrite et jouée** par
l'orchestration, et c'est ce qui l'a rendue juste : **jouer m'a corrigée cinq
fois**, et aucune des cinq erreurs ne se voyait à l'écriture. Elles sont toutes
dans la page, à leur place.

Joué sur luciepc, sur `~/git_workspaces/rag3db-lourd` (le dépôt rag3db entier),
services d'embarquement locaux, lib et extension du même bâti.

## Ce que ça donne, en un paragraphe

Une session Claude Code branchée sur un serveur MCP rag3weaver, avec **sa propre
base**, voit les douze outils du gabarit `code` et peut demander qui appelle une
fonction du moteur C++. Deux sessions = **deux dossiers, deux index, rien de
partagé** — aucun verrou à négocier, aucun démon à lancer. Le premier index
coûte une vingtaine de minutes ; les suivants sont incrémentaux.

## 1. Le manifeste : copier un gabarit, ne pas en composer un

**C'est la leçon la plus réutilisable de ma nuit.** J'ai écrit sept manifestes en
devinant ce qui était obligatoire, et j'ai perdu sept passes : un `schema`
d'entité omis, une déclaration d'embarquement omise (`load` l'exige **quel que
soit le signal**), `workspace.index` retiré par erreur (il ne commande pas
l'indexation mais **la déclaration du schéma de code** — sans lui, même
`read_file` répond « unknown entity: File »)… Un gabarit livré est la **liste
exhaustive** de ce qui est exigé, et `src/gabarits.rs` la vérifie à chaque passe
de la lib.

**Copiez tout `templates/`, pas le seul dossier du gabarit** — erreur n° 1 de ma
page précédente :

```sh
mkdir -p ~/.config/rag3weaver
cp -r <dépôt>/extension/rag3weaver/templates ~/.config/rag3weaver/mon-projet
```

Le gabarit `code` range ses graphes **hors de son dossier** (`../../tools/*.mmd`).
Copier `backends/code/` seul donne un manifeste qui ne charge pas. Et réécrire
les chemins en absolu n'est pas la réponse — erreur n° 2 : je les ai pointés sur
un worktree (qui a disparu), puis sur un clone resté sur une vieille tête (deux
graphes sur douze). **Copier l'arborescence préserve les chemins relatifs et
rend le manifeste autonome.**

Puis, dans `~/.config/rag3weaver/mon-projet/backends/code/backend.json`, **trois
champs et pas un de plus** :

```json
"database": "/home/vous/.cache/rag3weaver/mon-projet.rag3db",
"vector_extension": "<arbre où le moteur est bâti>/extension/vector/build/libvector.rag3db_extension",
"workspace": { "root": "<la racine du dépôt à fouiller>", … }
```

Vérification qui attrape tout : les douze `graph` doivent se résoudre depuis le
manifeste.

## 2. Le binaire et les variables

```sh
BUILD=<arbre où le moteur est bâti>/build/lecteurs-csv
export RAG3DB_SHARED=1 \
       RAG3DB_LIBRARY_DIR="$BUILD/src" \
       RAG3DB_INCLUDE_DIR="$BUILD/src" \
       CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-mcp \
       CARGO_INCREMENTAL=0
cargo build --features rag3db-native,code,daemon --bin rag3weaver-backend
```

**Les trois premières ne sont pas un détail de confort.** C'est le couple
`RAG3DB_LIBRARY_DIR` + `RAG3DB_INCLUDE_DIR` qui évite de rebâtir tout le C++
(`tools/rust_api/build.rs`), et `RAG3DB_SHARED` qui fait la liaison partagée :
sans elle, **chaque binaire de test embarque le moteur en statique, 1,35 Go
pièce** — la cause du disque plein du 10 octobre. Mesuré : le même travail fait
**49 Go de target au lieu de 125**.

Le binaire pèse 135 Mo avec la liaison partagée.

Pour l'exécution, trois variables :

```sh
export LD_LIBRARY_PATH="$BUILD/src"            # sinon le serveur meurt au lancement
export RAG3DB_ROOT=<arbre où le moteur est bâti>
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880
```

`RAG3DB_ROOT` **ne se dérive jamais du chemin du code** (piège 6 du journal) :
depuis un worktree, le code vit ici et l'extension est ailleurs. Et les adresses
du service sont **complètes**, jamais les ports seuls.

## 3. Ce que coûte le premier index — mesuré

`estimate` d'abord, qui dit ce qu'il retient **et ce qu'il écarte, avec les
raisons** :

```
7172 fichiers retenus (60.7 Mo) : 4135 code, 3037 texte
496 écartés (383.6 Mo) : artefact de build ou données
30 écartés (123.3 Mo) : texte trop gros pour être de la prose
81 écartés (17.6 Mo) : sans extension
20 écartés (2.4 Mo) : généré — marqueur en tête de fichier
modèle : granite-278m — calcul par un service d'embarquement
vecteurs : environ 7 min — au-delà de 5 min, confirmation demandée
tampon du moteur : 8.0 Gio
```

Puis `index` avec `confirm: true`. **Mesuré du 11 octobre, 01 h 33 → 01 h 53 :**

| étape | temps | état |
|---|---|---|
| analyse + plein texte | ~2 min | « plein texte prêt », base à 322 Mo |
| vecteurs | ~18 min | 133 984 morceaux, `granite-278m` |
| total | **~20 min** | base **971 Mo** → 1,18 Go après point de reprise |

**L'estimation de « 7 min » ne couvre que les vecteurs** : l'analyse des 7 172
fichiers et l'écriture en base s'y ajoutent. Dites 20 minutes à qui demande.

**Et le plein texte est prêt avant les vecteurs**, ce qui compte pour l'usage
visé : `usages`, `impact` et le voisinage sont des **traversées de graphe**, pas
des recherches vectorielles. Seul `search_code` par similarité attend la fin.

### Le piège de l'index : gardez la session ouverte

`index` rend **aussitôt** (« indexation lancée en fond ») avec un chemin de
journal, et le travail est un **enfant du serveur**. Fermer le tube le tue —
mesuré deux fois, 79 s puis 12 min au lieu d'aller au bout. Un vrai client MCP
garde son tube ouvert, mais **une session qui se ferme avant la fin perd son
index**. Ticket ouvert :
`docs/tickets/2026-10-11-indexation-enfant-du-serveur-mcp.md`.

En revanche, **une fois l'index fait, la base est persistée** : la session peut
fermer, et la suivante rouvre sans réindexer. Y compris **après un rebâti du
moteur** — vérifié le 11 octobre à 02 h, une base indexée avec l'ancienne
extension s'ouvre avec la nouvelle sans un mot, le contrôle de bâti portant sur
l'extension face à sa bibliothèque et non sur la base.

## 4. Brancher Claude Code

```sh
claude mcp add rag3weaver-code \
  --env LD_LIBRARY_PATH=$BUILD/src \
  --env RAG3DB_ROOT=<arbre du moteur> \
  --env RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880 \
  -- <target>/debug/rag3weaver-backend mcp \
     --manifest ~/.config/rag3weaver/mon-projet/backends/code/backend.json \
     --keys dev
```

**Pas de `.mcp.json` à la racine d'un dépôt partagé** : il est chargé par *toute*
session ouverte dedans — c'est un réglage commun, pas personnel. L'exemple est à
`templates/mcp/claude-code.exemple.mcp.json` pour un dépôt à soi.

Au démarrage, le serveur dit ce qu'il expose :

```
[mcp] code-poste — 12 outils, clés présentées : dev
```

**Douze ou treize selon la tête** : `callees` arrive (branche de codeparsers).

## 5. La question réelle, et sa réponse

**Ce que j'ai demandé d'abord, et qui ne marche pas :**

```json
{"name":"usages","arguments":{"name":"NodeTable::update"}}
```
```
# usages: NodeTable::update
## Définitions (0)
(aucune définition indexée sous ce nom)
```

**Une méthode C++ se cherche par son nom nu, pas par `Classe::méthode`.** Et en
C++ le nom nu est presque toujours ambigu, donc le `path` n'est pas un confort :

```json
{"name":"usages","arguments":{"name":"update","path":"src/storage/table/node_table","limit":3}}
```
```
# usages: update

## Définitions (1) — 45 autre(s) hors du chemin demandé
- function update — src/storage/table/node_table.cpp:619

Nom ambigu : plusieurs définitions. Les usages trouvés par le nom seul ne sont
pas attribués.

## Usages (103) — call 1

### call (1)
- function replayNodeUpdateRecord — src/storage/wal/wal_replayer.cpp:722
```

**Voilà la réponse utile à une session cœur C++** : `NodeTable::update` est
appelée par le rejeu du journal. Et l'outil **avoue son incertitude** au lieu de
rendre une liste trompeuse — 102 des 103 usages ne sont pas attribués parce que
le nom est ambigu, et il le dit.

Le réflexe à donner aux sessions : **toujours un `path`** sur un dépôt C++.

## 6. Deux sessions, deux bases — dit tel quel

**Deux dossiers, deux index, rien de partagé.** Chaque session copie son
`templates/`, pointe son `database` ailleurs, indexe son propre arbre. Aucun
verrou à négocier, aucun démon, aucune coordination. Le coût est double : deux
fois vingt minutes, deux fois 1,2 Go.

Ce qu'on **ne peut pas** faire ainsi : partager une base entre deux sessions.
Une base rag3db n'a qu'un écrivain, et Claude Code lance un processus MCP par
session. La forme prévue pour ça est `--demon`, qui n'est **ni validée ni
réfutée** — ticket `2026-10-11-mode-demon-ni-valide-ni-refute.md`, trois
questions ouvertes.

## 7. Si ça ne marche pas

- **le serveur meurt au lancement** : presque toujours `LD_LIBRARY_PATH` ;
- **« unknown entity: File »** : le manifeste ne déclare pas `workspace.index: "code"`
  (le refus le dit depuis `9dc3f083a`) ;
- **« Catalogue occupé : une indexation le tient en ce moment »** : attendez, ou
  prenez `search_code`, qui balaie les fichiers — le refus propose le repli
  lui-même ;
- **« Could not set lock on file »** : un serveur tient déjà la base. Et pour le
  trouver, **pas `pkill -x`** : le noyau tronque le nom à 15 caractères, donc
  `pgrep -c -x rag3weaver-backend` rend **0** sur un processus bien vivant.
  `pidof`, filtré par `/proc/<pid>/exe`, qui est le seul lien disant quel
  **fichier** tourne ;
- **le serveur sort avec le code 75** : la base demande une réouverture, et il
  l'a dit par un `notifications/message` avant de partir. Relancez-le.

## Ce que cette recette ne dit pas

Le plein texte **seul**, sans vecteurs. `workspace.index_signals` **ajoute** des
signaux, il n'en retire pas, et `needs_embeddings()` est vrai dès que
`workspace.index` est déclaré — donc un embarqueur est exigé à l'ouverture même
pour un usage purement textuel. Comme le plein texte est prêt en deux minutes et
que `usages` n'attend pas les vecteurs, **le besoin pratique est couvert** : on
peut travailler pendant que les vecteurs se calculent. Mais « indexer sans
jamais embarquer » n'est pas déclarable aujourd'hui, et ça mériterait de l'être
pour un dépôt où personne ne veut de recherche par similarité.
