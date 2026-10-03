# Arbre principal — ce que je sais

Ingestion du code, synchronisation, chargement en masse, requêtes par lot.
État au 3 octobre 2026, minuit.

## Où sont les choses

- `src/code.rs` : schéma du code (`File`, `Scope`, `Library`, `Symbol`,
  relations `RELATIONS`), `analyze_with` (codeparsers en parallèle), et les
  ingestions :
  - `ingest_code_jusqu_a` : par paquet ;
  - `ingest_code_differe` + `finir_les_relations_differees` : en masse ;
  - la couche de rendez-vous : `DEFINES`/`MENTIONS` vers `Symbol`,
    `materialiser_les_symboles`, `resoudre_les_symboles`. Abstention sur un nom
    à plusieurs définisseurs, sauf mention typée dont le type désigne le
    parent (noms comparés sans paramètres génériques).

  Ce fichier est indexé par le banc : c'est du corpus vivant.
- `src/code_sync.rs` (neuf exprès, hors corpus) : `reingest_file` (fin
  immédiate sur le grain du fichier), `remove_file`, `sync_source`
  (`RelationsMode::Bulk|PerBatch`, phases `Nodes`, `Relations`, `Resume`,
  `Done`, marque `relations_pending:`). Le registre « à reprendre » :
  `note_change`, `edit_file_shared`. Le rapport : `files_resumed`,
  `bulk_load_refused`.
- `src/catalog/synchronisation.rs`, `aside.rs` : la synchronisation déclarée
  (`SnapshotConfig` `scope`/`fine_scope`), la mise de côté, la marque à
  l'écriture `{session}+w`.
- `src/dialect.rs` : toutes les requêtes. **`unwind_par_cle` / `cle_de_ligne`**
  sont la seule fabrique des formes par lot. `alter_add_column_default`
  n'écrit pas `DEFAULT NULL`. `copy_links_from_csv` et `copy_nodes_from_csv`
  ont les mêmes options de lecture : `escaped_newlines=true`,
  `auto_detect=false`, `null_strings=[CSV_NULL]`.
- `src/dataflow/record_nodes.rs` : `InsertRecordNode` (COPY seulement sur une
  table vide), `LinkRecordNode` (COPY au-delà de `COPY_SEUIL` = 200 liens par
  relation, sémantique MERGE gardée), `EmbedNode`, `cellule_csv`,
  `REPLI_EN_MASSE`.
- `src/cypher_blob_store.rs`, `src/buffered_blob_store.rs` : les fichiers de
  lucivy en base (`_index_blobs`). Le tampon sert les lectures en attente ;
  `flush` ne pousse que les écritures en attente.

## Tests qui comptent

- `e2e_code_sync` (14) : synchronisation, modes de relations, édition pendant
  l'indexation (comparée à un index bâti à neuf, arête par arête avec leur
  multiplicité), replis muets.
- `e2e_code` (25) : seuils réels sur `src/dataflow`. Ne jamais relâcher un
  seuil sans prouver ce qu'il mesurait.
- `e2e_plans_par_lot` : `EXPLAIN` de chaque forme par lot, aucun
  `CROSS_PRODUCT`, et exécution.
- `e2e_copy_liens` : colonnes nommées, valeurs piégées après 800 lignes (le
  renifleur ne lit que le début).
- `e2e_mesure_sync_source` : mesure seulement, hors batterie.

## Mesures

- src/ du moteur (1 643 fichiers, 18 175 scopes, 95 944 relations) :
  - par paquet : 66 → 47 s ;
  - en masse : 59 → 37 s.
- Dépôt entier (rag3db-eb, 6 840 fichiers, 76 680 scopes, 438 104 relations) :
  643 → 499 s (COPY des liens) → 352 s (jointures). Postes à 352 s :
  - blobs : 119 s ;
  - graphe : 101 s, dont flush_fts 29, chunk_insert 15, insert 14, chunk_link 9,
    points de reprise ~30 ;
  - symboles : 56 s ;
  - `parse_project` : 46 s ;
  - vidages en route : 27 s ;
  - chargement final : 12 s ;
  - relire la base : 11 s ;
  - marquer : 9 s.

## Défauts du moteur connus

Cas minimaux faits ici :
- **DROP_VECTOR_INDEX rejoué du WAL** : laissait un index non chargé dans la
  table. Corrigé (`f5acca417`).
- **`ALTER … DEFAULT NULL`** fait refuser tout COPY qui omet la colonne
  (« vector with ANY type »). Contourné, pas corrigé.
- **Renifleur CSV** : avec `auto_detect`, il ne voit que le début du fichier.
  Contourné par `auto_detect=false`.
- **`MATCH (n {k: item.champ})`** fait un produit cartésien. Contourné par
  `unwind_par_cle`.
- **SET d'un vecteur vers un autre** : perd des lignes dans HNSW (rag3db-e3).
  Le SET depuis NULL est sain.
- **Rejeu sans `LOAD EXTENSION`** après un point de reprise : la garde 1
  (`fcd9a7882`) laisse l'index « behind its table » ; à reconnaître à
  l'ouverture.
- **WAL de taille 0** relu à la réouverture : intermittent, 1 sur 9.

## Essayé sans succès, ou à ne pas refaire

- Passer les liens en `CREATE` pour éviter le refus « type VARIABLE » : inutile,
  c'était l'ordre des `WITH`.
- Croire un test vert du premier coup : les quatre tests de l'édition, la
  pile, le repli muet ont tous été prouvés par mutation ou en remettant
  l'ancien code. Le premier test des replis passait à tort (les avertissements
  d'`ingest_entities_jusqu_a` n'étaient pas ramassés).
- Tests de reprise : aucun ne tue vraiment un processus avant de rouvrir.
  Voir journal §6.
