# 06 — Ce que lucivy sait déjà, et ce qu'on lui demande

26 août 2026, nuit. Suite du [05](05-origine-cellule-domaine.md). Question de
Lucie : « faut relire lucivy le distribué multi-shardé, et son sparse ;
sinon on lui met en cahier des charges ». Fait — deux lectures complètes de
`lucivy` 3.0.5, et le résultat corrige une de mes affirmations.

Les besoins ont été écrits **avant** la lecture, exprès : sinon on
rationalise ce qui existe au lieu de dire ce qu'on veut.

## 1. La correction : la fuite d'IDF n'existe que côté plein texte

J'avais écrit au [05](05-origine-cellule-domaine.md) que « partager l'index
n'est pas partager le fait », en invoquant l'IDF de BM25. C'est vrai du plein
texte. **C'est faux du sparse.**

Le score sparse est un **pur produit scalaire** `Σ q_w × r_w`, et le module
le dit lui-même : *« A dot product is local to a shard: no global statistics
are needed to merge results »* (`sparse_vector/src/sharded.rs:5-7`). Pas
d'IDF, pas d'avgdl, pas de longueur de document, pas de normalisation, pas
de quantification : les poids sont stockés bruts en `f32` tels que
l'appelant les fournit (`mmap_index.rs:46-51`). La fusion inter-shards est
une concaténation, un tri, une troncature (`sharded.rs:673-682`) — exacte
*parce que* le score est local.

Donc deux index sparse construits sur des corpus différents produisent des
scores **directement comparables**. Rien à recalculer, rien qui fuite.

Piège à ne pas confondre : il y a bien un IDF dans le système sparse, dans
le `ShardRouter` (`lucistore/src/shard_router.rs:150-168`) — mais il sert
**uniquement à choisir sur quel shard atterrit un vecteur**. Il change *où*,
pas *combien*.

## 2. Ce qui existe déjà, et qui est mieux que ce que j'espérais

**Un shard est déjà un index complet et autonome.** Pas un sous-ensemble de
segments : un `LucivyHandle` entier, avec son `meta.json`, ses segments, son
schéma, dans `shard_{i}` (`lucivy_core/src/sharded_handle.rs:1479`). C'est
exactement notre R3, et c'est la fondation la plus solide du lot.

**Les statistiques BM25 sont déjà globales à tous les shards du handle**, et
— point crucial — **recalculées à chaque requête**, à partir de searchers
frais (`search_dag.rs:361`, `:424`). Il n'existe aucun cache de statistiques.
Donc changer la composition d'un index ne demande *aucune* invalidation :
c'est notre R2, à moitié offert.

**Ils savent déjà cadrer les statistiques sur un sous-ensemble.**
`AggregatedBm25StatsOwned::with_subset_docs` (`bm25_global.rs:75`) score
« comme si l'index ne contenait que le sous-ensemble autorisé » : `N` devient
la taille du sous-ensemble, et `total_num_tokens` est mis à l'échelle pour
que l'avgdl reste celui du corpus. Le commentaire dit pourquoi : sans ça, un
hit passait de 0,02 à 2,9. C'est précisément le mécanisme dont un domaine
d'agent a besoin.

**Restreindre une requête à un sous-ensemble de shards existe** :
`ShardFilter::Subset(Vec<usize>)` (`search_dag.rs:751-762`). C'est notre R5.

**Et il existe un protocole de fédération auquel je n'avais pas pensé.**
`export_stats()` → `ExportableStats::merge(&[…])` → `search_with_global_stats()`
(`sharded_handle.rs:2533`, `bm25_global.rs:184`, `sharded_handle.rs:2573`) :
**deux index séparés peuvent être cherchés avec des statistiques communes
sans être fusionnés du tout**. Rien n'est copié, rien n'est monté.

Ce que ça veut dire pour nous est joli : la primitive de fédération de
lucivy tombe **exactement** sur notre frontière d'org. Partager les
statistiques revient à divulguer les `doc_freq` de l'autre corpus — donc
c'est légitime entre projets d'une même org (même frontière de confiance),
et jamais entre orgs. Le mécanisme et la politique coïncident, sans qu'on
ait rien à interdire.

