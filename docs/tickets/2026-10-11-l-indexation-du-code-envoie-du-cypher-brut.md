# L'indexation du code envoie du Cypher brut : le gabarit code ne s'indexe pas sur PostgreSQL

- **État** : ouvert (11 octobre 2026) — à la session embarquements, après le branchement d'impact sur `run_plan` ; l'arbre principal relit `code.rs`
- **Gravité** : blocage (sur PostgreSQL, `ingest_code` échoue)
- **Atteignable en service** : avec un montage PostgreSQL du backend de code ; rag3db n'est pas concerné
- **Touche rag3weaver** : oui (`src/code.rs`)

## Ce que c'est

`ingest_code` envoie une requête écrite en Cypher, hors du dialecte :
`src/code.rs:2396`, `UNWIND $uuids AS u MATCH (s:Scope {_uuid: u}) RETURN u, s.source, s.repo`.
Sur le conteneur PostgreSQL, la mesure d'impact (`e2e_postgres::mesure_impact_sur_postgres`)
s'arrête là : 42601, « syntax error at or near UNWIND ».

C'est la suite de l'inventaire du Cypher au-dessus des dialectes
(`extension/rag3weaver/docs/8-octobre-2026-16h29/embarquements/02-le-cypher-au-dessus-des-dialectes.md`) :
les nœuds sont passés aux formes de l'IR, l'indexation du code non.

## Ce qu'il faut pour le fermer

Convertir les requêtes de l'indexation du code en formes de l'IR (cette lecture est
un `Select::by_uuids`), relever les autres requêtes du même chemin, puis indexer
`src/daemon` sur le conteneur : la mesure d'impact sur PostgreSQL devient le témoin.

## Le témoin à venir

La mesure d'impact de bout en bout sur PostgreSQL, retirée de la branche
`impact-run-plan` parce qu'elle rougit ici : même corpus (`src/daemon`), mêmes noms
et même configuration que `e2e_mesure_impact::mesure_impact_sur_rag3db`, sur le
catalogue du conteneur (`e2e_postgres::catalogue`), avec la connexion qui compte les
instructions (`e2e_mesure_impact::Compteur`), `register_code_schema`, `ingest_code`,
`drain`, puis `e2e_mesure_impact::mesurer("postgresql", …)`. Elle se remet en place
avec la conversion et doit passer au vert.
