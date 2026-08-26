# 15 — L'identité d'un fichier : trois choses qu'on confond

26 août 2026, 9h. Question de Lucie : *« les chemins relatifs, je crois que
c'est une connerie — devrait être une option ; un agent de code local n'en a
pas besoin, un agent nuage oui. Et le problème traditionnel de RAGForge :
j'indexe un dossier, puis un sous-dossier de ce dossier — que doit-il se
passer ? J'indexe un dossier parent d'un dossier déjà ingéré — que doit-il
se passer ? »*

> **Corrigé une demi-heure plus tard** par le
> [16](16-le-monde-est-ouvert.md) : ce document suppose que tout fichier
> intéressant est déjà dans un projet, et c'est faux — « chemin relatif au
> projet » n'a pas de sens pour `/etc/hosts` ni pour le dépôt d'à côté. Ce
> qui tient : l'identité par URI (§3) et la couverture bornée (§4). Ce qui
> est remplacé : la présentation (§5), où le monde ouvert impose des
> chemins auto-descriptifs plutôt qu'une racine unique.

La réponse courte : ce n'est pas une option à ajouter, c'est **trois notions
à séparer**. Une fois séparées, les deux cas de RAGForge n'ont plus besoin
de règle particulière — ils tombent tout seuls.

## 1. Ce qui est confondu aujourd'hui

`File.hashsafe = ["path"]` (`src/code.rs`) : **l'identité d'un fichier est
son chemin relatif à la racine de la source qui l'a ingéré.** Tout découle
de là, et c'est ce qui rend les deux cas intenables :

| J'ingère | Le `File` créé | uuid |
|---|---|---|
| `~/proj` | `src/a.rs` | A |
| `~/proj/src` | `a.rs` | B ≠ A |
| `~` (parent) | `proj/src/a.rs` | C ≠ A ≠ B |

**Le même fichier sur le disque a trois identités.** Trois jeux de scopes,
trois jeux de relations, et une recherche qui rend trois fois la même
fonction. C'est exactement le symptôme qu'on vient de corriger côté chunks
([13](13-la-session-comme-graphe.md) §6 bis) — ici il vient d'un cran plus
haut.

