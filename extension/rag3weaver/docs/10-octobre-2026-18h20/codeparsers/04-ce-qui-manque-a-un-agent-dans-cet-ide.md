# Ce qui manque à un agent dans cet IDE

*Ouverte le 11 octobre 2026 par la session codeparsers, à la demande de Lucie : « le vrai but
c'est qu'avec ce MCP vous vous sentiez déjà comme dans un IDE adapté à vous ; s'il y a des
choses à revoir, on reverra ; comme des malvoyants qui ont d'autres sens ».*

**C'est cette page qui décide des outils suivants.** Chaque session qui a le MCP du code
(`code-rag3db`, recette dans le rapport de la session codeparsers) y écrit une entrée **chaque
fois qu'elle retombe sur `grep`, une lecture par tranches ou un outil à côté**. Une entrée par
fois, même courte. Ne pas corriger les entrées des autres : en ajouter une.

## Comment écrire une entrée

```
### <date heure> — <session> — <la question, en une phrase>

- **La question posée** : ce que je voulais savoir, dans mes mots.
- **L'outil essayé** : `usages` / `impact` / `callees` / `search_code` / … et ses arguments.
- **Ce qu'il a rendu** : l'essentiel, copié (quelques lignes), ou « rien », ou l'erreur.
- **Ce que j'ai dû faire à côté** : `grep`, `read_file` par tranches, lire un autre fichier,
  deviner…
- **Ce qui manquait** : l'outil, le paramètre ou la réponse qui m'aurait évité ça.
```

## En tête : l'index ne suit pas l'arbre que l'agent édite

*Ajouté le 11 octobre 2026 au soir, à la demande de l'orchestration, après les premiers retours
du cœur. Les retours ne le disent pas, mais il les touche tous. Ticket :
`docs/tickets/2026-10-11-l-index-ne-suit-pas-l-arbre-edite.md`.*

**Le fait** : la base `code-rag3db` indexe l'arbre principal de ce poste tel qu'il était sur
master à 15 h 22. Le cœur édite sa branche dans un worktree sur luciepc. Ses questions portent
donc sur un code qui n'est pas le sien, et ses propres changements n'entrent jamais dans l'index,
montre ou pas. Un IDE indexe l'arbre que l'agent édite.

