# lucivy 4.4.0 : garder les générations au-dessus de la dernière validée (la voie B)

Session lucivy, 20 novembre 2026. Conception, sans code. Elle répond au cadrage
`docs/coeur-cpp-hotfixs-tickets/20-11-2026/02-cadrage-le-plein-texte-et-le-point-de-reprise.md` et à ses trois
relectures (hotfixs, cœur, arbre). **Le choix entre A et B appartient à Lucie** ; si c'est B, le code part de cette
page. Lu sur lucivy main (4.3.12, edfe422).

## Le problème, côté lucivy

Après chaque commit, lucivy efface tout ce que le nouveau `meta.json` d'un shard ne nomme pas
(`garbage_collect_files`, `src/indexer/segment_updater.rs:186` ; ce qui vit, c'est `list_files`, `:124`).
`IndexWriter::rollback` ne revient qu'au dernier commit. Quand la base de rag3weaver nomme une génération N et que
le dossier est à N+k, il n'y a donc plus rien à quoi revenir : rag3weaver jette le dossier et rebâtit tout depuis les
lignes.

## Ce que fait la 4.4.0

Un **index étiqueté** associe à chaque commit une génération que l'appelant choisit, et garde sur disque toutes les
générations à partir de la dernière que l'appelant déclare validée. On peut y revenir, pour les quatre shards
ensemble, ou l'ouvrir en lecture sans toucher au dossier.

### L'API (sur `ShardedHandle`, `lucivy_core`)

```rust
/// Commits every shard as generation `g` (strictly greater than the last tagged one). The previous tagged
/// generations stay on disk until `release_below`. An untagged `commit()` on a tagged index is refused.
pub fn commit_tagged(&self, g: u64) -> Result<(), String>;

/// The caller's database has durably validated `g`: every tagged generation below `g` may go; their files leave at
/// the next garbage collection. Never deletes `g` itself, nor anything above.
pub fn release_below(&self, g: u64) -> Result<(), String>;

/// Brings the four shards back to generation `g` (kept on disk), under each shard's writer lock: `meta.json` and
/// `_shard_stats.bin` become those of `g`, the readers and the shard dictionary reopen from them, and every
/// generation above `g` is forgotten. All or nothing: a shard that cannot go back makes the call fail with the
/// directory left at its former state on every shard (each shard's former `meta.json` is put back).
pub fn revert_to(&self, g: u64) -> Result<(), String>;

/// The generations on disk, ascending, and the current one.
pub fn tagged_generations(&self) -> Result<(Vec<u64>, u64), String>;

/// Opens generation `g` of a directory for reading only: no writer, no lock taken for writing, nothing written or
/// deleted. What a reader of rag3weaver (`lecture_seule`) serves when the base names `g` and the directory is above.
pub fn open_read_only_at(path: &str, g: u64) -> Result<ShardedHandle, String>;
```

`commit_tagged` passe par le même chemin que le commit d'aujourd'hui (drain, flush, finalize, et la fin de commit
avec ses sauvegardes et son ramasse-miettes). Il ajoute deux choses : l'étiquette dans le payload
(`PreparedCommit::set_payload`), et la copie du méta sous son nom de génération.

### Sur disque, par shard

- **`meta.json`**, comme aujourd'hui : le méta courant, avec `payload = "lucivy-generation:<g>"`.
- **`meta.<g>.json`** : une copie, écrite par `atomic_write` juste après `meta.json`, de chaque génération gardée.
- **`generations.json`**, écrit atomiquement : `{"current": g, "kept": [g0, …, g], "released_below": g0}`. C'est la
  source de vérité du retour.

À la racine du dossier :
- **`_shard_stats.<g>.bin`** à côté de `_shard_stats.bin` : les statistiques BM25 globales de la génération g ;
- **`_generations.json`**, la même liste au niveau du dossier, écrite **après** les quatre shards.

Un index non étiqueté, comme tous les index 4.3, n'a aucun de ces fichiers.

### Le ramasse-miettes garde les générations retenues

`list_files` gagne, pour chaque `g` de `kept` :
- les fichiers des segments que nomme `meta.<g>.json`, avec leurs `.del` de l'opstamp de ce méta ;
- ses générations de dictionnaire et ses paires en attente (`SfxDictionaryMeta::files()`) ;
- `meta.<g>.json` lui-même.

À la racine, `_shard_stats.<g>.bin` pour chaque `g` de `kept`. `release_below(g)` retire ces noms de `kept`, et le
prochain ramasse-miettes efface ce qui n'est plus nommé par personne.

**Avec la compaction et le sursis** : une compaction du dictionnaire est un commit, donc une génération. Les
générations de dictionnaire qu'elle remplace restent tant qu'un méta gardé les nomme. Le sursis vit dans le méta
(`reprieve_generation`) et suit le retour. Revenir à N peut ramener un texte que N+1 avait mis en sursis : il est
alors cité et lisible, rien n'est perdu.

### Le retour, pas à pas (`revert_to(g)`)

Pour chaque shard, sous le verrou de son écrivain :
1. arrêter les fusions en cours (`wait_merging_thread`) ;
2. copier le `meta.json` actuel en `meta.revert-from.json` ;
3. écrire `meta.<g>.json` en `meta.json` (atomic_write) ;
4. rouvrir l'écrivain et le lecteur depuis le disque (`reopen_writer_after` existe déjà, `lucivy_core/src/handle.rs:287`), puis rouvrir le dictionnaire du shard depuis son méta (`SfxDictionary::open`), ce qui refait `next_ids` et vide la table des créations en attente ;
5. réécrire `generations.json` avec `current = g` et `kept` tronqué à g.

Puis `_shard_stats.bin` ← `_shard_stats.<g>.bin`, `_generations.json`, et effacement des quatre
`meta.revert-from.json`. Si un shard échoue, les shards déjà revenus remettent leur `meta.revert-from.json` et
rouvrent : le dossier reste à son état d'avant, et l'erreur est rendue. Un arrêt brutal au milieu laisse des shards à
des générations différentes. Les fichiers `generations.json` le disent, et rag3weaver fait comme aujourd'hui :
`fts_pending:` d'abord, puis le rebâti. Ce cas est rare, il est dit, et il ne perd rien.

Les ids créés par le dictionnaire en N+1 sont abandonnés avec les segments de N+1 : `next_ids` revient à celui du
méta de N.

## La migration 4.3 → 4.4

- **Une 4.4 ouvre un index 4.3** (sans `generations.json`) comme un index non étiqueté : rien ne change. Le premier
  `commit_tagged(g)` l'étiquette : il écrit `meta.<g>.json`, `generations.json` et `_generations.json`. À partir de
  là, l'index est un index étiqueté.
- **Une 4.3 qui ouvre un index étiqueté** : elle lit `meta.json`, qui est un méta valide (le payload est ignoré),
  mais son ramasse-miettes effacerait les générations gardées. Pour l'éviter, `generations.json` porte
  `"format": "lucivy-4.4-tagged"`, et la 4.4 écrit `"index_settings": {"min_lucivy": "4.4.0"}` dans `meta.json`.
  **Une 4.3 refuse alors de l'ouvrir en le disant** : il faut donc un correctif dans la 4.3 (une **4.3.13** qui lit
  `min_lucivy` et refuse avec « index tenu par lucivy ≥ 4.4.0 »). Elle doit sortir avant ou avec la 4.4.0, et les
  hôtes doivent l'avoir avant d'ouvrir un index étiqueté. Sans elle, une 4.3.12 ignorerait le champ (serde par
  défaut) et effacerait.
