# Surveiller les fichiers — l'index du code suit les éditions faites ailleurs

*11 octobre 2026, session everything declarative. Page courte avant le lot, pour
l'orchestration et pour Lucie, qui l'a demandé : « surveillance des fichiers, on
fait comment avec le MCP ? Claude Code ne nous dira pas quand il fait un edit,
donc il faut surveiller. » Le relevé est fait sur master `f997b3568`, et les
chemins sont relatifs à `extension/rag3weaver/`. Renversable par Lucie.*

## 1. Ce qui existe

- **Réindexer un fichier, ou le retirer, existe déjà** :
  - `code_sync::reingest_file` (`src/code_sync.rs:184`) : la fin de l'instantané
    sur le grain du fichier, l'oubli de ses arêtes sortantes, l'ingestion, puis
    la re-résolution des symboles ;
  - `code_sync::remove_file` (l.278) : les scopes et la ligne `File` retirés ;
  - pendant une indexation entière, `reingest_file` rend `deferred_to` : le
    fichier entre dans le registre « à reprendre » (l.93-148), déjà employé par
    `edit_file`.

  Il ne manque que **qui les appelle** quand le disque change hors de nos
  outils.
- **Une montre existe, pour les déclarations seules** : celle de l'atelier
  (`src/atelier.rs:146`). Elle compare la date de modification et la taille
  toutes les 500 ms. Son fil dort un intervalle entier, son arrêt attend donc
  au plus un intervalle.
- **Les réactions** :
  - un graphe d'en-tête `%% on: <sujet>` et `%% policy: debounce|batch <ms>`,
    monté par l'hôte au niveau de la cellule (`mount_reactions`,
    `reactions_de_la_cellule`) ;
  - réveillé par une sonnette sur l'`EventBus` ;
  - la tâche y lit son curseur par `EventSourceNode` ;
  - une liste blanche de nœuds réactifs (`backend_code::reaction_nodes`), où
    rien ne touche à l'index du code aujourd'hui.
- **Aucun événement « fichier changé »** sur le bus, aucune crate `notify`, et
  aucune notion de retard dans l'état de l'index : `IndexState` ne bouge qu'à
  une indexation.

## 2. La forme

### 2.1 Une source déclarée : `watches`

Le manifeste déclare une section `watches` :

```json
"watches": {
  "code": { "source": "workspace", "every_ms": 250, "settle_ms": 150 }
}
```

