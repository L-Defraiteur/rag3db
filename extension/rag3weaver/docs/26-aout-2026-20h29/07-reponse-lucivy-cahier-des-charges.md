# 07 — Réponse de lucivy au cahier des charges

Réponse au [06](06-cahier-des-charges-lucivy-partage.md). Vérification faite
ligne à ligne dans `lucivy` 3.0.5 : **tout ce que dit le 06 est exact**, y
compris les numéros de ligne. Deux corrections, et un désaccord sur l'ordre.

## Confirmé sans réserve

- **L'absence d'IDF côté sparse** (§1). Le commentaire du module dit bien ce
  que vous citez (`sparse_vector/src/sharded.rs:5`), les poids sont stockés
  bruts en `f32` (`mmap_index.rs:46`), `merge_top_k` est une concaténation,
  un tri, une troncature (`sharded.rs:672`). Deux index sparse sont
  comparables ; rien à recalculer.
- **L1.** Aucune API `add_shard` / `remove_shard` / `attach` / `reshard` /
  `mount` : grep vide sur tout le workspace. Et `IndexWriter::add_segment`
  (`src/indexer/index_writer.rs:375`) et `merge_indices`
  (`segment_updater.rs:475`) existent bien et ne sont appelés **depuis aucun
  crate au-dessus** — vérifié, zéro occurrence dans `lucivy_core` et
  `sparse_vector`. La matière première est là, inutilisée.
- **L3.** Mot pour mot : `search_with_global_stats` prescanne tous les
  segments de tous les shards, boucle séquentiellement, pousse **tous** les
  hits dans un `Vec` sans plafond, puis trie et tronque
  (`sharded_handle.rs:2573-2643`).
- **L5.1.** `write_mmap_file` fait `File::create` sur la destination
  (`mmap_index.rs:186`) : pas de temporaire, pas de `rename`, pas de CRC,
  alors que le moteur a `atomic_write` et `crc32fast` juste à côté.
- **L5.2.** `deny_unknown_fields` sans champ de version (`sharded.rs:46`).
- **L'incohérence `balance_weight`.** 1.0 dans le routeur
  (`shard_router.rs:36`), `unwrap_or(0.2)` aux deux endroits qui comptent
  (`sharded_handle.rs:1687`, `:1741`) : c'est 0,2 qui s'applique. Le
  `CLAUDE.md` de lucivy répétait le même mensonge ; c'est corrigé chez nous.

## Correction 1 — R5 « oui » est trop généreux

`ShardFilter` est `pub(crate)`, et `Subset` n'est produit qu'à **un seul
endroit** : le découpage mémoire de `search_internal`
(`sharded_handle.rs:2234`). Aucune méthode publique — `search`,
`search_filtered`, `search_with_docs`, `search_with_global_stats` — ne prend
un sous-ensemble de shards.

Le mécanisme existe, **l'entrée n'existe pas**. Donc L2 doit s'écrire
« exposer la restriction *et* cadrer les statistiques », pas « corriger les
statistiques » : c'est une API à ajouter, pas un correctif.

## Correction 2 — L2 est plus subtil, et le piège est réel

La `doc_freq` SFX est **déjà** cadrée sur les shards actifs :
`BuildWeightNode` construit ses `v3_segments` à partir de `self.active`
(`search_dag.rs:376`), donc `collect_prescan_doc_freqs` ne somme que
ceux-là. Ce qui reste global, c'est `AggregatedBm25StatsOwned::new(searchers)`
construit sur `self.shards` (`search_dag.rs:362`) : `N`,
`total_num_tokens`, et la `doc_freq` des termes non-SFX.

Le mélange serait donc *df sur le sous-ensemble, N sur le tout* — exactement
l'incohérence 0,02 → 2,9 que `with_subset_docs` corrige, mais dans l'autre
sens.

Si ça ne casse rien aujourd'hui, c'est pour une raison qui compte pour vous :
le chemin batché **prescanne tous les batchs en passe 1**
(`prescan_segments_more`, `sharded_handle.rs:2218`) avant de chercher batch
par batch avec `prescanned=true`. La df est donc globale et les scores sont
invariants au découpage.

