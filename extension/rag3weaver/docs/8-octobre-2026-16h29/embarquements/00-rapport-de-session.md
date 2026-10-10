# Embarquements — rapport de session (chantier F)

Mis à jour le 10 octobre 2026 au soir, avant le nettoyage du disque. L'état
d'avant est dans
`../../3-octobre-2026-23h31/embarquements/01-rapport-de-session.md`.

## Fait, sur master

| Quoi | Où | Commit |
|---|---|---|
| Le contrat du dialecte : la page avant le code | [01-le-contrat-du-dialecte.md](01-le-contrat-du-dialecte.md) | `2f2f8e77c` |
| Les capacités déclarées (`DialectCapabilities` : cypher, transactions, bulk_load, structured_fields) ; deux tickets PostgreSQL ; le compteur `copy_journal_fallbacks` dans la mesure | `src/dialect.rs`, `src/filter.rs`, `docs/tickets/2026-10-10-*` | `2e67ce3e0` |
| L'inventaire du Cypher au-dessus des dialectes (42 sites), l'IR en cinq formes ; les décisions de Lucie | [02-le-cypher-au-dessus-des-dialectes.md](02-le-cypher-au-dessus-des-dialectes.md) | `981780f14`, `414248e2d` |
| La série d'avant bascule (lib qui fuyait) | [03-la-serie-d-avant-bascule.md](03-la-serie-d-avant-bascule.md) | `4c708ace6` |
| La porte unique : un corps par défaut écrit en Cypher, sur un dialecte sans `cypher`, rend une instruction d'un seul mot qui nomme la méthode | `src/dialect.rs` | `752e858b4` |
| La crate `rag3weaver-ir` : `Value` (l'ancienne `CypherValue`), `QueryParam`, `FieldType`, l'arbre d'un filtre, `Scope` ; réexportés aux anciens chemins | `ir/` | `41b869ba4` |
| La forme `Hop`, traduite par rag3db, refusée en la nommant ailleurs | `ir/src/form.rs`, `src/dialect.rs` | `ba11e003b` |
| `graph_walk::neighbors`, le voisinage et le pas « aussi », les usages directs passent par `Hop` (parité au caractère près, suites du graphe de code vertes) | `dataflow/graph_walk.rs`, `neighborhood_nodes.rs`, `usage_nodes.rs` | `4a4f8e935`, `fdb43e804`, `26bc4207f` |
| La série de confirmation du basculement : fichiers +1 % sous COPY journalisé, blobs ramenés par le forçage de rag3weaver | [04-la-serie-de-confirmation.md](04-la-serie-de-confirmation.md) | `38e4b1758` |
| La forme `Count` (lignes, degré par uuid), les degrés de `graph_walk` par elle | `ir/src/form.rs`, `dataflow/graph_walk.rs` | `27c541207` |

## En cours, sur la branche `ir-hop-libre` (`d885a6833`, non fusionnée)

`Hop` à départ et arrivée sans table (`Hop::untyped`), et deux colonnes :
l'étiquette et le nœud entier. `fetch_related` (FetchRelatedNode,
GroupFrameNode) et la transition de `react_nodes` passent par lui ; le chemin
du catalogue (`catalog.rs`, à A) garde `fetch_related` dans le dialecte rag3db
le temps qu'A passe le sien. Les tests unitaires sont verts (100). **Les
suites ne sont pas jouées** : e2e_code, e2e_scope, e2e_catalogue_gabarits,
structured_payloads (les gabarits de recherche qui chargent les voisins) et
e2e_reacteur. Ici la parité n'est plus au caractère près (les alias changent
dans le texte) : elle se prouve par les lignes rendues.

## Ce qui reste, dans l'ordre

1. Jouer les suites de `ir-hop-libre`, puis la fusionner.
2. Les sites de `Hop` à un seul uuid (`definitions_query`,
   `pivot_usages_query`, les voisins de `code_tools` et `code.rs`), puis
   `declarations_of` (parité par les lignes).
3. `Select`, avec le compte filtré de `composable_results`. Ensuite `Write`,
   qui doit naître de l'ingestion (le vrai client), pas d'une table système
   (note de la session mémoire).
4. Le contournement du pool PostgreSQL (une connexion tenue le temps d'une
   transaction), après la fusion de `execution-asynchrone` par la session
   recherche.
5. La preuve vivante sur PostgreSQL (`e2e_postgres`, puis la batterie du
   contrat) attend que Lucie lance le conteneur `pgvector/pgvector:pg17` sur
   le port 5433. Ni la session recherche ni moi n'avons accès à docker.

## Écarts à la règle, dits

- 10 octobre, vers 14 h 55 : j'ai basculé `rag3db-lourd` sur luciepc pendant
  environ 5 minutes, avant que la règle « un worktree par chantier » ne soit
  posée. Je l'ai remis sur sa tête d'avant.
- Le même après-midi : un push en force de ma branche de travail `ir-hop`
  (amendée pour une importation en trop), branche qui n'a jamais touché
  master et qui est supprimée. C'est contraire à la règle ; depuis, chaque
  correction part sous un nom neuf.
