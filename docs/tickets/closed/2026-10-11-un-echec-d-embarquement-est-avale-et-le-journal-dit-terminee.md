# Un échec d'embarquement est avalé, et le journal dit « indexation terminée » sur une dette laissée

- **État** : corrigé (11 octobre 2026, arbre principal) — ouvert et fermé dans le même lot, à la demande de l'orchestration
- **Gravité** : réponse trompeuse (l'indexation se dit finie alors que des vecteurs manquent)
- **Atteignable en service** : oui, dès qu'un service d'embarquement refuse ou tombe pendant une indexation
- **Touche rag3weaver** : oui (`src/catalog.rs`, `src/dataflow/index_nodes.rs`)

## Ce que c'est

`Catalog::embarquer_le_retard` émettait l'échec d'une table (`CatalogEvent::Error`, contexte
« rattrapage ») et rendait `Ok`, sans la compter. Quand toutes les tables échouaient, la passe
rendait zéro, la boucle de `run_index` s'arrêtait comme sur une dette soldée, et le journal
finissait sur « indexation terminée ». La dette restait entière, sans un mot.

## Correctif

`embarquer_le_retard_suivi` relève l'échec dans le suivi de l'indexation (`EmbedWatch::errors`) ;
`run_index` le dit au journal (« N passe(s) d'embarquement en échec — <cause> ») et la dernière
ligne devient « indexation terminée — N morceau(x) non embarqué(s), à reprendre (relancer
index) ». `index_states` le dit (`embedding.failed_passes`).

Témoin : `e2e_generic_search::un_echec_d_embarquement_est_releve_et_non_avale` — un embarqueur
qui refuse tout (503) : la passe rend zéro, l'échec est relevé avec sa cause, la dette reste
entière.