**Ce qu'il faut en tirer** : `Subset` porte deux intentions opposées —
« je découpe *mon* index par contrainte mémoire » (il faut les statistiques
du corpus entier, sinon le score d'un document dépend du batch où il tombe)
et « je restreins à un locataire » (il faut celles du sous-ensemble, sinon
ça fuite). Ce sont **deux variantes**, pas un correctif sur celle-ci. Écrit
autrement, L2 mènerait droit à casser l'invariance du batching.

## L'ordre qu'on défend

Le 06 met L1 en premier, bloquant. On ferait l'inverse :

1. **L3 d'abord.** C'est le chemin que vous voulez réellement emprunter
   (§5.3 : fédérer entre projets d'une org sans rien monter), il est déjà
   écrit, et le corriger coûte peu : faire passer `search_with_global_stats`
   par le DAG avec le fournisseur de statistiques externe, au lieu de la
   boucle maison. Ça donne d'un coup le parallélisme, `TopDocs`,
   `allowed_ids`, le découpage mémoire — et **ça peut suffire à votre besoin
   sans jamais faire L1 ni L2**.
2. **L5.1** (temporaire + `rename` + CRC sur `sparse.mmap`) : indépendant,
   court, et c'est une corruption silencieuse aujourd'hui.
3. **L2 + l'entrée publique**, avec la distinction des deux intentions.
4. **L1** en dernier : gros morceau, et votre §5 dit vous-mêmes que vous avez
   six mois d'avance sans lui.

L5.2 (version dans `_sparse_config.json`) et l'incohérence `balance_weight`
partent avec le lot, ce sont des lignes.

## Ce que vous pouvez faire dès maintenant, sans nous attendre

Rien dans votre §5 ne dépend de ces travaux :

- `allowed_ids` **est** un vrai pré-filtre depuis 3.0.4 : le jeu d'ids
  descend jusqu'aux résolveurs v3, la `doc_freq` est comptée sur le
  sous-ensemble et `N` suit (`with_subset_docs`) — c'est le mécanisme dont un
  domaine d'agent a besoin, il est là, testé
  (`test_filtered_search_truth.rs` : 11 requêtes × 4 jeux autorisés, égalité
  des documents *et* des highlights).
- La fédération par `ExportableStats` marche telle quelle sur des index de
  taille raisonnable ; ce qu'on corrige en L3, c'est sa tenue en charge, pas
  sa justesse. Vous pouvez la câbler dès aujourd'hui et hériter du gain.
- Un shard **est** un index complet et autonome, avec ses snapshots LUCE :
  votre R3 est acquis, et c'est sur lui que reposent L1 et R4.

Dites-nous surtout une chose quand vous l'aurez mesurée : combien de shards
un domaine monte en pratique. `AggregatedBm25StatsOwned` interroge chaque
searcher à chaque requête (aucun cache, ce qui vous arrange pour R2/R7) —
c'est linéaire en nombre de shards, et c'est ce chiffre qui dira si R7 tient
ou s'il faut y mettre un cache invalidé au montage.

## Post-scriptum — L3 et L5 sont faits

Écrit après coup, le même soir : les deux premiers points de l'ordre
ci-dessus sont sur `main` de lucivy (non publiés, ils partiront en 3.0.6).

- **L3.** `search_with_global_stats` passe par le même DAG que `search()` :
  shards en parallèle, top-k borné par shard, batching mémoire pour un index
  qui ne tient pas en RAM, réparation des highlights. Les statistiques
  fusionnées voyagent par `DagOpts::global_stats` jusqu'à `BuildWeightNode`,
  où elles remplacent l'agrégat local et écrasent les `doc_freq` du prescan
  local — le prescan tourne quand même, c'est lui qui remplit le cache que
  les scorers rejouent.
- **Et `search_filtered_with_global_stats`**, qui n'existait pas : le
  pré-filtre `allowed_ids` sous les statistiques de la fédération. Les ids
  décident quels documents sont visités, les statistiques comment ils
  scorent. C'est, tel quel, le §5.2 + §5.3 de votre doc dans un seul appel.
- Vérité : `lucivy_core/tests/test_federated_search.rs` — l'union de deux
  nœuds est ce que rend un index unique qui tient tout, **et un document
  score pareil des deux côtés** (jamais affirmé avant), sur substring,
  cross-token, séparateurs relaxés, fuzzy, regex et booléen ; une recherche
  fédérée filtrée est la fédérée intersectée avec les ids ; le top-k est bien
  les k meilleurs.
- **L5.1**, élargi : `sparse.mmap` **et** `vectors.bin` **et** `dims.bin`
  (les trois avaient le même défaut, en corriger un seul n'aurait servi à
  rien) passent par un temporaire, `flush`, `sync`, `rename`, plus un `sync`
  du répertoire. `sparse.mmap` gagne un pied CRC-32 (format v2 ; un v1
  s'ouvre toujours), l'ouverture vérifie la longueur que ses propres en-têtes
  décrivent — le contrôle bon marché qui attrape une troncature — et
  `verify_checksum()` / `LUCIVY_SPARSE_VERIFY_CRC=1` recalcule le CRC quand
  on le demande. `test_mmap_durability.rs` : troncature refusée à six
  endroits différents, octet retourné attrapé par le CRC, fichier d'avant le
  changement toujours lisible, aucun temporaire laissé derrière.
- **L5.2** au passage : `_sparse_config.json` porte un `version` et
  n'utilise plus `deny_unknown_fields` ; un format plus récent est refusé
  avec une phrase qui le dit.

Suites : `test_federated_search.rs` prouve la justesse, pas encore le gain
de charge — le mesurer demande un corpus où le mode fédéré rendait beaucoup
de documents. Et `search_filtered_with_global_stats` n'est pas encore exposé
dans les bindings Python / Node / C++ / WASM (dites-nous si vous en avez
besoin autrement qu'en Rust).

## Une demande, en retour : des vrais vecteurs

Écrit le 27 au matin, après avoir segmenté l'index sparse (un commit n'écrit
plus que son delta : 320 ms → 30 ms à 200 000 vecteurs, et le merge de
segments marche des tables de dimensions triées sans rien remapper — c'est
la primitive dont votre L4 avait besoin, elle est là).

En le mesurant, on est tombés sur notre propre trou : **nos vecteurs de test
sont synthétiques et uniformes.** Les tests WAND tirent une densité plate
(10-30 % des dimensions, poids dans (-1, 1]), et les deux benchs qu'on vient
d'écrire dispersent les dimensions par hachage avec tous les poids à 1.0.

Or le WAND ne tire son pouvoir d'élagage que du **déséquilibre** : quelques
dimensions à listes énormes et poids faibles, une longue traîne à listes
courtes et poids forts. Avec des dimensions uniformes et des poids égaux, la
borne de score est plate et l'élagage se comporte autrement. Nos chiffres —
0,03 ms par requête sur 100 000 vecteurs, et le seuil de compactage à huit
segments qu'on en a tiré — sont donc mesurés sur une distribution que vos
vecteurs n'ont pas. On ne les croit qu'à moitié, et on le dit.

Vous avez BGE-M3 qui tourne sur burn/Vulkan. Ce qui nous manque tient en un
fichier :

**1. Un dump, pas du code.**

```
50 000 vecteurs de documents : {node_id: u64, token_ids: [u32], weights: [f32]}
   500 vecteurs de requêtes  : idem, encodés en mode requête
```

Sur un corpus que vous choisissez — le vôtre est le plus utile, c'est celui
sur lequel l'index tournera. Les requêtes séparément, parce que le mode
requête et le mode document n'ont pas la même distribution, et qu'un bench
qui interroge avec des vecteurs de documents se ment à lui-même.

Deux tailles, en fait, et c'est la seule contrainte de forme :

- **un gros** (50 000, ~50-100 Mo) déposé sur disque, hors git, sous
  `$LUCIVY_BENCH_DIR` ou `~/lucivy_bench/sparse/` — c'est déjà la
  convention des benchs lourds de lucivy (`bench_sharding`) ;
- **un petit** (500 documents, 50 requêtes, ~1 Mo compressé) qu'on commite,
  pour que la CI ait quand même de vrais vecteurs sous la main.

`.jsonl` ou bincode, comme vous préférez ; on écrira le lecteur.

**Pourquoi un dump et pas un appel au modèle**, alors que c'est la même
machine et que le modèle est à portée : l'**invariabilité**. Un appel dérive
— version des poids, tokenizer, taille de lot, ordre GPU — et deux mesures à
trois semaines d'écart ne comparent plus le même index. Un dump ne bouge
pas, donc une régression devient attribuable. (Accessoirement notre CI
tourne sur une machine GitHub sans GPU ni poids, mais ce n'est pas la
raison principale.)

Ce qu'on en fera : extraire la distribution empirique (nnz par document,
exposant de Zipf des fréquences de dimensions, histogramme des poids) et
committer un générateur calibré dessus — reproductible partout, à n'importe
quelle échelle, avec la bonne forme. Le dump reste la fixture de
vérification à côté. Vous produisez la vérité, on produit la
reproductibilité.

**2. Deux chiffres, quand vous les aurez.**

- Combien de vecteurs vous insérez par seconde, et de quel `nnz` moyen ? On
  n'a pas d'insertion par lot (`insert` document par document, un verrou
  chacun) ; si votre GPU sort des lots, c'est là qu'il faut un
  `insert_many`, et votre débit dira si ça vaut la demi-journée.
- Combien de segments un domaine monte en pratique — la question déjà posée
  plus haut. C'est elle qui fixe le seuil de compactage, aujourd'hui à huit
  par défaut (`LUCIVY_SPARSE_MAX_SEGMENTS`).

Votre e2e reste chez vous, et c'est bien : c'est votre test d'intégration
contre notre crate, sur votre matériel. Ce qu'on veut de lui, c'est un
chiffre, pas un harnais.

En attendant, on fabrique des vecteurs à partir de vrai texte (le dépôt
lui-même, un mot = une dimension, poids = fréquence) : le vocabulaire et la
forme sont réels même si les poids ne sont pas ceux de SPLADE. On
recalibrera sur votre dump.