Et il y a déjà, dans le schéma, **les pièces de la réponse, inutilisées** :
`File.absolute_path` (rempli, jamais lu pour l'identité) et
`FileSource::cursor()` (`worktree:/home/…`, `snapshot:t`) qui nomme la
source.

## 2. Les trois notions

| Notion | La question | Aujourd'hui |
|---|---|---|
| **Identité** | qu'est-ce qui fait que deux fichiers sont le même ? | le chemin relatif — d'où les doublons |
| **Couverture** | de quoi cette ingestion est-elle responsable ? | implicite, d'où les suppressions dangereuses |
| **Présentation** | qu'est-ce qu'on montre au modèle, et qu'est-ce qu'on accepte de lui ? | le chemin relatif, d'où les deux erreurs mesurées |

Les mélanger, c'est ce qui fait qu'« indexer un sous-dossier » n'a pas de
réponse. Séparées, chacune en a une, et elles ne se contredisent pas.

## 3. Identité : une URI de source

> **Un fichier est identifié par l'URI que sa source lui donne**, pas par sa
> position relative à la racine d'une ingestion.

C'est `cursor()` + le chemin **dans le projet** :

- `file:///home/lucied/proj/src/a.rs` pour un arbre de travail ;
- `git://github.com/x/y@a1b2c3/src/a.rs` pour un dépôt distant cloné ;
- `snapshot:demo/port.rs` pour un instantané.

`File.hashsafe` devient `["uri"]`. Alors :

- **sous-dossier d'un dossier déjà ingéré** → mêmes URI, mêmes uuid : c'est
  une **ré-ingestion d'un sous-ensemble**. Les fichiers changés sont mis à
  jour (le `content_hash` tranche), les autres ne bougent pas, et les
  relations vers l'extérieur du sous-arbre restent valides parce qu'elles
  pointent des URI, pas des chemins ;
- **dossier parent d'un déjà ingéré** → les fichiers connus sont reconnus,
  seuls les nouveaux s'ajoutent. L'index grandit, rien ne se duplique ;
- **la même machine, deux clones** → deux URI, deux identités. C'est
  **voulu** : ce sont deux copies, elles peuvent diverger. Pour qu'un index
  soit partageable entre machines, la source doit donner une URI
  indépendante de la machine — d'où `git://…@<commit>`, et c'est le
  `FileSource` qui choisit son schéma. C'est le bon endroit.

## 4. Couverture : une ré-ingestion n'élague que chez elle

Aujourd'hui, ré-ingérer supprime les scopes disparus d'un fichier
(`reingest_file`). À l'échelle d'un dossier, la même logique dirait « les
fichiers absents de ce que je viens de lire ont disparu » — et effacerait
tout le reste du projet si on ingère un sous-dossier.

> **Une ingestion déclare sa racine, et n'élague que sous elle.**

Une entité `Root { uri, cursor, ingested_at }` et une relation
`File —UNDER→ Root` suffisent. Une ré-ingestion de `file:///proj/src`
compare avec les `File` sous cette racine, pas avec la base entière. Deux
racines emboîtées coexistent sans se marcher dessus, et « qu'est-ce qui est
indexé ? » devient une question qu'on peut poser.

## 5. Présentation : relatif au **projet**, pas à la source

C'est là que la remarque de Lucie porte, et les mesures le confirment : les
**deux** modèles ont écrit des chemins relatifs au *dépôt*, pas à la source.

| Modèle | Ce qu'il a tapé | Ce qu'il fallait |
|---|---|---|
| Gemini | `src/dataflow/node_factories.rs` | `node_factories.rs` |
| Qwen3-Coder | `grep(path_prefix='src/')` | pas de préfixe |

Ils n'ont pas tort : ils écrivent ce qu'un humain écrit en regardant un
dépôt. C'est **notre** convention qui est étroite — « relatif à la racine de
la source », où la source peut être `…/src/dataflow`.

Donc :

- **le chemin de présentation est relatif au projet**, pas à la portion
  ingérée. Une source qui ne couvre qu'un sous-arbre expose quand même
  `extension/rag3weaver/src/dataflow/port.rs` ;
- **un outil accepte les trois formes** — projet, source, absolu — et les
  **normalise**. Un chemin qu'on sait résoudre ne devrait jamais être une
  erreur ; le « vouliez-vous dire » reste pour ce qui est vraiment inconnu ;
- **l'absolu est une option d'affichage** (`paths: project | absolute`),
  utile à un agent local qui passe les chemins à un shell. Deux raisons de
  ne pas en faire le défaut : un index cesse d'être partageable, et la
  disposition de la machine part chez le fournisseur du modèle avec chaque
  requête.

Ce qui répond exactement à la distinction posée : **le local peut vouloir de
l'absolu ; le nuage ne le peut pas.** Ce n'est pas relatif *contre* absolu,
c'est « relatif à quoi » — et la bonne réponse est « au projet », pour les
deux.

## 6. Ce que ça change, concrètement

| | Aujourd'hui | Après |
|---|---|---|
| `File.hashsafe` | `["path"]` | `["uri"]` |
| `File.path` | relatif à la source | relatif au projet |
| `File.absolute_path` | rempli, jamais lu | rempli quand la source est locale, sert à l'option d'affichage |
| `FileSource` | `cursor`, `list`, `read`, `write` | plus `project_root()` et `uri(path)` |
| Ré-ingestion | élague ce qu'elle ne voit pas | élague sous sa racine |
| Outils | un seul format accepté | trois acceptés, un affiché |

## 7. L'ordre, et le test qui tranche

1. **`uri` sur `File`**, `hashsafe = ["uri"]`, `FileSource::uri(path)`.
   *Test* : ingérer `proj`, puis `proj/src` — le nombre de `File` ne bouge
   pas, et `search` rend une seule fois `a.rs`.
2. **Chemins de présentation relatifs au projet** et normalisation à
   l'entrée des outils. *Test* : `read('extension/rag3weaver/src/dataflow/port.rs')`,
   `read('port.rs')` et le chemin absolu rendent **le même fichier** ; le
   « vouliez-vous dire » ne se déclenche plus que sur l'inconnu.
3. **`Root` et l'élagage borné.** *Test* : ingérer `proj`, ingérer
   `proj/src` où un fichier a disparu — seul celui-là part, ceux de
   `proj/docs` restent.
4. **L'option d'affichage** `paths: project | absolute`, réglage du graphe
   et non de la fiche ([13](13-la-session-comme-graphe.md) §6).

Les étapes 1 et 2 suppriment les doublons et les deux erreurs mesurées ;
elles valent d'elles-mêmes. La 3 est ce qui rend « indexer un dossier puis
un sous-dossier » sûr, et pas seulement correct.
