# Des noms écrits dans du Cypher généré, ou lus par l'analyseur, sans échappement

- **État** : ouvert (22 novembre 2026, hotfixs), reste du ticket de l'accent grave doublé, fermé le même jour
- **Famille** : moteur
- **Gravité** : confort. Un nom qui porte un caractère spécial (un accent grave, une apostrophe, un espace) casse une
  instruction générée ou se lit mal ; rien de tel dans les noms de rag3weaver.
- **Atteignable en service** : oui, avec de tels noms.
- **Touche rag3weaver** : non, par lecture (ses noms n'ont ni accent grave, ni apostrophe, ni espace).
- **Signalé par** : hotfixs, le relevé des sites du lot `escaped-backtick-in-names`
- **Pour** : à attribuer

## Ce qui reste

1. **Hors du bâti d'ici, non modifiés à l'aveugle** : l'extension neo4j (`neo4j_migrate.cpp:254`, `:257`, `:359`,
   `:369`, qui écrivent `CREATE NODE/REL TABLE` et `COPY` entre accents graves sans les doubler), et l'outil
   `tools/check_rel_directions/check_rel_directions.cpp:87`, `:98`, `:100`, `:109`. Le correctif est le même :
   `StringUtils::quoteIdentifier`.
2. **Un nom de paramètre entre accents graves** (`` $`p` ``) est pris brut par `transform_expression.cpp:471`
   (`ctx.oC_SymbolicName()->getText()`), sans passer par `transformSymbolicName` : ses accents graves restent.
3. **Des noms écrits sans accents graves dans du Cypher généré** : les tables de relations d'un index vectoriel
   (`create_hnsw_index.cpp:478`, `:480`, `:494-505`, `drop_hnsw_index.cpp:76-80`) ; les noms des paramètres d'une
   macro (`scalar_macro_function.cpp:62-66`) ; les champs de struct dans la chaîne de type
   (`property_definition_collection.cpp:100`, `transform_ddl.cpp:346`).
4. **Des noms écrits entre apostrophes**, qu'une apostrophe dans le nom casserait : `nextval('{}')`
   (`sequence_catalog_entry.cpp`), les options from et to de l'EXPORT de relations (`export_db.cpp:109-110`),
   `CREATE_VECTOR_INDEX('{}', …)` (`hnsw_index_catalog_entry.cpp:41`).

## Pour le fermer

Un témoin par famille (un nom à accent grave ou à apostrophe dans chaque instruction générée, relu après
EXPORT/IMPORT ou recréation de l'index), puis `quoteIdentifier`, ou un équivalent pour les littéraux entre
apostrophes.
