# Le dialecte postgres n'a pas la forme à générations

- **État** : ouvert (14 novembre 2026).
- **Famille** : produit
- **Gravité** : manque — une base postgres neuve reste à la forme d'avant (« forme sans couches ») : elle n'aura ni l'historique de l'ancre (2a′-2) ni les couches (2b′)
- **Atteignable en service** : oui, toute base de code ouverte sur postgres
- **Touche rag3weaver** : oui (`src/dialect.rs`, `PostgresDialect`)
- **Pour** : everything declarative
- **Signalé par** : everything declarative, au lot 2a′-1, sur la condition de c7 du 14 novembre 2026 (postgres hors du lot par `sait_les_generations`, un ticket pour sa forme)

## Ce que c'est

2a′-1 pose les générations sur une base neuve si son dialecte les sait
(`SchemaDialect::sait_les_generations`). Seul `Rag3dbDialect` répond oui. Sur postgres, une base
neuve reçoit l'estampille `code-scope-v1/content` et non `code-scope-v1/content/generations` ; elle
se lit comme avant, et `index_state` la dit « forme sans couches ».

## Ce qui manque à postgres

Le rendu du prédicat `Predicate::Visible` existe déjà (`postgres_condition_sur`, colonnes citées).
Il manque :
- `sait_les_generations` à `true` ;
- `condition_visible_sur`, le rendu sur un alias pour `resoudre_le_filtre` et le filtre vectoriel ;
- `create_generation_table`, `merge_generation` et `latest_generation_rank` (la table interne
  `_generation`) ;
- l'ajout des trois colonnes par `alter_add_column_default`, à vérifier sur `Scope` et `File` ;
- `e2e_postgres` joué sous une base neuve à générations, et le témoin 0 sur ce dialecte.

## Comment le voir

Ouvrir une base de code neuve sur postgres, puis lire `code_scope_form` dans les métadonnées : on y
trouve `code-scope-v1/content`.