**Les formes possibles**, à cadrer avec l'arbre principal :
1. **Une base par arbre de travail**, déclarée au manifeste (ou dérivée du chemin de l'arbre),
   montée par l'hôte à la demande, avec la montre dessus. Simple et juste, mais un premier index
   par worktree (ici 12 min 27 s et 1,9 Go pour le dépôt entier), à moins de partir d'un
   instantané reflink de la base de master.
2. **Un index qui suit la branche de l'arbre** : la base sait de quel commit elle part, et
   réindexe le delta (`git diff` du commit indexé à l'arbre courant) à chaque changement de
   branche ou d'arbre. Moins de disque, mais une base ne sert qu'un arbre à la fois.
3. **Pour luciepc** : la montre d'ici ne peut pas monter un arbre distant. L'index doit tourner
   sur le poste de l'arbre, donc un hôte par poste et un client qui sait lequel interroger
   (l'arbre est sur luciepc, l'hôte de luciepc répond).

Les formes 1 et 2 se combinent : une base par arbre, née d'un instantané de la base de master,
puis tenue à jour par le delta.

### La règle de 20 h 25 : aucune réindexation entière d'un backend pendant qu'on l'interroge

*11 octobre 2026, décision de l'orchestration, après deux minutes de blocage du cœur : la
réindexation de `code-principal`, lancée pour le binaire neuf de luciepc, a tenu le catalogue
(plein texte rebâti, index vectoriel tombé puis reconstruit, la montre qui faisait rattraper les
fichiers d'essai sans fin) pendant que le cœur interrogeait ce même backend. Ticket « un appel
d'outil attend derrière un index sans le dire ».*

- **Une réindexation entière s'annonce aux sessions du backend** (`daemon_status` les liste), et se
  fait à une heure convenue, quand elles ne travaillent pas.
- **Un backend dont le binaire a changé sert son index tel quel** : le binaire neuf profite tout de
  suite à ce qui ne dépend pas de la réingestion ; seule la qualité C++ de l'envoi virtuel (les
  gabarits explicites, la récursion) la demande, et elle attend le soir.
- **Les mesures à chaud de la soirée** (luciepc, `code-principal`, la montre) : 11,2 s au plus
  (18 h 42, mesure codeparsers, borne haute) ; moins de 6 s (20 h 10, mesure du cœur, définition
  et appel). Lucie trouve 6 s long pour un seul fichier : le détail par étape (montre, extraction,
  écriture, catalogue) manque — dans le ticket.

### La règle d'aujourd'hui : un worktree et sa base vivent et meurent ensemble

*11 octobre 2026, 18 h, décision de l'orchestration, en attendant « une base par arbre née d'un
reflink ». Le cas qui l'a fait naître : `rag3db-bascule`, indexé à 17 h 02, effacé vers 17 h 50
par le banc qui ignorait qu'il l'était. Le backend `code-bascule` de l'hôte de luciepc pointait
dans le vide : `search_code` rendait « IO error … No such file or directory », `usages` restait
« Catalogue occupé » le temps que la montre retire 6 000 fichiers disparus, et l'hôte a mis
plusieurs minutes à s'arrêter proprement.*

- **C'est la session qui tient le worktree qui crée et retire le manifeste.** Elle l'annonce en
  le créant et en l'effaçant.
- **On n'indexe un arbre que s'il vit plus d'une heure.** Un worktree d'un lot (`rag3db-pk`,
  `rag3db-b2`) ne se monte pas.
- **À l'effacement d'un worktree indexé** : retirer d'abord son manifeste, puis redémarrer l'hôte
  par son unité (aucune route ne ferme un backend), puis effacer le worktree.

**La recette d'une ligne**, sur le poste de l'arbre (luciepc ici), pour un worktree `<arbre>`
(`~/git_workspaces/<arbre>`) :

```
D=~/.config/rag3weaver/code-<arbre>; mkdir -p "$D" && cp -a ~/git_workspaces/rag3db-hote/extension/rag3weaver/templates "$D/templates" && python3 -c "import json,os,sys; p=sys.argv[1]; m=json.load(open(p)); h=os.path.expanduser('~'); m['name']='code-<arbre>'; m['database']=h+'/.local/share/rag3weaver/code-<arbre>.rag3db'; m['embeddings']={'model':'granite-278m','dimensions':768,'address':'127.0.0.1:7878'}; m['vector_extension']=h+'/git_workspaces/rag3db-lourd/extension/vector/build/libvector.rag3db_extension'; m['workspace'].update(root=h+'/git_workspaces/<arbre>', commands='off', read_only=True, locks_via=['acquireLock','acquireLocks','lockRowForWrite','lockIndexForWrite']); [m['tools'].pop(t,None) for t in ('edit_file','run_command')]; json.dump(m,open(p,'w'),ensure_ascii=False,indent=2)" "$D/templates/backends/code/backend.json"
```

Puis `index {"confirm": true}` par un client sur ce
manifeste. Pour le retirer : `mv ~/.config/rag3weaver/code-<arbre> ~/.config/rag3weaver/code-<arbre>.retire`
et `systemctl --user restart rag3weaver-daemon`.

## Les entrées

### 11 oct., 15 h 30 — codeparsers — monter le MCP du code pour rag3db (avant toute question)

Ce n'est pas une question d'agent, mais c'est le premier contact, et chaque point a coûté un
tour :

- **La question posée** : indexer `src/` et `test/` de rag3db par l'hôte.
- **L'outil essayé** : `estimate`, puis `index`, par un client MCP sur stdio.
- **Ce qu'il a rendu, et ce que j'ai dû faire à côté** :
  1. Un workspace en lecture seule (la racine est l'arbre principal, partagé) est refusé tant
     que le manifeste déclare `edit_file`, puis `run_command` : refus nommés et justes, mais il
     faut retirer ces outils à la main de la copie du gabarit. Il n'y a pas de gabarit
     « lecture seule ».
  2. Borner l'index à `src/` et `test/` : impossible. La source refuse les liens symboliques
     (« lien symbolique hors du projet », une garde voulue), et `generated.dirs` exclut un nom
     de dossier partout (il aurait pris `src/extension/`). J'ai indexé le dépôt entier
     (7 310 fichiers, 62 Mo).
  3. **L'hôte ne relit pas un manifeste modifié**, et aucune route ne ferme un backend : après
     chaque correction du manifeste, `estimate` rendait le premier état. J'ai dû changer de
     chemin de manifeste **et** de base (sinon deux backends sur la même base dans un même
     processus). L'ancien backend reste ouvert jusqu'au prochain redémarrage de l'hôte.
  4. `estimate` dit que l'index demandera environ 8 Gio de tampon, exactement le tampon donné
     (8 Gio) : à la limite.
- **Ce qui manquait** :
  - un gabarit `code` en lecture seule, prêt à l'emploi ;
  - des racines multiples, ou des chemins inclus, dans `workspace` ;
  - une relecture du manifeste par l'hôte (ou une route pour fermer un backend), avec un mot
    clair quand le backend servi n'est plus celui du fichier.

### 11 oct., 15 h 40 — codeparsers — « qui appelle NodeTable::update, et qui tient le verrou de l'index avant ? »

**Le banc** : le premier index réel de rag3db par l'hôte. Dépôt entier, 7 310 fichiers retenus
(62,3 Mo ; 4 177 code, 3 133 texte), granite-278m par le service 7979. Lancé à 15 h 22 min 06 s,
fini à 15 h 34 min 33 s : **12 min 27 s**. Le plein texte était prêt en ~30 s (878 082 liens à
poser), puis 135 592 morceaux vectorisés. Base : **1,2 Go** + **728 Mo** de plein texte.
L'hôte (qui tient aussi la mémoire longue) : **9,8 Go de mémoire résidente au pic** (VmHWM),
8,7 à 9,6 Go pendant les vecteurs ; aucun mur, aucun refus. Hôte bâti à 14 h 50 avec codeparsers
`9531860` : **sans** les lots 5 (C++ anonymes, classes exportées) et 6 (nom seul).

- **La question posée** : qui appelle `NodeTable::update` ; qui tient le verrou de l'index avant
  l'écriture.
- **L'outil essayé** : `usages` puis `impact` (`name = update`,
  `path = src/storage/table/node_table.cpp`, `depth = 3`), puis `callees` et
  `usages {name: lockIndexForWrite}`.
- **Ce qu'il a rendu**, tel quel :

  `usages` (0,6 s) :
  ```
  # usages: update

  ## Définitions (1) — 45 autre(s) hors du chemin demandé
  - function update — src/storage/table/node_table.cpp:635

  Nom ambigu : plusieurs définitions. Les usages trouvés par le nom seul ne sont pas attribués.

  ## Usages (103) — call 2

  ### call (2)
  - function set — src/processor/operator/persistent/set_executor.cpp:57
  - function replayNodeUpdateRecord — src/storage/wal/wal_replayer.cpp:746

  ### non attribués (nom ambigu) (101)
  - function main — bindings/cpp/test.cpp:112 (par le nom)
  - module file_scope_15 — bindings/emscripten/js/lucivy-worker.js:229 (par le nom)
  - module file_scope_10 — bindings/nodejs/test.mjs:69 (par le nom)
  - function build_rows — experiments/mtga/card_rows.py:62 (par le nom)
  …(16 lignes du même genre : Python, Rust, JS)
  … 81 de plus
  ```

  `impact` (1,2 s) :
  ```
  # impact: update

  ## Départ (1)
  - function update — src/storage/table/node_table.cpp:635

  Nom ambigu : toutes les définitions sont des départs ; les usages trouvés par le nom seul sont comptés à part, non montrés.

  **4 touchés** (2 à 1 saut, 1 à 2 sauts, 1 à 3 sauts) — dont **0 Tests qui la traversent** ; 101 par le nom seul, non montrés.

  ## Code qui en dépend, à 1 saut (2)
  - function set — src/processor/operator/persistent/set_executor.cpp:57
  - function replayNodeUpdateRecord — src/storage/wal/wal_replayer.cpp:746

  ## Code qui en dépend, à 2 sauts (1)
  - function replayWALRecord — src/storage/wal/wal_replayer.cpp:464

  ## Code qui en dépend, à 3 sauts (1)
  - function replay — src/storage/wal/wal_replayer.cpp:359

  ## Verrous pris sur le chemin (1)
  - `lockRowForWrite` — update (départ, lock)
  ```

  `callees` (0,6 s), la fin :
  ```
  ## Ce qu'il appelle, à 1 saut (8)
  - function lockKeyOfRow — src/storage/table/node_table.cpp:1030
  - function lockRowForWrite — src/storage/table/node_table.cpp:1072
  - function lockIndexForWrite — src/storage/table/node_table.cpp:1084
  - function isUnCommitted — src/transaction/transaction.cpp:224
  - function usesLocks — src/transaction/transaction.cpp:59
  …
  ## Verrous pris sur le chemin (3)
  - `acquireLock` — lockIndexForWrite (1 saut, lock) ; lockRowForWrite (1 saut, lock)
  - `acquireLocks` — acquireLock (2 sauts, lock)
  - `lockRowForWrite` — update (départ, lock)
  ```

  `usages {name: lockIndexForWrite}` (0,4 s) : appelé par `update` (`node_table.cpp:657`) et
  `delete_` (`:738`) ; la déclaration de `node_table.h:201` sort en « AnonymousFunction ».

- **La réponse, après vérification au source** :
  - **Les appelants** : quatre sites, pas deux. `SingleLabelNodeSetExecutor::set`
    (`set_executor.cpp:57`, trouvé) **et `MultiLabelNodeSetExecutor::set` (`:79`, manqué)** du
    côté des requêtes ; `replayNodeUpdateRecord` (`wal_replayer.cpp:746`, trouvé) au rejeu.
    Les sites `:114`, `:134` et `wal_replayer.cpp:803` appellent `RelTable::update` : à juste
    titre absents.
  - **Le verrou de l'index** : **personne ne le tient avant**. `update` le prend lui-même
    (`node_table.cpp:657`), `lockIndexForWrite` puis `lockRowForWrite`, seulement pour une
    ligne déjà validée et sous `usesLocks()`. `lockIndexForWrite` ne prend rien si la table n'a
    pas d'index secondaire chargé ; sinon `LockResource::index(tableID)` en **exclusif**, et
    rafraîchit l'instantané si la transaction n'a encore rien écrit. `delete_` fait de même
    (`:738`). L'insertion (`:560`) tient l'index en exclusif si la table est indexée, en
    partagé sinon, avec la clé, par `acquireLocks`.
- **Ce que j'ai dû faire à côté** : `git grep` des appels `->update(` / `.update(` dans `src/`
  (pour trouver le site `:79` et écarter les `RelTable`) ; lire `node_table.cpp:635-660`,
  `:1072-1092` et `:552-566` (l'ordre et les conditions des verrous) ; lire
  `set_executor.cpp:50-80` (pour savoir quel `set` est lequel). Dix lectures et quatre grep
  pour deux appels d'outil d'1,8 s.
- **Ce qui manquait** :
  1. **Le nom de la classe** : « function set » ne dit pas lequel des quatre `set` de
     `set_executor.cpp`, ni « function update » qu'il s'agit de `NodeTable::update` (l'hôte n'a
     pas le lot 5 ; à revoir après le rebâti).
  2. **Un appelant manqué** (`MultiLabelNodeSetExecutor::set`, `tableInfos.at(…).table->update`) :
     le receveur sort d'un conteneur, sans type ; ni attribué, ni compté comme manqué.
  3. **Le chemin donné ne suffit pas à lever l'ambiguïté** : avec `path`, l'outil retient la
     bonne définition, mais classe toujours 101 usages « par le nom » de tout le dépôt (Python,
     JS, Rust, expériences MTG). Une question sur le moteur C++ voudrait borner au langage ou à
     un dossier (`scope: src/`).
  4. **Le verrou de l'index n'est pas dans `impact`** : la section « Verrous » ne nomme que ce
     que `locks_via` déclare (`lockRowForWrite`), et seulement au départ. `lockIndexForWrite`
     n'y était pas : il a fallu `callees`, où il apparaît, mais sous le nom générique
     `acquireLock`. L'agent doit connaître d'avance les noms des verrous pour les déclarer.
     Manque : les verrous **pris par le départ lui-même**, dans l'ordre, avec la ressource et le
     mode (`index(table)` exclusif, puis `row(table, clé)` exclusif) et la condition
     (`usesLocks`, ligne validée).
  5. **« Qui tient le verrou avant ? »** n'est pas une question que l'outil sait poser : il
     faudrait les verrous pris **par les appelants** avant l'appel (ici aucun), pas seulement
     ceux pris sur le chemin descendant.

### 11 oct., 17 h 20 — cœur C++ — « qui appelle `acquireLocksBeforeExecution`, que je viens de changer, et quels tests traversent `lockIndexForWrite` ? »

- **La question posée** : après le correctif du `CREATE TABLE … AS` sous le mode multi-écrivains
  (`NodeBatchInsert::acquireLocksBeforeExecution`), je voulais la liste des appelants de cette
  méthode virtuelle, et les tests qui traversent `NodeTable::lockIndexForWrite`, pour savoir quoi
  rejouer. Session relancée pour charger le serveur ; `daemon_status` : en marche, trois backends
  ouverts, huit sessions MCP.
- **L'outil essayé** : `usages {name: acquireLocksBeforeExecution, usage: call}` ;
  `impact {name: lockIndexForWrite, path: src/storage/table/node_table}` ;
  `usages {name: update, path: src/storage/table/node_table, usage: call}`.
- **Ce qu'il a rendu** : `usages` — trois définitions (la virtuelle de `PhysicalOperator`, la
  surcharge de `NodeBatchInsert`, celle de `QueryProcessor`), **un seul appel** :
  `QueryProcessor::execute` (processor.cpp:38 → :67). `impact lockIndexForWrite` — 8 touchés
  (`delete_`, `update` à un saut ; le rejeu, `set_executor`, l'aide du banc à deux sauts), « **0
  tests qui la traversent** », et une section « Verrous pris sur le chemin » (`acquireLock` au
  départ, `lockRowForWrite` à un saut) : celle-ci est neuve et utile. `usages update` dans
  node_table — la bonne définition retenue, 2 appels attribués (`set_executor.cpp:57`,
  `replayNodeUpdateRecord`), 88 « non attribués (nom ambigu) » de tout le dépôt.
- **Ce que j'ai dû faire à côté** : `grep -rn acquireLocksBeforeExecution src` — qui montre
  **trois** sites dans processor.cpp : l'appel de :38, **l'appel récursif de :69 sur les enfants
  de la tâche, et l'envoi virtuel de :71** (`sink->acquireLocksBeforeExecution(context)`), le seul
  qui atteigne la surcharge de `NodeBatchInsert`, celle que je venais de changer. Et pour les
  tests : je les connais (InsertLockTest, IndexLockBench), ils traversent `lockIndexForWrite` par
  des requêtes Cypher — `MATCH … SET`, `COPY` — que le graphe d'appels ne voit pas.
- **Ce qui manquait** :
  1. **L'envoi virtuel n'est pas un usage** : `sink->acquireLocksBeforeExecution(…)` n'est ni
     attribué à la surcharge de `NodeBatchInsert`, ni compté comme manqué, et l'appel récursif
     (:69) manque aussi. Une méthode virtuelle changée dans une classe fille rend « aucun appelant »
     — c'est l'inverse de la vérité. Il faudrait : un appel sur une référence à la classe mère
     attribué à toutes les surcharges (ou dit « virtuel, N surcharges »), et la récursion comptée.
  2. **« 0 tests qui la traversent » est faux et dangereux** : les tests du moteur traversent le
     C++ par des chaînes Cypher dans des fichiers `.test` ou des `conn->query("…")` ; le graphe
     d'appels s'arrête au binder. Il faudrait au moins que `impact` dise « les tests par requêtes
     (`.test`, `query(`) ne sont pas suivis » plutôt que « 0 », et à terme relier un opérateur
     physique aux mots-clés Cypher qui y mènent (`COPY` → `NodeBatchInsert`, `SET` → `set_executor`).
  3. **88 non attribués de tout le dépôt** pour un `update` C++ : la même remarque que l'entrée
     de 15 h 40 — borner au langage ou au dossier (`scope: src/`) ; le `path` ne borne que les
     définitions.
  4. **Ce qui va bien** : la section « Verrous pris sur le chemin » d'`impact` est exactement la
     lecture dont j'avais besoin pour le mode multi-écrivains ; `usages` avec `path` lève
     l'ambiguïté sur la définition ; les réponses sont instantanées.

## 11 octobre, 17 h 20 — seconde session cœur C++ (hotfixs), le point de reprise (A7, A8)

- **La question** : qui appelle `stopNewTransactionsAndWaitForOthersNoLock`, et ce qu'appelle
  `writeRequestedCheckpointIfLastNoLock`, deux fonctions du `TransactionManager` que les lots A7 et
  A8 du jour ont changées ou créées.
- **Ce que l'outil a rendu** :
  - `usages stopNewTransactionsAndWaitForOthersNoLock path=src/transaction` : **juste**, deux
    appels (`commit`, `checkpointNoLock`). Le troisième site que j'avais en tête passe en fait par
    `checkpointNoLock`, et l'outil avait raison contre ma mémoire. Les numéros de ligne sont ceux
    d'avant A7 (`:196`, `:91`, `:349`), pas ceux de master.
  - `callees writeRequestedCheckpointIfLastNoLock path=src/transaction` : « aucune définition
    indexée sous ce nom ». La fonction est sur master depuis `f024578d5` (16 h).
- **Ce que j'ai dû faire à côté** : `grep -n writeRequestedCheckpointIfLastNoLock
  src/transaction/transaction_manager.cpp`, pour savoir si le nom était absent du code ou
  seulement de l'index.
- **Ce qui manquait** : **dire l'âge de l'index**. Une réponse « aucune définition » ne se
  distingue pas d'un index en retard. Il faudrait le commit indexé et son heure en tête de chaque
  réponse (« index de `<hash>`, 14 h 05 ; master est à `<hash>` »), et, quand le nom manque, la
  phrase « peut-être plus récent que l'index » plutôt qu'un zéro sec.


### 11 oct., 19 h 05 — cœur C++ — le serveur de luciepc (`code-rag3db-luciepc`) : « qui appelle `lockKeyOfRow`, que je viens de corriger ; que rejouer pour `lockIndexForWrite` ; où est le verrou de fichier du second écrivain (B2) ? »

- **La question posée** : les mêmes qu'à 17 h 20, sur le serveur monté sur luciepc par Lucie
  (session relancée pour le charger), plus une recherche par le sens pour préparer B2.
- **L'outil essayé** : `daemon_status` ; `usages {name: lockKeyOfRow, usage: call}` ;
  `impact {name: lockIndexForWrite, path: src/storage/table/node_table}` ;
  `search_code {query: "le verrou de fichier F_SETLK du second processus écrivain refusé à
  l'ouverture", options: {limit: 5}}`.
- **Ce qu'il a rendu** : `daemon_status` — en marche sur 127.0.0.1:7971, trois backends ouverts
  (`hote-essai/boutique`, `code-principal`, `code-bascule`), une session ; réponses instantanées
  malgré le tunnel. `usages lockKeyOfRow` — **juste et complet** : deux appels, `NodeTable::update`
  (node_table.cpp:658) et `RelTable::lockEndpoints` (rel_table.cpp:225) — le second, je ne l'avais
  pas en tête : c'est le chemin des relations, où le même rafraîchissement d'instantané peut jouer
  (à lire avant de fermer le lot). `impact lockIndexForWrite` — comme ici, mais la section
  « Verrous pris sur le chemin » a **trois** entrées (`acquireLock`, `lockIndexForWrite`,
  `lockRowForWrite`, avec les appelants à un saut) ; toujours « 0 tests qui la traversent ».
  `search_code` par le sens — cinq résultats, **tous pertinents** : `C9_SecondWriterProcessRefused`
  (le témoin à retourner pour B2), le e2e Rust `deux_processus_ne_peuvent_pas_ouvrir_la_meme_base`,
  et `test/api/lecteurs_concurrents_test.cpp` (un second processus lecteur pendant qu'un écrivain
  tient la base), que je ne connaissais pas ; avec un avertissement « parse without boolean
  operators : OR des mots » — la phrase est prise comme un sac de mots, et pourtant le classement
  est bon (vector + bm25).
- **Ce que j'ai dû faire à côté** : rien pour ces trois-là. Pour savoir quel arbre et quel commit
  le serveur indexe (`code-principal` ? `code-bascule` ?), rien ne le dit : les numéros de ligne
  correspondent à master d'aujourd'hui, c'est ma seule preuve.
- **Ce qui manquait** :
  1. **Quel arbre, quel commit** : `daemon_status` nomme des manifestes, pas le dépôt ni la tête
     indexée ; un agent qui travaille sur une branche veut savoir si l'index est celui de master
     ou de sa branche, et de quand (« indexé à <heure>, tête <hash>, N fichiers en retard »).
  2. Toujours « **0 tests qui la traversent** » pour `lockIndexForWrite` : voir l'entrée de 17 h 20
     (les tests passent par Cypher).
  3. La recherche par le sens pourrait dire **pourquoi** chaque résultat est là (les mots qui ont
     porté) — le ★ seul ne l'explique pas ; mais c'est un confort, pas un manque.
- **En une ligne** : sur ces questions-là, l'outil a gagné sur grep — l'appelant `lockEndpoints` de
  `rel_table.cpp` que je n'aurais pas cherché, et un test de l'API que grep sur « F_SETLK »
  n'aurait pas trouvé.

### 11 oct., 17 h 55 (heure luciepc) — cœur C++ — « une édition est-elle retranscrite à chaud ? » (demande de Lucie)

- **La question posée** : déposer un fichier avec une fonction au nom unique dans l'arbre indexé,
  et voir si `usages` la connaît sans réindexation.
- **L'outil essayé** : un fichier `test/essai_indexation_a_chaud.cpp` déposé à 17:55:48 dans
  `/home/lucied/git_workspaces/rag3db` sur luciepc (la racine de `code-principal`) ; puis `usages
  {name: essaiIndexationAChaudDixNeufHeures}`, `search_code` (scan puis indexed), `daemon_status`,
  et `claude mcp get code-rag3db-luciepc` ici.
- **Ce qu'il a rendu** : `usages` — « Catalogue occupé : une indexation le tient en ce moment »
  (17:56, 17:58), puis « aucune définition » (17:57:25). `search_code` scan — **erreur brute** « IO
  error for operation on /home/lucied/git_workspaces/rag3db-bascule: No such file or directory ».
  `search_code` indexed — trois résultats sans lien (set_tinysnb.test, interval.test, openai_llm.rs)
  à ★0,74-0,75 « vector ». `daemon_status` — trois backends `open: true`, dont `code-bascule`.
  `claude mcp get` — **le serveur est lié au manifeste `code-bascule`**, racine
  `/home/lucied/git_workspaces/rag3db-bascule`, un worktree du banc effacé après sa preuve.
- **Ce que j'ai dû faire à côté** : lire la configuration locale du serveur pour comprendre
  qu'il n'indexe pas l'arbre où j'avais écrit ; l'essai est à refaire avec un serveur lié à
  `code-principal` (le fichier d'essai reste en place pour ça).
- **Ce qui manquait** :
  1. **Un backend dont la racine a disparu doit le dire** — dans `daemon_status` (« racine
     absente ») et dans chaque outil, nommément ; pas une erreur d'entrée-sortie brute, et pas
     `open: true`.
  2. **Une lecture ne se refuse pas pour une indexation en cours** : « catalogue occupé » sur
     `usages` devrait servir le dernier état indexé, et dire « indexation en cours depuis N s,
     M fichiers ».
  3. **Un jeton inconnu ne rend pas une liste confiante** : trois chunks sans rapport à ★0,74 ;
     dire « aucune correspondance lexicale, voici les voisins par le sens » avec un score bas.
  4. **Quel arbre, quelle tête** (déjà dit à 19 h 05) : l'agent ne sait pas sur quoi il interroge ;
     ici le serveur regardait un arbre mort.

### 11 oct., 18 h 42 (heure luciepc) — cœur C++ et codeparsers — « une édition localisée est-elle retranscrite vite, l'hôte étant sain ? »

- **La question posée** : la même qu'à 17 h 55, refaite sur `code-principal` après le retour de
  l'hôte (18 h 40 ; Lucie a forcé l'arrêt après 42 min de réaction de la montre sur l'arbre disparu,
  `code-bascule` mise de côté).
- **L'outil essayé** : une seconde fonction ajoutée au fichier d'essai (`mtime` 18:42:18.7), puis
  `usages {name: essaiIndexationAChaudSecondeFonction}` et `usages {name:
  essaiIndexationAChaudDixNeufHeures}` par codeparsers (mon client de session avait expiré au
  démarrage et ne se reconnecte pas sans `/mcp`).
- **Ce qu'il a rendu** : à 18:42:29.9, la première interrogation rendait déjà la définition
  (`test/essai_indexation_a_chaud.cpp:19`) : **11,2 s au plus** après l'écriture (borne haute, la
  boucle n'a commencé qu'alors) ; à 18:42:41, l'appelant aussi (deux appelants de la première
  fonction, dont la seconde, :20). Le premier fichier (déposé à 17:55) était lui aussi dans l'index
  dès le retour de l'hôte (définition :7, appelant :12).
- **Ce que j'ai dû faire à côté** : passer par codeparsers pour interroger ; retirer le fichier à la
  main.
- **Ce qui manquait** :
  1. **Une borne basse** : pour la lire, une boucle d'interrogation doit tourner avant l'écriture —
     à refaire si Lucie veut le chiffre exact ; 11 s est déjà dans l'ordre attendu d'une montre.
  2. **Dire quand** : la réponse d'`usages` pourrait porter « index à jour au <heure> » (ou « N
     fichiers en attente »), pour qu'un agent sache s'il lit l'état d'après son édition.
  3. **Un client de session qui se reconnecte seul** quand l'hôte revient (ici `/mcp` à la main).

### 11 oct., 18 h 45 — codeparsers — « un seul changement localisé doit être rapide » (question de Lucie, mesure pour le cœur)

- **La question posée** : combien de temps entre l'écriture d'une fonction dans l'arbre indexé et
  la première réponse d'`usages` qui la rend ?
- **Le montage** : l'hôte de luciepc (binaire master a3c1ca98f, montre `reindex_on_change`),
  `code-principal` sur `~/git_workspaces/rag3db` ; le cœur ajoute
  `essaiIndexationAChaudSecondeFonction` (qui appelle la première) au fichier d'essai de Lucie
  `test/essai_indexation_a_chaud.cpp`, écrit à 18:42:18.7 (mtime).
- **Ce qu'il a rendu** : à 18:42:29.9, la première interrogation rendait déjà la définition
  (`test/essai_indexation_a_chaud.cpp:19`) — **11,2 s au plus** (borne haute : la boucle n'a
  commencé qu'à cet instant) ; à 18:42:41, `usages essaiIndexationAChaudDixNeufHeures` rendait le
  nouvel appelant (`:20`).
- **Ce qui manquait** : une mesure à borne basse (une boucle qui tourne avant l'écriture), et
  surtout le client du cœur lui-même — son client MCP de session avait expiré au redémarrage de
  l'hôte et ne se reconnecte pas sans un `/mcp` de Lucie : un client qui survit au redémarrage de
  l'hôte (relit le jeton, se reconnecte) manque.
- **Les temps de luciepc, pour mémoire** : premier index `rag3db-bascule` 450 s (pic de l'hôte
  7,9 Go), `rag3db` (mtg-experiments) 490 s (12,1 Go, deux bases ouvertes) ; l'arrêt de l'hôte
  pendant que la montre retirait un arbre effacé : 42 min, forcé par Lucie (ticket de l'arrêt).

### 11 oct., 20 h 10 (heure luciepc) — cœur C++ — « une petite édition, on l'attend combien ? » (Lucie), depuis ma session reconnectée

- **La question posée** : la même mesure, faite moi-même, montre en main, après la reconnexion de
  mon client (`/mcp`) sur l'hôte relevé à 19 h 46.
- **L'outil essayé** : un fichier `test/essai_a_chaud_2.cpp` (une fonction et son appelant)
  déposé dans `~/git_workspaces/rag3db` sur luciepc à **20:10:52.2**, puis `usages {name:
  essaiAChaudVingtHeuresTrente}` aussitôt ; puis le fichier retiré à 20:11:08 et `usages` encore.
- **Ce qu'il a rendu** : la première interrogation, finie avant **20:10:57.9**, rendait déjà la
  définition (`:4`) **et l'appel** (`appelantVingtHeuresTrente`, `:5`) : **moins de 6 s** entre
  l'écriture et une réponse complète (borne haute, une seule interrogation). Après le retrait, pas de lecture :
  l'hôte venait d'être arrêté proprement (journal : « SIGTERM : arrêt propre », 20:11, codeparsers
  pour son index de code-lourd) — « Connection reset by peer » à ma seconde interrogation.
- **Ce que j'ai dû faire à côté** : rien.
- **Ce qui manquait** : rien de neuf — le délai est celui d'une montre, et les deux manques
  d'avant restent (dire « index à jour au <heure> », un client qui se reconnecte seul).

### 11 oct., 20 h 16 (heure luciepc) — cœur C++ — « et le signal lexical seul, sur un fichier frais ? » (Lucie) — un appel suspendu deux minutes

- **La question posée** : les 6 s mesurées valent pour `usages` (l'index structurel, sans
  embarquement) ; Lucie demande si le signal lexical seul (bm25) d'un fichier frais s'obtient plus
  vite, sans attendre les vecteurs — et trouve déjà 6 s long pour relier un seul fichier.
- **L'outil essayé** : fichier `test/essai_bm25_frais.cpp` (un mot rare) écrit à 20:16:29 dans
  `~/git_workspaces/rag3db` sur luciepc, pendant que l'hôte (relevé 20:12, pid 4190522)
  **réindexait code-principal** ; puis `search_code {query: zorglubetteFraicheur, mode: indexed,
  options: {signals: [bm25], limit: 3}}` et `usages {name: zorglubetteFraicheur}`.
- **Ce qu'il a rendu** : **rien — les deux appels sont restés suspendus environ deux minutes**,
  sans « occupé », sans délai, jusqu'à ce que Lucie les interrompe. Pas de mesure.
- **Ce que j'ai dû faire à côté** : retirer le fichier, et renoncer tant que l'index tourne.
- **Ce qui manquait** :
  1. **Un appel ne se suspend jamais** : pendant une indexation, répondre tout de suite « occupé
     (indexation de code-principal depuis N s), réessayez » ou servir le dernier état, avec un
     délai borné côté client (quelques secondes), jamais une attente ouverte.
  2. **Le signal lexical d'un fichier frais devrait être servi avant ses vecteurs** : le texte est
     là dès l'écriture ; si l'index le relit en deux temps (structure puis embarquement), dire
     lequel est à jour, et laisser `signals: [bm25]` répondre sur le premier.
  3. **6 s pour un fichier, c'est long** (Lucie) : à mesurer où passe le temps (montre, extraction,
     écriture en base, relâchement du catalogue) — une édition localisée devrait se voir en moins
     d'une seconde.
