# Le filtre du signal creux compare des offsets de parents à des offsets de morceaux

- **État** : corrigé par le lot recherche-trois-lots (7 novembre 2026, recherche) : filter_resolve_chunk_offsets dans les deux dialectes, le creux filtré par les offsets des morceaux des parents admis ; témoin e2e_simple_entity::le_filtre_du_signal_creux_porte_sur_les_morceaux_du_parent_admis
- **Gravité** : réponse fausse. Une recherche filtrée (scope, language, `filter_condition`) admet
  sur le signal creux des morceaux hors du filtre et en écarte de bons.
- **Atteignable en service** : oui, pour un backend dont une entité porte le signal `sparse` ; pas
  pour le backend de code, qui ne cherche pas en creux.
- **Touche rag3weaver** : oui (`SparseSearchNode`, `search_sparse_via_backend`).
- **Famille** : produit

## Ce que c'est

`SparseSearchNode` résout le filtre en offsets sur `target.parent_table`
(`allowed_ids_for` → `resolve_filter_to_ids`, `src/dataflow/generic_search_nodes.rs`,
`src/read_view.rs`), puis les passe tels quels à la poignée creuse de `target.chunk_table`
(`search_sparse_via_backend`, `src/search.rs` : `handle.search_filtered(&sv, limit, ids)`). Or la
poignée range chaque morceau sous l'offset de **sa** ligne dans la table des morceaux
(`src/dataflow/record_nodes.rs`, `embed_get_offset(table)` puis `handle.insert(offset, …)`). Les
deux espaces d'offsets ne coïncident pas : le filtre laisse passer les morceaux dont l'offset
égale par hasard celui d'un parent admis. Le plein texte cherche sur le parent et n'est pas
touché ; le dense filtre par sa propre projection.

## Recette

Pas de Cypher brut : le défaut est dans le produit. Une entité découpée à signal `sparse`, deux
parents de plusieurs morceaux chacun, un filtre qui n'admet que le premier ; comparer les uuids des
morceaux rendus par le seul signal creux (`signals: ["sparse"]`) à ceux du premier parent.

## Le témoin

À écrire (e2e, avec `MockSparseEmbedder` ou l'`EmbedDaemon` du dépôt qui sert le creux) : la
recette ci-dessus ; rouge si un morceau d'un autre parent passe ou si un morceau du parent admis
manque alors qu'il est le plus proche.

## Ce qu'il faut pour le fermer

Traduire les offsets admis du parent en offsets de ses morceaux avant `search_filtered` (ou
résoudre le filtre directement sur la table des morceaux par leur `_parent_uuid`), comme le dense
le fait par sa projection ; le témoin vert ; la ligne de l'index passée aux fermés.
