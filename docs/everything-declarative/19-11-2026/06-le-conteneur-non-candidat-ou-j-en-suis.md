# Le conteneur non candidat (2.12) : où j'en suis

Session everything declarative, 19 novembre 2026, 20 h 15. Branche `container-not-candidate`, empilée sur
`codeparsers/stop-words-locals` (11177477e6) pour une seule repasse. La règle et le mécanisme sont décidés par c7 :
voir `04-le-namespace-devant-la-classe-la-mesure.md` et les échanges du soir.

## Fait

- **Le témoin rouge** (656d9a550a) : `backend::tests::a_folded_container_is_not_a_search_candidate`. Trente en-têtes
  ouvrent chacun `namespace zorg {` replié. Rouge : huit namespaces repliés dans les dix premiers de « zorg ».
- **La règle** (575d33824e) :
  - `ChunkingConfig::not_candidate`, générique : un champ de type, ses types, des champs de texte et un seuil ;
  - pour le code, `scope_chunking_with(seuil)` : `namespace` et `module`, `content` plus `docstring`, sous
    `workspace.container_min_chars` (80 par défaut, `CONTAINER_MIN_CHARS`) ;
  - le seuil fait partie du découpage, donc de `version_de_l_analyse` ;
  - `ChunkRecordNode::compute_chunks` et l'écriture du plein texte d'`InsertRecordNode` sautent une ligne exclue.
- **Vert sur un index neuf** : le témoin passe.

## À faire avant le dépôt

1. **Le geste pour une base existante.** Une version déjà écrite n'est pas réécrite : son rendu ne change pas, et
   `derived_content_uuid` la retrouve « inchangée ». Il faut donc la retirer du plein texte et de ses morceaux, sans
   toucher la ligne :
   - **la sélection**, générique : un `Select` de l'IR des versions dont le champ de type est parmi les types, avec
     `_uuid` et les champs de texte ; la règle se juge en Rust par `NotCandidate::excludes` ;
   - **les morceaux** : `Write::Delete` sur `<entité>_Chunk` par `_parent_uuid`, en cascade, avec le compte. Les
     vecteurs partent avec eux, et la dette ne les compte plus (`count_marqueur_manquant` ne compte que les morceaux) ;
   - **le plein texte** : rien ne retire aujourd'hui le document d'une ligne qui reste. Il faut un retrait par
     `delete_by_node_id` (l'offset vient de `NodeIdCache`), et son pendant dans le tampon : une opération « effacer »
     dans `fts_tampon`, appliquée au COMMIT comme les écritures ;
   - **où il se joue** : au début de chaque `sync_source` sur une base à versions. C'est rejouable (rien à retirer la
     seconde fois). Le nombre retiré est compté dans `SourceSyncReport` et dit au journal d'index.
2. **Le seuil, propriété de la base** : la métadonnée `container_min_chars` est posée au premier geste. Un geste joué
   avec un autre seuil est refusé, en nommant les deux valeurs.
3. **Les reconstructions du plein texte** sautent les lignes exclues : `rebuild_fts_step` (catalog.rs ~1633),
   `rebatir_le_plein_texte_d_une_table` (~8654) et `reindex_fts_rows` (record_nodes.rs). Sans cela, une reconstruction
   ramènerait les documents.
4. **Le témoin d'une base existante** : indexer sans la règle (un seuil à 0), puis avec. Les namespaces repliés
   quittent le plein texte et leurs morceaux. Le geste rejoué ne retire rien.
5. **La parité du banc** sur ses autres requêtes : aucun premier résultat perdu. Puis la ligne de nouveautés et le
   dépôt, juste derrière `stop-words-locals`.

## Hors du lot, à noter

- **Postgres** sert le plein texte en natif (tsvector) : la règle n'y joue que pour les morceaux. Le saut du plein
  texte natif demanderait une colonne ou un index partiel ; ce sera un ticket si c7 le veut.
