# Une relation ne peut pas perdre sa clé

- **État** : ouvert (21 novembre 2026)
- **Famille** : produit
- **Gravité** : basse. Le chemin est refusé par son nom, rien ne se perd en silence. Mais une relation qui porte une
  clé ne peut plus la retirer, ni la changer.
- **Atteignable en service** : non aujourd'hui, car aucune relation déclarée ne porte de clé avant le lot des
  occurrences. Il le deviendra le jour où un schéma retire ou change la clé d'une relation à clé.
- **Touche rag3weaver** : oui (`Catalog::register_keyed_relation`, `SchemaDialect::add_rel_key`)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, au pas 3 des liens à clé, sur la demande de c7

## Constat

Depuis les liens à clé (`RelationDef::key`, branche `codeparsers/keyed-links-2`), une relation peut porter plusieurs
liens entre deux mêmes nœuds, distingués par sa clé. La clé peut être donnée à la création, ou ajoutée à une relation
existante : sur PostgreSQL, la clé primaire passe alors de la paire à (paire, clé), dans un bloc `DO`.

Le chemin inverse est **refusé par son nom** à l'enregistrement : « la relation R portait la clé k ; la retirer n'est
pas pris en charge », ou « la changer en j ». Le refus vaut même quand aucune paire ne porte plusieurs liens. Son
témoin est `a_relation_that_had_a_key_cannot_lose_it`.

## Ce que demande la vraie migration inverse

1. **Fusionner les liens multiples d'une même paire en un seul**, avec une règle dite. Laquelle gagne : le dernier
   écrit, ou un choix par propriété ? Une perte de données ne se fait pas en silence : elle se mesure et s'annonce.
2. **Sur PostgreSQL**, ramener la clé primaire sur la paire, après la fusion.
3. **Sur rag3db**, il n'y a pas de contrainte, mais les liens multiples restent à fusionner, sinon le `MERGE` sans clé
   en trouve plusieurs.
4. **Changer une clé**, c'est retirer l'ancienne puis donner la nouvelle, avec les mêmes questions.

## Témoin attendu

Une relation à clé avec deux liens sur une paire. La clé est retirée par une migration déclarée : un seul lien reste,
selon la règle dite, et la contrainte PostgreSQL revient sur la paire.
