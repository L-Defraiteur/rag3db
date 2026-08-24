# lucivy : `drop_index` + récap du jour (24 août, soir)

Complète les docs 06 et 08. **Épinglez le submodule sur `a31dcf5`.**

## 1. Nouveau : `ShardedHandle::drop_index()`

Le dernier trou du mapping est comblé — plus de « list + delete par préfixe »
chez vous :

```rust
handle.drop_index()?;   // consomme le handle : close() puis destruction totale
```

- ferme proprement (commit, drain des merges, libération des locks) ;
- détruit **tout** côté storage : les blobs de chaque shard
  (`Lucivy_{index}/shard_i`) **et** les fichiers racine (`_shard_config.json`,
  `_shard_stats.bin`) qui vivent, eux, sous le nom d'index **nu** — la
  subtilité de préfixe qu'un appelant aurait ratée à moitié ;
- marche sur `BlobShardStorage`, `FsShardStorage` (supprime le répertoire) et
  `RamShardStorage` ; testé store vide + répertoire disparu
  (`v3_drop_index_leaves_nothing`).

→ Remplacez votre équivalent de `DROP_LUCIVY_INDEX` par ça (schema change,
rebuild d'entité, `Catalog` drop). La ligne du tableau du doc 04 est mise à jour.

Au passage : le **drop de document** n'a jamais manqué — `delete_by_node_id`
pose une tombstone, la policy de merge réécrit et récupère l'espace, LUCIDS
transporte le `.del`. Rien à faire de spécial.

## 2. Rappel de l'état à `a31dcf5` (tout ce qui vous concerne depuis le 04)

| Sujet | État | Commit |
|---|---|---|
| `parse` réparé (OU par mot×champ / QueryParser si syntaxe booléenne) — **retirez votre contournement** | doc 08 | `0d70904` |
| Warnings « fields ignoré », « parse routé », + erreur field/fields réécrite | doc 08 | `0d70904` |
| `_node_id` estampillé automatiquement, id contradictoire refusé | doc 06 §1 | `ce03ac6` |
| `add_document_json` (champs par nom, erreurs qui listent le schéma) | doc 06 §2 | `ce03ac6` |
| Config stricte (clés inconnues = erreur nommant les valides) + `from_stored_json` tolérant à la réouverture | doc 06 §3 | `32ca1dc` |
| Lazy loading optionnel (`with_load_mode(Lazy)`) — vous avez déjà `blob_len`/`load_range` | doc 06 §4 | `4fa729e` |
| `impl BlobStore for Arc<T>` (votre `DynBlobStore` supprimé) | doc 06 §5 | `f7dd5c2` |
| `drop_index` | ce doc | `a31dcf5` |

Suites lucivy à `a31dcf5` : lib 1415/1415, lucivy-core complet vert (mêmes 2
échecs pré-existants de bench_sharding, hors sujet), bindings natifs compilent.

## 3. Toujours côté lucivy (rien à attendre de vous)

Publication crates.io 2.1.0 (votre `[patch.crates-io]` reste le bon montage
d'ici là) ; exécution du binding emscripten C (sans objet pour vous : vous
compilez `lucivy-core` en Rust dans votre wasm).

## 4. Git : pousser sans bascule de compte (le taff tourne en parallèle)

Le compte sairen ne doit apparaître nulle part ici. Le montage qui fait que ça
marche tout seul, vérifié aujourd'hui — à préserver, pas à « améliorer » :

- **Identité des commits** : la config **globale** est l'email sairen (pour le
  taff) ; chaque repo perso a un override **local** `user.email =
  luciedefraiteur@gmail.com`. C'est déjà posé sur les quatre repos de
  `~/git_workspaces` (lucivy, rag3db, llama.cpp, vrrpgeditor). Un commit ne
  contient QUE name/email/dates — si l'override local est là, rien de sairen
  ne peut s'y inscrire. Avant tout premier commit dans un **nouveau** repo :
  `git config user.email luciedefraiteur@gmail.com` d'abord.
- **Authentification des push** : `~/.ssh/config` sépare par alias d'hôte,
  avec `IdentitiesOnly yes` (SSH n'essaie jamais une autre clé) :
  - `Host github.com` → clé perso `id_ed25519_github` → `ssh -T
    git@github.com` répond `Hi L-Defraiteur!` ;
  - `Host github-sairen` → clé taff `id_ed25519_sairen`, utilisée SEULEMENT si
    un remote pointe explicitement sur `git@github-sairen:...`.
  L'aiguillage se fait donc par **l'URL du remote**, pas par un état global à
  basculer : nos remotes sont en `git@github.com:...` → clé perso, toujours.
- Conséquence : commits locaux = zéro trace hors machine ; push = clé perso
  vers un repo perso ; aucun `gh auth switch`, aucun credential helper à
  toucher. Ne jamais mettre de trailer/mention d'outil dans les messages de
  commit (règle projet, l'historique a été réécrit une fois pour ça).
