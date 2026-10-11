# Load — le chargement en masse par le dialecte, avant le code

11 octobre 2026, nuit, chantier F. C'est la dernière variante de `Write`
(page 05). Lu sur `8065f8acd`.

## Ce qui se fait aujourd'hui

À la première ingestion, ou pour un lot de naissances, `record_nodes.rs`
écrit les lignes dans un fichier CSV (`cellule_csv`, sans en-tête, NULL par
un mot convenu, sauts de ligne échappés). Ensuite :

- **les nœuds** : `dialect.copy_nodes_from_csv(table, colonnes, chemin)`
  (`record_nodes.rs:803`), puis la relecture des identifiants internes par
  `select_node_ids` (`$uuids`, par tranches de 5 000) ;
- **les liens** : `dialect.copy_links_from_csv(relation, (de, vers),
  propriétés, chemin)` (`record_nodes.rs:903`), quand le groupe dépasse
  `COPY_SEUIL` (200), après avoir écarté les paires qui existent déjà
  (`existing_links`).

Les deux méthodes rendent une `Option` : `None` veut dire « pas de chargement
en masse ici », et l'appelant repasse par `Upsert` / `Link`. Un COPY **refusé
par le moteur** empoisonne la base et la ferme sans point de reprise
(`record_nodes.rs:774`) : ce n'est pas un repli, c'est une panne nommée.

Seul rag3db redéfinit les deux (COPY de Kuzu, options de lecture épinglées par
les tests `e2e_copy_liens` et le banc de masse). PostgreSQL ne déclare pas
`bulk_load` et rend `None`.

## La forme

```text
Write::Load { target: Nodes { table, columns } | Links { relation, ends, props }, path }
```

- rag3db la traduit par `copy_nodes_from_csv` / `copy_links_from_csv` : même
  texte, au caractère près (test, comme pour les autres variantes) ;
- un dialecte qui ne déclare pas `bulk_load` la **refuse en la nommant**
  (`TranslateError::Untranslated { form: "Write::Load" }`). L'appelant
  traite ce refus comme le `None` d'aujourd'hui : il repasse par `Upsert` /
  `Link`. Rien ne change en service, mais le « pas ici » a maintenant un nom ;
- le fichier reste écrit par `record_nodes.rs` : le format CSV est celui que
  le COPY de rag3db lit. Un autre dialecte qui voudrait charger en masse
  dirait aussi le format qu'il attend. Ce n'est pas à faire maintenant.

## PostgreSQL : refus nommé, et ce qu'il faudrait pour `COPY FROM STDIN`

PostgreSQL a un chargement en masse, `COPY … FROM STDIN`, mais il passe par un
**flux** sur la connexion (`copy_in` de tokio-postgres), pas par une requête
texte. Pour le brancher, il faudrait :

1. une méthode de connexion `DbConnection::copy_in(sql, octets)`, avec un
   défaut qui refuse, et une redéfinition chez `PostgresConnection` (fichier
   de la session recherche) ;
2. un `PostgresDialect` qui déclare `bulk_load` et dit `COPY t (cols) FROM
   STDIN (FORMAT csv, NULL '…')` avec les mêmes conventions de cellules ;
3. un témoin vivant, qu'on ne peut pas jouer aujourd'hui (pas de base
   PostgreSQL sur les postes).

**Proposition** : refuser en le nommant maintenant (c'est déjà le
comportement, avec un nom en plus), et garder `COPY FROM STDIN` pour le jour
où PostgreSQL sera monté pour de vrai. Il ne gagne rien tant que personne ne
synchronise de gros dépôts sur PostgreSQL. À Lucie de le dire si elle le
veut avant.