## 3. Ce qui manque — le cahier des charges

### L1. Composer un index à partir de shards existants (bloquant)

`shards` est **figé à la création**, dans `_shard_config.json`
(`sharded_handle.rs:1666-1681`, relu tel quel à l'ouverture `:1728-1751`).
Aucune API `add_shard`, `remove_shard`, `attach`, `reshard` — grep exhaustif,
zéro résultat. `apply_sharded_delta` refuse d'ailleurs un `shard_id >= len`
(`:3106-3108`).

Et le routage n'est pas non plus le nôtre : un shard n'est **pas** clé sur
quelque chose de métier, c'est un routeur d'équilibrage qui décide où va un
document (`shard_router.rs:98`). Donc « un shard par origine » n'est pas
exprimable aujourd'hui.

Ce qu'on demande :

> Pouvoir **monter et démonter un shard déjà construit** dans un handle, à
> chaud, sans réindexer — et pouvoir **imposer** le shard d'écriture au lieu
> de le laisser au routeur.

Bonne nouvelle : la matière première est là, **un cran plus bas et
inutilisée par la couche sharding**. Le fork tantivy expose
`merge_indices` (`src/indexer/segment_updater.rs:475`) et surtout
`IndexWriter::add_segment` (`src/indexer/index_writer.rs:375`), qui est
littéralement « attacher un segment déjà écrit ». `lucivy_core` ne les
appelle nulle part.

### L2. Les statistiques doivent suivre le sous-ensemble de shards

`ShardFilter::Subset` restreint *la recherche*, mais l'agrégat BM25 est
construit sur `self.shards` — **tous** —, pas sur les shards actifs
(`search_dag.rs:361`). Donc chercher dans un sous-ensemble score avec les
`doc_freq` et le `N` de ce qu'on ne cherche pas.

C'est bénin dans le cadre actuel (un index = un locataire). Ça ne l'est plus
du tout si un index devient un assemblage de shards d'origines différentes :
les statistiques diraient alors quelque chose de ce qu'on n'a pas le droit
de voir.

> Quand une requête est restreinte à un sous-ensemble de shards, **les
> statistiques doivent porter sur ce sous-ensemble**. Le mécanisme existe
> déjà pour les documents (`with_subset_docs`) ; il manque pour les shards.

### L3. `search_with_global_stats` doit rejoindre le chemin normal

Le point d'entrée du mode fédéré **ne passe pas par le DAG**
(`sharded_handle.rs:2573-2643`) : boucle séquentielle sur shards et segments,
collecte de **tous** les hits dans un `Vec` sans plafond, puis `sort` +
`truncate`. Donc pas de parallélisme, pas de `TopDocs`, pas de `ShardFilter`,
pas d'`allowed_ids`, pas de découpage mémoire — et une consommation
proportionnelle au nombre total de documents appariés.

> Le mode fédéré doit emprunter le même DAG que `search()`. C'est le chemin
> qu'on veut utiliser entre projets d'une org ; il ne peut pas être celui
> qui tient le moins la charge.

### L4. Côté sparse : fusion, et dimensions portables

Le sparse n'a **aucune** primitive de fusion ou d'import — ni `merge`, ni
`add_segment`, ni `bulk_load`. Et l'obstacle n'est pas sémantique (les scores
sont déjà comparables, §1), il est **mécanique** : le `dim_map`
(`token_id → dimension dense`) est local à chaque index (`index.rs:153-156`),
donc absorber un index dans un autre demande de remapper les dimensions.

> Une opération de fusion d'index sparse, avec remappage des dimensions —
> ou, mieux, des **dimensions globalement stables** qui rendent le remappage
> inutile.

À noter : un **design existe déjà** dans lucivy et n'est pas implémenté —
`docs/24-mars-2026-20h35/07-design-sparse-segments-incremental-sync.md`
propose des segments WORM (`meta.json` + `seg_<uuid>.mmap`), une recherche
multi-segments et un merge en tâche de fond. C'est notre L1 côté sparse,
écrit avant nous. Il s'agit de le réveiller, pas de l'inventer.

### L5. Deux défauts à corriger indépendamment de tout ça

1. **`sparse.mmap` est écrit sans atomicité et sans somme de contrôle.**
   `write_mmap_file` écrit directement sur le fichier de destination
   (`mmap_index.rs:186-187`) — pas de temporaire, pas de `rename`, pas de
   CRC, alors que d'autres parties de lucivy utilisent `crc32fast`. Une
   coupure pendant un commit corrompt l'index, en silence.
2. **`_sparse_config.json` porte `deny_unknown_fields` et aucun champ de
   version** (`sharded.rs:46`). Une version qui ajoute un champ rend le
   fichier illisible par la précédente. La compatibilité est structurellement
   cassée pour ce fichier. (Le routeur, lui, fait ça bien : magic `SHRD`,
   version 3, lecture compatible `1..=3` — `shard_router.rs:308`, `:355`.)

Et une incohérence à signaler : `DEFAULT_BALANCE_WEIGHT` vaut `1.0` dans le
routeur, documenté « round-robin, indexation la plus rapide »
(`shard_router.rs:36`), mais `ShardedHandle` passe `unwrap_or(0.2)`
(`sharded_handle.rs:1687`, `:1741`). Les deux défauts divergent ; c'est 0,2
qui s'applique. L'un des deux commentaires ment.

## 4. Le tableau de confrontation

| Besoin | État | Ce qu'on demande |
|---|---|---|
| **R1** monter/démonter un shard à chaud | **non** | L1 — la matière première existe dans le fork tantivy, inutilisée |
| **R2** stats sur l'ensemble monté | **à moitié** — toujours globales, recalculées par requête, mais insensibles au `ShardFilter` | L2 |
| **R3** shard autosuffisant et portable | **oui** — un shard *est* un index complet ; snapshots LUCE | — |
| **R4** lecture seule concurrente | **à moitié** — `open_snapshot` sert un blob en place, mais comme index entier | L1 |
| **R5** restreindre à un sous-ensemble | **oui** — `ShardFilter::Subset` | (dépend de L2) |
| **R6** sparse aux mêmes propriétés | **mieux et moins** : aucune statistique globale (mieux), aucune fusion (moins) | L4 |
| **R7** coût de montage borné | **oui par construction** — aucun cache de stats à invalider ; à surveiller quand le nombre de shards monte | mesurer |
| **R8** format versionné | **inégal** — routeur exemplaire, `sparse.mmap` strict, configs JSON non versionnés | L5 |

## 5. Ce que je ferais maintenant, chez nous

Rien de ce qui précède ne bloque le doc [05](05-origine-cellule-domaine.md).
L'ordre ne change pas :

1. **`Origin` découvert par l'analyse** — chez nous, aucune dépendance à
   lucivy.
2. **Le domaine comme sélecteur** — chez nous aussi, avec `allowed_ids`
   d'abord (qui existe et marche), et `ShardFilter::Subset` quand L1 et L2
   seront là.
3. **Le partage entre projets d'une org** — c'est là que lucivy est
   nécessaire, et c'est aussi là que la fédération par `ExportableStats`
   offre un chemin **sans rien monter du tout**. À évaluer avant L1 : il est
   peut-être suffisant, et il est déjà écrit.

Autrement dit : on a de quoi avancer six mois sans que lucivy bouge, et on a
maintenant une liste précise à lui remettre — écrite depuis l'usage, avec
les numéros de ligne.

---

## 6. Addendum, après la réponse de lucivy ([07](07-reponse-lucivy-cahier-des-charges.md))

Deux corrections reçues, **vérifiées dans le code toutes les deux**, et un
désaccord sur l'ordre que j'accepte.

### 6.1 R5 était trop généreux : le mécanisme existe, l'entrée n'existe pas

`ShardFilter` est `pub(crate)` (`lucivy_core/src/search_dag.rs:796`) et
`Subset` n'est construit que dans le découpage mémoire de `search_internal`
(`sharded_handle.rs:2237`, `:2248`). Aucune méthode publique ne prend un
sous-ensemble de shards. Donc R5 passe de **oui** à **non** : c'est une API
à ajouter, pas un correctif à appliquer.

### 6.2 L2 était faux — et l'aurait cassé

Ma formulation (« les statistiques doivent porter sur le sous-ensemble »)
aurait cassé une invariance qu'on ne voyait pas. Vérifié :

- l'agrégat BM25 est construit sur `self.shards` — tous (`search_dag.rs:370`) ;
- la boucle de prescan v3 travaille sur `self.active` — le sous-ensemble
  (`search_dag.rs:384-387`) ;
- et le chemin batché **prescanne tous les lots en passe 1** avant de
  chercher lot par lot en passe 2 (`sharded_handle.rs:2222`, « Pass 2 —
  search, batch by batch, with the fully prescanned query »).

C'est ça qui rend les scores **invariants au découpage mémoire** : sans
statistiques du corpus entier, le score d'un document dépendrait du lot dans
lequel il tombe.

Donc `Subset` porte **deux intentions opposées** :

| Intention | Statistiques voulues | Pourquoi |
|---|---|---|
| « je découpe mon index par contrainte mémoire » | **corpus entier** | sinon le score dépend du lot |
| « je restreins à un locataire / un domaine » | **sous-ensemble** | sinon ça fuite |

Ce sont **deux variantes**, pas un réglage. L2 se réécrit :

> Exposer publiquement la restriction à un sous-ensemble de shards, **et**
> distinguer explicitement les deux intentions — la restriction par mémoire
> garde les statistiques globales, la restriction par périmètre les cadre
> sur le sous-ensemble.

C'est la bonne leçon de la journée, appliquée à un autre dépôt : *un même
mécanisme qui sert deux intentions doit les nommer, sinon l'une des deux
sera fausse en silence.*

### 6.3 L'ordre : ils ont raison, on inverse

Je mettais L1 en premier et bloquant. Leur ordre — **L3, puis L5.1, puis L2
avec son entrée publique, puis L1 en dernier** — est meilleur pour une
raison que j'avais moi-même écrite au §5 sans en tirer la conséquence : le
chemin qu'on veut réellement emprunter (fédérer entre projets d'une org) est
`ExportableStats`, il est **déjà écrit**, et L3 ne fait que le remettre sur
le DAG. Il peut donc suffire **sans jamais faire L1 ni L2**.

