# Le dialecte postgres n'a pas la forme à générations

- **État** : fermé (19 novembre 2026, everything declarative) ; ouvert le 14 novembre 2026.
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

## Fermé le 19 novembre 2026 (ED) : postgres sait les générations

Le dessin est la page `docs/everything-declarative/19-11-2026/03-dessin-du-dialecte-postgres-a-generations.md`,
relue par c7. Deux lots.

**Lot A** (`writer-through-ir`) : les onze requêtes Cypher brutes de l'écrivain à générations passent par le
dialecte.
- Les lectures sont des `Select` avec `Visible`.
- Les écritures passent par `Write::Mark`, `Write::CopyEdges` (neuve), `Write::Detach` avec l'autre bout et
  `set_generation_counts`. `Predicate::Not` est neuf aussi.
- La preuve est une parité par résultat avec chaque ancienne requête : mêmes lignes et mêmes pas d'accès pour une
  lecture, même état pour une écriture.

**Lot B** (`postgres-generations`) : `PostgresDialect` reçoit :
- les conditions de visibilité ;
- la table `_generation` : création, publication sans toucher une génération déjà posée, dernier rang, lecture,
  compteurs ;
- la marque à valeurs ;
- la recopie d'arêtes ;
- la ligne entière ;
- l'index de visibilité.

Les témoins, rouges d'abord :
- `e2e_postgres::a_new_code_base_on_postgres_has_generations` : l'estampille `…/generations`, puis une édition qui
  retire au rang en attente et garde le rang publié. Rouge avant le lot B, avec l'estampille `Versions` ;
- `e2e_postgres::the_visibility_is_read_by_its_index_on_postgres` : sur 40 400 lignes surtout d'histoire, la
  lecture visible passe par `scope_visibilite_idx`. Au rang haut, c'est un `BitmapOr` de `_jusqu_a IS NULL` et de
  `_jusqu_a > r`. Sans l'index, c'est un `Seq Scan`.
