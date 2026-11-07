# Le sparse filtré résout ses offsets sur le parent et cherche dans les morceaux

- **État** : ouvert (7 novembre 2026), **non joué** : lu dans le code de master 9dc626c70 par
  l'arbre principal, en écrivant `extension/rag3weaver/docs/07-novembre-2026-arbre/01-ce-que-le-moteur-doit-au-modele-des-vues.md` ;
  pour recherche.
- **Gravité** : réponse fausse, si les deux espaces d'offsets diffèrent (à prouver).
- **Atteignable en service** : oui, si le défaut est réel : tout `search_code` (ou recherche
  structurée) filtré par `scope`, `language` ou `filter_condition` dont le signal sparse est
  allumé.
- **Touche rag3weaver** : oui (`src/dataflow/generic_search_nodes.rs`, `src/read_view.rs`).

## Ce que c'est

Le signal sparse filtré passe à sa poignée lucivy un ensemble d'offsets admis qui sont ceux de la
table **parent** (`Scope`), alors que cette poignée indexe les lignes de la table des **morceaux** :
deux espaces d'offsets qui, a priori, ne se recouvrent pas.

## Le chemin lu

- `SparseSearchNode` : `allowed_ids_for(…)` puis `handle.search_filtered(&sv, limit, ids)`
  (`src/dataflow/generic_search_nodes.rs` l. 1717-1723 ; `src/search.rs` l. 2355-2380).
- `allowed_ids_for` (l. 2885-2910) appelle `resolve_filter_to_ids` (`src/read_view.rs`
  l. 333-384), qui rend `OFFSET(id(n))` des lignes du **parent** qui passent le filtre
  (`MATCH (n:Scope) … WHERE … RETURN OFFSET(id(n))`, `src/dialect.rs` l. 2499-2530).
- La poignée sparse est celle de `target.chunk_table` (l. 1781-1783), et le sparse s'écrit par
  offset de ligne **de morceau** (`src/record_nodes.rs` l. 2232-2240).
- Le BM25 cherche, lui, sur le parent : ses offsets et son index sont du même espace ; il n'est
  pas touché.

## Le scénario qui rendrait de mauvaises lignes

Une table `Scope` de 1 000 lignes et sa table de morceaux de 3 000 lignes. Un filtre n'admet que
le scope d'offset 12 (ses morceaux sont, disons, les offsets 2 410 à 2 413). Le sparse reçoit
`{12}` et ne cherche que le morceau d'offset 12 — le morceau d'un autre scope, qui peut ne
répondre à rien ou répondre à tort ; les morceaux 2 410 à 2 413 ne sont jamais candidats. Le
signal sparse rend alors un résultat hors du filtre, ou rien, et la fusion le mélange aux autres
signaux sans le voir.

## Le témoin à écrire

Sur un backend à signal sparse (gabarit de code avec `sparse`) : indexer quelques fichiers dont un
seul scope répond à une requête sparse ; `search_code` avec un `scope` (ou un `filter_condition`
sur `file_path`) qui n'admet que ce scope ; exiger que les uuids rendus par le signal sparse
(`meta` par signal) soient ceux des morceaux de ce scope, et qu'un filtre qui l'exclut ne le rende
pas. Rouge d'abord, si le défaut est réel ; s'il est vert, le ticket se ferme en disant pourquoi
(une traduction d'offsets que la lecture n'a pas vue).

## Ce qu'il faut pour le fermer

Le témoin ci-dessus ; puis, si le défaut est réel, résoudre les offsets admis dans l'espace de la
poignée qui les reçoit (les morceaux des scopes admis, par la jointure `_CHUNKED_FROM`), pour le
sparse comme pour tout signal qui cherche dans les morceaux.
