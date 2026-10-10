# Embarquements — rapport de session (chantier F)

Mis à jour le 10 octobre 2026, vers 12 h 30, avant le redémarrage du poste.
L'état d'avant est dans
`../../3-octobre-2026-23h31/embarquements/01-rapport-de-session.md`.

## Fait

| Quoi | Où | Commit |
|---|---|---|
| Le contrat du dialecte : la page avant le code | [01-le-contrat-du-dialecte.md](01-le-contrat-du-dialecte.md) | `2f2f8e77c` |
| Les capacités déclarées d'un seul tenant (`DialectCapabilities` : cypher, transactions, bulk_load, structured_fields). `speaks_cypher` et `supports_copy_from` les lisent ; les filtres imbriqués aussi, au lieu de `name()`. 98 tests unitaires du dialecte et des filtres verts ; toutes les cibles de test compilent | `src/dialect.rs`, `src/filter.rs` | `2e67ce3e0` |
| Deux tickets PostgreSQL : la transaction de paquet sur deux sessions du pool (**bloque le montage PostgreSQL**) ; le rebâti du plein texte en Cypher | `docs/tickets/2026-10-10-*` | `2e67ce3e0` |
| Le compteur `copy_journal_fallbacks` lu à la fin de la mesure | `tests/e2e_estimate.rs` | `2e67ce3e0` |
| L'inventaire du Cypher au-dessus des dialectes (42 sites à reprendre), et un langage intermédiaire en cinq formes | [02-le-cypher-au-dessus-des-dialectes.md](02-le-cypher-au-dessus-des-dialectes.md) | `981780f14` |
| La série d'avant bascule, lib de 10:46, fuite non corrigée : 19 passes sur 20 | [03-la-serie-d-avant-bascule.md](03-la-serie-d-avant-bascule.md) | ce commit |

## Décidé par Lucie (relayé par l'orchestration)

- PostgreSQL est un dialecte qui doit marcher quand on le monte, sans être
  le produit : pour ses manques, des **contournements**, et le refus nommé
  comme filet.
- Les corps par défaut en Cypher passent par **une seule porte**, qui rend un
  refus nommé si le dialecte ne déclare pas `cypher`.
- « Éviter complètement le Cypher au-dessus des dialectes, avec notre
  langage intermédiaire s'il le faut » : la page 02 attend sa décision sur
  les cinq formes.
- La bascule des défauts est acceptée ; l'arbre principal fusionne.

## Accordé avec les autres chantiers

- **A (arbre principal)** : sur un dialecte sans `transactions`, la
  transaction par paquet demandée explicitement est refusée par son nom ; au
  défaut, elle est coupée avec un avertissement nommé. A ajoute un accesseur
  du dialecte dans `catalog.rs`, et le test sur le nom de `catalog.rs:1557`
  passe par `structured_fields`.
- **C (recherche)** : le runtime dédié garde le pool. Il corrige au passage
  `--features postgres`, qui ne compile plus sur master (`CypherValue::Typed`
  n'est pas couvert dans `postgres_connection.rs`). Je ne touche pas ce
  fichier avant son push.

## Ce qui reste, dans l'ordre

1. Après le push de C : la suite `e2e_postgres` (docker `rag3weaver-pg`,
   port 5433) pour constater l'état réel.
2. La batterie commune de 14 tests (page 01, §5), sur rag3db puis
   PostgreSQL, un ticket par rouge.
3. La porte unique des corps Cypher, et `execute_raw` refusé sans `cypher`.
4. Le contournement du pool : une connexion épinglée le temps de la
   transaction, puis `PostgresDialect` déclare `transactions`.
5. La série d'avant bascule, rejouée sur la lib qui corrige la fuite (au
   signal du cœur C++) : les douze passes dans une seule tenue.
6. L'IR (`Hop` et `Count` d'abord), si Lucie dit oui à la page 02.