- **Les fichiers surveillés sont ceux de la source** (`FileSource::list()`,
  celle de l'index) : on surveille ce qu'on indexe, les ignorés compris.
  `paths` restreint à des sous-chemins, s'il est donné.
- **Un fil par montre**, monté à l'ouverture du backend à côté des réactions,
  et arrêté avec elles (`Backend::shutdown`, et la réouverture de l'hôte).
  L'arrêt réveille le fil (Condvar) au lieu d'attendre la fin de l'intervalle.
- **Le balayage** compare la date de modification et la taille de chaque
  fichier à l'empreinte précédente.
  - Pour un changement, il attend `settle_ms` de stabilité : une sauvegarde en
    plusieurs écritures n'émet qu'un événement.
  - Il calcule ensuite le hachage du contenu, le même que le `content_hash` de
    `File`, et n'émet rien si le contenu n'a pas changé (un `touch`).
- **L'événement** : `Event::FileChanged { watch, path, change, hash, at }`,
  où `change` vaut `created`, `modified` ou `removed`. Il est publié sur un
  sujet neuf, `files`, du bus de la cellule.
- **Plus tard, sans changer la forme** : les notifications du noyau (inotify)
  derrière la même source, si une mesure le demande. Avec l'exécuteur
  concurrent, la même source deviendra aussi un nœud à sortie `Open` (comme
  `peer_in`) : la montre est une source de flux, que le réacteur consomme
  aujourd'hui et qu'un graphe pourra consommer demain.

### 2.2 La réaction du gabarit code

Le gabarit `templates/backends/code` déclare une réaction `reindex_on_change`
(`%% on: files`, `%% policy: batch 300`) : `EventSourceNode(topics='files')`,
puis un nœud neuf `ReindexFileNode`.

- Il appelle `reingest_file` (créé, modifié) ou `remove_file` (supprimé).
- Ce nœud entre dans la liste blanche réactive comme une **capacité nommée**
  (écrire l'index du code), et non sous la forme de `IndexNode` ou
  d'`EditFileNode`.
- Le travail se fait dans la cellule de la base, sous l'hôte.

Une indexation entière en cours n'est pas doublée : `deferred_to` met le
fichier en reprise, et la fin de l'indexation le reprend, comme pour
`edit_file`.

### 2.3 Le retard se dit

- **Un compteur de retard par source** : chemins changés et non réindexés, avec
  l'heure du premier. La montre l'alimente, `ReindexFileNode` le vide.
- **L'état de l'index le dit** : « N fichiers changés non réindexés depuis
  HH:MM ».
- **Un outil interrogé pendant le retard le dit dans sa réponse**, par le point
  commun des lectures (`catalog_read::with_status`, qui préfixe déjà un statut).
  `usages` ou `impact` ne rendent donc pas l'état d'avant comme sûr.

`code_tools.rs` et l'état de l'index sont à l'arbre principal (73) : je lui
propose le diff, il le relit.

### 2.4 En recette seulement

Un crochet Claude Code après une édition (`PostToolUse` sur `Edit`/`Write`), qui
demande la réindexation du fichier tout de suite (par le MCP ou la CLI) : il
raccourcit le retard. La montre reste ce qui tient pour **tout** éditeur.

## 3. Ce que coûte le balayage

La date de modification et la taille de chaque fichier, cache chaud,
mesurées en Python sur ce poste (médiane de cinq) :

| arbre | fichiers | un balayage |
|---|---|---|
| `src/` du moteur | 1 653 | 2,6 ms |
| `extension/rag3weaver/src` | 163 | 0,1 ms |
| le dépôt entier, sans `.git` ni sous-modules | 6 605 | 13,2 ms |

Ces chiffres sont un plafond, puisque Rust est plus rapide. À une seconde de
période, `src/` coûte moins de 0,3 % d'un cœur. Le hachage ne porte que sur
les fichiers dont la date ou la taille a bougé. Le lot remesurera en Rust, par
la montre elle-même, et cache froid.

## 4. Les lots

| Lot | Ce qu'il pose | Témoins, rouges d'abord |
|---|---|---|
| W1 | `watches` au manifeste ; la montre (balayage, regroupement, hachage, arrêt réveillé) ; `Event::FileChanged` sur `files` ; le montage et l'arrêt par l'hôte | un fichier modifié, créé puis supprimé émet un événement chacun ; un `touch` n'émet rien ; trois écritures rapprochées en émettent un ; l'arrêt rend la main sans attendre la période |
| W2 | `ReindexFileNode` et la capacité réactive ; la réaction `reindex_on_change` du gabarit code | un fichier édité à la main, puis `usages` voit la nouvelle ligne sans `index` manuel ; un fichier supprimé, et ses usages disparaissent ; pendant une indexation entière, le fichier est repris à la fin |
| W3 | le compteur de retard ; l'état de l'index et la note des outils (diff proposé à 73) | l'état dit le retard entre l'édition et la réindexation ; un outil interrogé dans cet intervalle le dit dans sa réponse |
| W4 | (recette) le crochet Claude Code | — |

Le lot 4 des agents (`AgentNode`) reprend après W3.

## 5. Ce que la mise en service a appris (11 octobre 2026, le soir)

La montre a tourné sur l'hôte de Lucie et sur celui de luciepc. Ce qui en est
sorti, corrigé chacun avec son témoin et son ticket fermé :

- **La réconciliation au démarrage.** La montre prenait le disque comme
  photographie de départ : un fichier changé entre le dernier index et son
  démarrage n'était jamais repris. Désormais elle compare le disque à
  **l'empreinte que l'index garde par fichier** (`File.content_hash`, le blake3
  du texte lu par l'ingestion) : écart → `modified`, absent de l'index →
  `created`, indexé mais plus sur le disque → `removed`. Un index vide ne
  réconcilie pas : la montre ne remplace pas `index`. Les montres se montent
  **après** les réactions, pour que la réconciliation trouve la réaction à
  l'écoute.
- **Le coût au repos.** La première forme relisait la liste de la source (le
  parcours du workspace avec ses règles d'exclusion) à chaque tour. Un tour au
  repos ne lit plus que **la date des fichiers connus et de leurs dossiers** ;
  la liste n'est relue que quand un dossier bouge (une création, une
  suppression, un renommage) ou toutes les `relist_ms` (60 s). Mesure, bâti de
  test : sur le dépôt entier (6 818 fichiers), une liste 20 à 33 ms, un tour au
  repos 9 à 22 ms. Les compteurs par phase sont dans `index_state`
  (`watches.<nom>` : relectures, dates, hachages, octets, temps, chemins les
  plus hachés).
- **La racine disparue** (un worktree effacé) : la montre se met en pause et ne
  retire rien ; `index_state` le dit, `WorkingTree::list` nomme la racine.
- **Une lecture n'est plus refusée pour un changement localisé** : elle attend
  jusqu'à 1 s un catalogue tenu le temps d'une réindexation.
- **L'arrêt coupe une longue réaction** entre deux fichiers ; la
  réconciliation reprend le reste au démarrage suivant.
- **Le registre « indexation en cours »** de `code_sync` est clé par la racine
  de la source, et non plus par `source_id` (« local » pour toutes les copies
  de travail) : une indexation d'un backend ne met plus en reprise les
  éditions d'un autre.

## 6. Une base copiée d'un autre arbre

La forme retenue pour « chaque arbre indexé, à jour quand on l'édite » : une
base par arbre, née d'un instantané (reflink) de la base de master, puis la
réconciliation au démarrage, qui ne réindexe que les fichiers où l'arbre
diffère.

- **Ce qui est fait** : la réconciliation lit l'index **par chemin relatif à
  la racine que chaque ligne a enregistrée** (`File.cursor`), pas à la racine
  courante. Une base copiée, ouverte sur un autre arbre, ne rend que les
  fichiers qui diffèrent (témoin
  `backend::tests::a_copied_base_reconciles_by_relative_path`).
- **La condition** : tant que l'identité de `File` contient le chemin
  **absolu**, réindexer sous la nouvelle racine crée des lignes neuves à côté
  des anciennes, et un fichier supprimé dans la copie ne retire pas sa ligne
  d'origine. La copie se salit. C'est le lot B4 (chemins relatifs dans
  l'identité, chez G) ; d'ici là, la forme « une base par arbre » n'est pas
  mise en service, et la réconciliation sert les bases existantes.
- **Le témoin qui prouvera la propreté après B4** : une base indexée sur une
  racine A, copiée ; la copie ouverte sur un arbre B qui diffère de quelques
  fichiers (modifiés, ajoutés, supprimés) ; la réconciliation réindexe ces
  fichiers-là seulement ; puis un **recompte** : autant de lignes `File` que
  de fichiers dans B, aucun doublon de chemin relatif, et les usages d'un
  fichier supprimé dans B partis.

**12 octobre, nuit** : B4 est sur master (`6e3561268`), et l'identité de `File` est relative à la
racine. `indexed_files` suit cette règle et ne retranche plus de racine. Le témoin
`a_copied_base_reconciles_by_relative_path` est gardé ; le recompte est le témoin
`e2e_base_par_arbre` de G.

## 7. Où passaient les « 6 s » (12 octobre 2026, la nuit)

Codeparsers a mesuré de bout en bout, sur une base entière (102 469 morceaux, hôte release) : un
renommage sur disque devient visible à `usages` en 7,26 s, dont 5,7 s pendant lesquelles un appel
attend le catalogue tenu par la réindexation d'**un** fichier. La mesure par phase, ici :

- **Le banc.** `backend::tests::measure_one_file_reindexing` (`#[ignore]`) indexe `src/` (180
  fichiers, 15 631 morceaux, embarqueur `hashembedder`). Il réindexe ensuite cinq fois
  `arbre/scope.rs`, dont un scope change à chaque fois, puis retire `arbre/meter.rs` (32 scopes,
  45 morceaux). Il se joue avec `RAG3WEAVER_INGEST_PROFILE=1 … -- --ignored --nocapture`. Les
  quatre premières passes ont tourné sur la lib de 2 h 34 ; la passe finale sur la lib courante du
  magasin (`f6179ade0`, md5 de la lib et de l'extension vector vérifiés contre le magasin).
- **Ce qui dominait** : une seule instruction, la suppression des morceaux du scope changé,
  `UNWIND $uuids AS uuid MATCH (c:Scope_Chunk {_parent_uuid: uuid}) DETACH DELETE c`. Une sonde
  jetable a chronométré chaque étape de `DeleteRecordNode` :

  | étape | avant | après |
  |---|---|---|
  | lire les morceaux d'un scope (par `_parent_uuid` ou par l'arête) | 2 à 8 ms | — |
  | supprimer les morceaux d'un scope (1 morceau) | 2,1 à 2,9 s | ~20 ms (vidage compris) |
  | supprimer les morceaux de 32 scopes (45 morceaux) | 96 à 114 s | 180 ms |
  | relire et supprimer le parent, désindexer le plein texte | < 10 ms | < 10 ms |
  | **réindexer un fichier**, de bout en bout | **2,5 à 3,9 s** | **0,56 à 0,68 s** |
  | **retirer un fichier** de 32 scopes | **96 à 114 s** | **0,40 s** |

- **La cause** est dans le moteur : supprimer une ligne qui porte encore un vecteur sous index
  HNSW coûte environ 2,5 s par ligne, et plus d'un essai à l'autre. Si le vecteur est remis à
  NULL d'abord, la suppression prend quelques millisecondes. Ticket
  `docs/tickets/2026-10-12-supprimer-une-ligne-indexee-hnsw-coute-des-secondes.md`, pour hotfixs.
- **Le contournement**, rag3db seulement, sur le modèle de celui de `write_vectors` :
  - `SchemaDialect::vector_columns_to_clear` décrit la table de morceaux ;
  - `clear_vector_by_parent` remet chaque colonne vecteur à NULL sur les morceaux des parents
    cités, juste avant le `DETACH DELETE` ;
  - c'est fait dans `DeleteRecordNode` et dans `RechunkDeleteNode` ;
  - les lignes partent aussitôt, donc ce qu'un `SET` à NULL par lot ferait perdre à l'index ne
    concerne qu'elles ; `e2e_invariant_des_vecteurs` le vérifie.
- **Ce qui reste**, par réindexation (moyenne de cinq) :

  | phase | durée |
  |---|---|
  | ingérer | 471 ms, dont 249 ms pour File, Scope et Library, et 193 ms pour les symboles et rendez-vous |
  | fin d'instantané, vidage et suppression compris | 113 ms |
  | résoudre les symboles | 29 ms |
  | oublier les arêtes du fichier | 21 ms |
  | analyser le fichier | 9 ms |
  | lister le projet | < 1 ms |

  La cible « sous la seconde » est tenue sur cet index. Sur la base de Lucie, sept fois plus
  grande, c'est codeparsers qui le dira, en rejouant son renommage sur un binaire bâti avec ce
  lot.
- **La ligne `[montre]`.** Chaque fichier que la réaction réindexe écrit, dans le journal de
  l'hôte, le verrou attendu, le verrou tenu et ces phases, sans variable d'environnement. Le
  journal suffit donc à répéter ce partage sur n'importe quelle base.
- **Le contournement est retiré** (12 octobre, après midi). hotfixs a corrigé le moteur
  (690029ab7 : finalizeDelete sans recherche par voisin). Sur la lib rebâtie, sans le vidage :
  11 à 13 ms pour un scope, 5,4 ms par morceau pour 45 morceaux, un fichier retiré en 413 ms
  (307 avec le vidage) et réindexé en 0,43 à 0,59 s, comme avec.
- **À part** : à l'index initial, 309 s sur 329 passaient à écrire 15 631 vecteurs (`write_ms`
  du nœud `embed`), environ 20 ms par vecteur. C'est dans le même ticket ; le contournement ne
  le change pas.

## 8. La période par défaut, et ce que la montre coûte au repos (12 octobre 2026)

Quatre rejeux de codeparsers sur la base entière (102 469 morceaux) ont fait passer la fraîcheur
d'un renommage sur deux fichiers de 7,26 s à 5,21 s : la suppression HNSW contournée, la
réaction par lot, et la fin par l'arête. Sur ces 5,21 s, ~1,5 s passaient avant que la montre ne
voie le changement : la période d'une seconde, plus 300 ms de pose. Les défauts sont désormais de
**250 ms de période et 150 ms de pose**, dans `WatchConfig` et dans le gabarit de code.

Ce que coûte un tour au repos (le test `a_quiet_scan_of_the_engine_tree_costs_less_than_a_listing`,
cache chaud, sous `poste mesure`). Un tour lit les dates des fichiers connus et de leurs dossiers,
pas la liste :

| arbre | fichiers | un tour | à 1 000 ms | à 250 ms |
|---|---|---|---|---|
| `src/` du moteur | 1 654 | 2,0 ms | 0,2 % d'un cœur | 0,8 % d'un cœur |
| le dépôt entier | 6 929 | 8,8 ms | 0,9 % d'un cœur | 3,5 % d'un cœur |

Pour mémoire, la première forme relisait la liste à chaque tour : 0,7 cœur sur 7 310 fichiers,
sur l'hôte de Lucie, le 11 octobre. Un poste qui veut moins remonte la période dans le manifeste.
La forme générique est de réagir sur l'événement du noyau (inotify, par la crate `notify`) en
gardant le balayage comme filet. C'est un lot à part, après les symboles et les jetons.

**Le même jour, l'après-midi : le noyau réveille la montre.** Le cinquième rejeu de codeparsers
donnait 2,29 s de bout en bout, dont ~0,6 s entre l'édition et la prise du lot. La montre écoute
maintenant les événements du noyau (la crate `notify` : inotify sous Linux, FSEvents,
ReadDirectoryChangesW). Un événement sous la racine réveille le balayage, qui reste le juge (dates,
empreinte, pose, porte de l'index) et le filet d'un événement perdu. Au plus un tour par pose,
pour qu'un build qui écrit en rafale ne fasse pas balayer en boucle. Avec les événements, la
période n'est plus qu'un filet (`filet_ms`, 2 000) : le coût au repos baisse d'autant. Sans eux
(limite d'inotify), la montre le dit et balaie toutes les `every_ms`. `WatchStats` compte les
réveils (`wakes`) et dit si les événements sont écoutés (`events`). Témoin : avec une période
d'une minute, une modification est vue en moins de 3 s (rouge avant, aucun balayage).

