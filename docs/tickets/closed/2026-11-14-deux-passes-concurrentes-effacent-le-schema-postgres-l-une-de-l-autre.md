# Deux passes concurrentes effacent le schéma postgres l'une de l'autre

- **État** : fermé le 14 novembre 2026, doublon de `docs/recherche/tickets/2026-11-14-deux-passes-concurrentes-partagent-la-base-postgres-de-test.md` (recherche, 88d2c84fc0), ouvert pour la même collision quelques minutes avant ; la trace et les sept panics y sont reportés
- **Famille** : produit
- **Gravité** : faux rouges. Une passe qui joue `e2e_postgres` pendant qu'une autre le joue aussi voit ses tests échouer sur un schéma effacé sous elle, sans rapport avec son code. Plusieurs lots du soir rejouent cette suite en même temps : ça reviendra
- **Atteignable en service** : non (outillage de passe)
- **Touche rag3weaver** : oui (`run_e2e.sh`, les tests de `tests/e2e_postgres.rs`, et le schéma `rag3weaver` de la base `rag3weaver_test` qu'ils se partagent)
- **Pour** : G (outillage de passe)
- **Signalé par** : everything declarative, 14 novembre 2026, 23 h 24, passe courte de `vue-publiee-pile` (23101838b) à luciepc, lib 373b5477b. La trace est dans `~/.cache/rag3weaver-build/ed-generations-2/pile/e2e_postgres-schema-partage.log`

## Ce que c'est

Toutes les passes visent la même base, `RAG3WEAVER_PG=postgres://rag3weaver:rag3weaver@127.0.0.1:5433/rag3weaver_test`,
et le même schéma, `rag3weaver`. Un test qui initialise le catalogue le recrée ou l'efface. Deux
passes en même temps se marchent dessus. Sur la passe de 23 h 24, 7 tests sur 36 ont échoué, tous dans
le schéma, aucun dans une réponse :
- six fois : `initialize sur postgres : 42704: referenced schema was concurrently dropped`, sur
  `CREATE EXTENSION IF NOT EXISTS pg_trgm|vector SCHEMA rag3weaver` ;
- une fois : `42P01: relation "rag3weaver._dataflow_node_state" does not exist`.

La même suite, rejouée seule sur le même commit, est le verdict de la passe (c7).

## Ce qu'on attend (c7)

**Un schéma par passe, pas un verrou.** Il serait nommé par le worktree, ou par un
`RAG3WEAVER_PG_SCHEMA` posé par défaut et dérivé de la passe, puis effacé à la fin de la passe. Deux
passes ne partagent plus rien. Un verrou ne ferait que les mettre en file, sur un poste dont c'est
déjà la ressource rare.
