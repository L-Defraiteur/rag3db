# Sur PostgreSQL, le rebâti du plein texte en fond envoie du Cypher

- **État** : ouvert
- **Gravité** : blocage (le rebâti meurt ; l'index reste « en cours de rebâti »)
- **Atteignable en service** : seulement avec un montage PostgreSQL et lucivy
  comme moteur de texte (`MoteurTexte::Lucivy`, ou `Auto` sans plein texte natif)
- **Touche rag3weaver** : oui (`dialect.rs`, `catalog.rs`)

## Ce que c'est

Le rebâti en fond (`Catalog::rebuild_fts_step`, 5 octobre) lit les lignes par
pages avec `SchemaDialect::select_page_after_offset`. Son corps par défaut est
écrit en Cypher (`MATCH (n:T) WHERE OFFSET(id(n)) > $apres …`), et
`PostgresDialect` ne le redéfinit pas. Sur PostgreSQL, la première page échoue.

C'est un cas de la famille « corps par défaut en Cypher » : une quarantaine de
méthodes du trait en ont un, et un dialecte qui en oublie une envoie du Cypher à
sa base sans que rien ne le dise (page du contrat, §2).

## La recette

Pas encore jouée : un montage PostgreSQL avec `MoteurTexte::Lucivy`, puis un
dossier de plein texte retiré avant la réouverture, ce qui déclenche le rebâti.

## Le témoin

À écrire. Dans la batterie commune, le test 8 (« le plein texte se rebâtit
depuis les lignes ») et le test 14 (une connexion espionne refuse tout Cypher
envoyé à un dialecte qui ne déclare pas `cypher`).

## La cause

Le défaut en Cypher. S'y ajoute une question de fond : la clé du document
lucivy est le décalage de nœud rag3db (`_node_id`), qui n'existe pas sur
PostgreSQL. Lucivy sur PostgreSQL demande une clé stable, à choisir.

## Ce qu'il faut pour le fermer

- `select_page_after_offset` redéfini dans `PostgresDialect`, sur la clé
  choisie ;
- en attendant, ou en plus, les corps par défaut en Cypher passent par une
  seule porte qui rend un **refus nommé** quand le dialecte ne déclare pas
  `cypher` (voie recommandée par l'orchestration, en attente de Lucie).
