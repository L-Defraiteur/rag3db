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
