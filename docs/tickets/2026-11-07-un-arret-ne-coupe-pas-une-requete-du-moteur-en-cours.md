# Un arrêt ne coupe pas une requête du moteur en cours

- **État** : ouvert (7 novembre 2026), relevé par hooks-essai en fermant « une recherche continue
  sur l'hôte après l'abandon de son client ».
- **Famille** : produit
- **Gravité** : coût. Un run arrêté (par l'IDE, `stop_run` ou un `deadline_ms`) va au bout de la
  requête que son nœud a déjà lancée au moteur, par exemple un plein texte flou de 30 s.
- **Atteignable en service** : oui, sur une requête lente arrêtée en vol.
- **Touche rag3weaver** : oui (`Rag3dbConnection`, le trait `DbConnection`, l'exécution d'un graphe).

## Ce que c'est

L'arrêt d'un run se lit **entre deux nœuds** (`CancelHandle`, `cancel_remaining`), et les nœuds
qui attendent le lisent par `ctx.cancellation()`. Un nœud bloqué dans un appel au moteur ne le lit
qu'au retour. Le moteur sait pourtant interrompre une requête : `Connection::interrupt` et
`set_query_timeout` existent dans le binding (`tools/rust_api/src/connection.rs:197`, `:205`).
Rien dans rag3weaver ne les appelle : `Rag3dbConnection` ne les expose pas, et le trait
`DbConnection` n'a pas de méthode d'interruption.

## Pourquoi ce n'est pas un appel de plus

La connexion est **partagée** par le backend. L'interrompre à l'arrêt d'un run couperait aussi les
requêtes des autres appelants (une autre session, une réaction). Il faut que la requête d'un run
passe par une connexion à lui, du pool de lecture par exemple, et que l'arrêt de ce run
n'interrompe que celle-là.

## Ce qu'il faut pour le fermer

`DbConnection::interrupt` (sans effet par défaut), une connexion par run pour les nœuds qui
interrogent le moteur, et l'arrêt d'un run qui interrompt la sienne. Témoin : une requête lente au
moteur arrêtée en vol rend la main en moins d'une seconde, et une requête d'un autre run sur le
même backend va au bout.
