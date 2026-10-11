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
  "code": { "source": "workspace", "every_ms": 1000, "settle_ms": 300 }
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
