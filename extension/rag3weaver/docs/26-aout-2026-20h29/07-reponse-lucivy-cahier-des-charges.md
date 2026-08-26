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