- Le format des segments et du dictionnaire ne change pas.

## Le coût disque

C'est le delta entre la génération validée la plus ancienne gardée et la courante :
- **le cas courant** : un paquet de synchronisation ajoute quelques segments neufs, quelques Mo ;
- **au pire** : une fusion complète ou une compaction entre N et N+k. L'index entier existe alors deux fois le temps
  d'un `release_below`, soit +850 Mo pour l'index de code de la base du dépôt.

rag3weaver appelle `release_below` dès que la base a validé de façon durable (règle du cœur) : la fenêtre se compte
en secondes.

## Les témoins, rouges d'abord (dans lucivy)

1. `revert_to_brings_back_the_answers_of_g` : commits étiquetés N, N+1, N+2 (k = 2, règle de l'arbre), avec une fusion
   et une compaction du dictionnaire entre eux, puis `revert_to(N)`. Mêmes documents, mêmes réponses (contains,
   parse, flou) et mêmes spans qu'à l'époque de N. Rouge aujourd'hui : l'API n'existe pas, et les fichiers de N sont
   effacés.
2. `the_garbage_collector_keeps_every_kept_generation` : après N+2, les fichiers nommés par `meta.N.json` existent
   tous ; après `release_below(N+2)`, ils partent.
3. `revert_to_is_all_or_nothing` : un échec injecté au troisième shard (point de faute) laisse les quatre shards à
   N+2.
4. `a_crash_in_the_middle_of_a_revert_is_said` : un arrêt simulé entre deux shards. À la réouverture,
   `tagged_generations` dit l'écart, et l'index refuse de s'ouvrir en écriture sans `revert_to` ni rebâti.
5. `open_read_only_at_serves_g_without_writing` : le dossier à N+2, lecture de N. Les réponses sont celles de N, et
   aucun fichier du dossier n'est créé, modifié ni effacé (empreinte avant et après).
6. `the_dictionary_after_a_revert` : des ids créés en N+1, puis `revert_to(N)`, puis un nouveau document qui contient
   les mêmes mots. `check_dictionary` donne 0 perdu (la table des créations en attente est bien vidée).
7. `a_4_3_index_opens_untagged_and_is_tagged_by_its_first_commit_tagged` ; et, dans la 4.3.13,
   `a_4_4_tagged_index_is_refused_by_name`.

## La taille

- **lucivy 4.4.0** : environ 600 lignes (le méta par génération, `generations.json`, le ramasse-miettes, le retour,
  la lecture à g), plus 7 témoins. Environ un jour d'agent.
- **lucivy 4.3.13** : le refus d'un index étiqueté, environ 30 lignes et un témoin. À publier avant la 4.4.0.
- **rag3weaver**, par l'arbre principal : la garde qui revient au lieu de jeter, `release_below` après le COMMIT
  durable, le témoin du cas 810 à k ≥ 2, et la réouverture sans rebâti après un ROLLBACK. Puis le pointeur 4.4.0.

## Ordre de sortie, si c'est B

1. 4.3.13 (le refus), publiée, puis son pointeur dans rag3weaver et les hôtes posés dessus.
2. 4.4.0, PR et CI, publiée à son tour.
3. Le pointeur 4.4.0 et l'étape 2 de l'arbre, dans un même lot rag3weaver, puis la pose des hôtes.
