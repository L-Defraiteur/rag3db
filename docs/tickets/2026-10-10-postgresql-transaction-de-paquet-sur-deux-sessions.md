# Sur PostgreSQL, la transaction d'un paquet part sur deux sessions du pool

- **État** : ouvert — **bloque le montage PostgreSQL** avec la transaction par paquet
- **Gravité** : perte (un paquet « validé » qui ne l'est pas, ou défait à moitié)
- **Atteignable en service** : non aujourd'hui (`RAG3WEAVER_TX_PAR_PAQUET=1`
  et aucun binaire ne monte PostgreSQL) ; **oui** dès que la transaction par
  paquet devient le défaut et qu'un montage PostgreSQL synchronise
- **Touche rag3weaver** : oui (`code_sync.rs`, `postgres_connection.rs`)

## Ce que c'est

`code_sync.rs` ouvre et ferme la transaction d'un paquet en envoyant
`BEGIN TRANSACTION`, puis `COMMIT` ou `ROLLBACK`, en texte brut sur
`catalog.conn()`. Sur PostgreSQL, chaque `execute` prend une connexion du pool
(`postgres_connection.rs`, `pool.get()` dans `execute_async` et
`execute_with_params_async`). Le `BEGIN`, les écritures et le `COMMIT` peuvent
donc partir sur des sessions différentes. Les écritures sont alors validées une
à une hors de toute transaction, un `ROLLBACK` ne défait rien, et une session
du pool peut rester ouverte dans une transaction jamais close.

## La recette

Pas encore jouée. Il faut un montage PostgreSQL (`tests/e2e_postgres.rs`,
`RAG3WEAVER_PG`) avec `RAG3WEAVER_TX_PAR_PAQUET=1`, puis un paquet qui échoue
au milieu : ses premières lignes doivent rester absentes après le `ROLLBACK`.

## Le témoin

À écrire : le test 5 de la batterie commune (« un paquet défait ne laisse
rien »), `docs/8-octobre-2026-16h29/embarquements/01-le-contrat-du-dialecte.md`
§5.

## La cause

La connexion PostgreSQL n'a pas de session tenue, et `DbConnection` n'a pas de
primitive de transaction : la transaction passe par du texte, et le texte ne
dit pas sur quelle session il part. Le runtime dédié de la session recherche
(chantier C) garde le pool tel quel, le défaut reste donc entier après lui.

## Ce qu'il faut pour le fermer

1. **Tout de suite, côté synchronisation** (chantier A) : la transaction par
   paquet est **refusée par son nom** sur un dialecte qui ne déclare pas
   `DialectCapabilities::transactions`. `PostgresDialect` ne la déclare pas.
2. **Ensuite** : `begin`, `commit` et `rollback` dans `DbConnection`, sur une
   session tenue du début à la fin du paquet ; `PostgresDialect` déclare alors
   `transactions`, et le test 5 de la batterie passe sur PostgreSQL.