Et L5.1 (temporaire + `rename` + CRC sur `sparse.mmap`) passe devant tout
le reste dans mon esprit : c'est une corruption silencieuse aujourd'hui, pas
une amélioration pour demain.

### 6.4 Leur question, et notre réponse provisoire

« Combien de shards un domaine monte-t-il en pratique ? » — parce que
l'agrégat interroge chaque searcher à chaque requête, donc linéairement.

On ne peut pas encore le mesurer : `Origin` n'existe pas. Mais l'ordre de
grandeur se devine, et il vaut mieux le dire tout de suite qu'attendre :

- **poste de développement réel** (celui-ci) : 26 racines git sous un seul
  répertoire de travail. C'est le haut de la fourchette d'un domaine « tout
  ce que je touche » ;
- **un domaine de travail normal** — un projet et ses dépendances lues :
  **3 à 10 origines** ;
- **un domaine dérivé par défaut** (l'origine du fichier courant, §3 du
  [05](05-origine-cellule-domaine.md)) : **1**.

Donc la fourchette utile est **1 à 30**, pas 1 000. À ces nombres, un
agrégat linéaire sans cache est probablement le bon compromis — et c'est
même un avantage (R7 : rien à invalider au montage). On mesurera pour de
vrai dès qu'`Origin` sera là, et on leur enverra le chiffre.
