# Sur PostgreSQL, défaire un lien ne trouve rien

- **État** : ouvert
- **Gravité** : réponse fausse (une arête qui devait partir reste)
- **Atteignable en service** : avec un montage PostgreSQL, quand une
  ingestion est défaite (reprise après incident, annulation d'un nœud de
  liens)
- **Touche rag3weaver** : oui (`dataflow/record_nodes.rs`, `dialect.rs`)

## Ce que c'est

L'annulation d'un nœud de liens (`LinkRecordNode`, `record_nodes.rs`, autour de
la ligne 1285) envoie au dialecte des lignes `{from, to}`.
`Rag3dbDialect::batch_delete_relation` lit bien `from` et `to`. Mais
`PostgresDialect::batch_delete_relation` (`dialect.rs`, autour de la
ligne 2333) déplie `jsonb_to_recordset(...) AS v(from_uuid TEXT, to_uuid TEXT)` :
les deux colonnes valent `NULL`, et le `DELETE` ne touche aucune ligne, sans
erreur.

## La recette

Pas jouée, faute de base PostgreSQL vivante sur les postes (pas de docker) :
un lien posé puis défait par l'annulation du nœud doit disparaître de la
table de relation.

## Le témoin

À écrire : un test de la batterie commune du contrat (page
`extension/rag3weaver/docs/8-octobre-2026-16h29/embarquements/01-le-contrat-du-dialecte.md`,
§5, test 4 « une suppression emporte ses relations »), qui vise aussi
l'annulation.

## La cause

Le contrat ne dit pas les clés de `$items` pour cette méthode : chaque dialecte
a choisi les siennes. C'est exactement ce que la forme `Write` du langage
intermédiaire doit fixer (une forme, des clés nommées une fois).

## Ce qu'il faut pour le fermer

`PostgresDialect::batch_delete_relation` lit `from` et `to` (le plus court),
ou la suppression d'arêtes passe par `Write` ; puis le témoin, vert sur
PostgreSQL vivant.
