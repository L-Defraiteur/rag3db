# Le témoin des transactions PostgreSQL crée sa table sans `_uuid`

- **État** : corrigé (11 octobre 2026, session embarquements), ouvert et fermé dans le même commit
- **Gravité** : test faux (un témoin rouge pour une raison étrangère à ce qu'il prouve)
- **Atteignable en service** : non (tests)
- **Touche rag3weaver** : oui (`tests/e2e_contrat_transactions.rs`)

## Ce que c'est

`e2e_contrat_transactions::un_paquet_tient_entier_par_le_dialecte_et_la_connexion`
créait sa table avec la seule colonne `titre`, alors que
`PostgresDialect::create_table` pose `PRIMARY KEY (_uuid)` : PostgreSQL refusait
la table (42703, « column _uuid named in key does not exist »). Vu par la session
recherche à la première exécution sur le conteneur de Lucie : le témoin n'avait
jamais tourné, faute de base PostgreSQL sur les postes.

## La correction

Le témoin passe `_uuid` (texte) puis `titre` à `create_table`, comme toute table
d'entité. Il est vert sur le conteneur, et le témoin du lien défait
(`defaire_un_lien_le_retire`) crée ses tables par le même chemin. C'est le seul
fichier de ce lot qui crée ses tables à la main : `e2e_postgres` passe par le
catalogue.
