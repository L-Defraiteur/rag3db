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

## Fait ensuite, le 10 octobre au soir

| Quoi | Commit | Vérifié contre |
|---|---|---|
| `Hop` sans table au départ ni à l'arrivée, colonnes étiquette et nœud entier ; `fetch_related` (FetchRelatedNode, GroupFrameNode) et la transition réactive par lui | `2bef57c6d` | luciepc, lib de 14 h 20 (`41b869ba4`, **d'avant** `ff9bad960`) : e2e_reacteur, e2e_scope, e2e_catalogue_gabarits, e2e_code, e2e_usages, e2e_impact verts |
| Les définitions et les usages par le rendez-vous par `Hop`, un pivot à la fois | `f1b5e7407` | luciepc, même lib d'avant : lib 1 259, e2e_usages, usages_rendu, e2e_code, e2e_impact verts |
| `Hop` borné ; les déclarations, les voisins de `code_tools`, les liens de `code.rs` par lui | `e1f490e3e` | **ici**, lib de 17 h 27 (le seul écart est `d10b92306`, statistiques) : lib 1 259, e2e_code, e2e_usages, e2e_impact verts |

**Écart à savoir** : les deux premiers lots ont été vérifiés contre une lib
du moteur d'avant la bascule du COPY journalisé. Ils ne touchent que le
dialecte et les requêtes de lecture, pas le chargement, et l'orchestration
n'a pas demandé de les rejouer. Depuis, la date de la lib est prouvée contre
la tête de master avant chaque suite.

`structured_payloads` n'a jamais été jouée : elle exige un démon jetable, de
vrais embarquements et le corpus MTGA.

## Ce qui reste, dans l'ordre

1. `Hop` couvre tous les sites en forme de saut de l'inventaire. Reste à
   retirer `fetch_related` sans dialecte quand A aura passé `catalog.rs` sur
   `fetch_related_in`.
2. `Select`, avec le compte filtré de `composable_results`. Ensuite `Write`,
   qui doit naître de l'ingestion (le vrai client), pas d'une table système
   (note de la session mémoire).
3. Le contournement du pool PostgreSQL (une connexion tenue le temps d'une
   transaction), après la fusion de `execution-asynchrone` par la session
   recherche.
4. La preuve vivante sur PostgreSQL (`e2e_postgres`, puis la batterie du
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
