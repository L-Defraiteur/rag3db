# Un arrêt ne coupe pas une requête du moteur en cours

- **État** : fermé (19 novembre 2026, session recherche, entrée 6 de la passation). Ouvert le 7 novembre 2026, relevé par hooks-essai en fermant « une recherche continue sur l'hôte après l'abandon de son client ».
  sur l'hôte après l'abandon de son client ».
- **Famille** : produit
- **Gravité** : coût. Un run arrêté (par l'IDE, `stop_run` ou un `deadline_ms`) va au bout de la
  requête que son nœud a déjà lancée au moteur, par exemple un plein texte flou de 30 s.
- **Atteignable en service** : oui, sur une requête lente arrêtée en vol.
- **Touche rag3weaver** : oui (`Rag3dbConnection`, le trait `DbConnection`, l'exécution d'un graphe).
- **Pour** : recherche (suivi et lot ; toute réattribution passe par l’orchestration).
- **Signalé par** : recherche ; sources et observations dans ce ticket et le [rapport de session](../11-11-2026/01-rapport-de-session.md).

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

## Ce qui le ferme (19 novembre 2026, branche `interrupt`)

- **`DbConnection::interrupt`**, sans effet par défaut. `Rag3dbConnection` appelle `Connection::interrupt` du binding :
  le moteur pose le drapeau de sa requête, et les opérateurs en cours le lisent.
- **`Instantane::interrompre`** :
  - sous le verrou de la phase de l'instantané : tant qu'il est ouvert, sa connexion est à cet appel seul ; une fois
    fini et rendu au pool, il n'interrompt plus rien ;
  - il marque l'instantané : toute requête suivante est refusée (« l'appel a été arrêté : sa lecture est
    interrompue »). Le moteur remet son drapeau à zéro au début de chaque requête, si bien qu'une interruption
    tombée entre deux requêtes se perdrait sans cette marque ;
  - à la fin, la transaction de l'instantané est défaite et la connexion oubliée.
- **`CancelHandle::on_cancel`** : un rappel appelé au premier arrêt, une fois, ou tout de suite si le run est déjà
  arrêté. Il est oublié quand sa garde tombe.
- **Le backend** pose ce rappel après l'instantané d'un appel. L'arrêt du run interrompt la requête de cet instantané,
  et jamais la connexion partagée. Toutes les voies d'arrêt passent par la poignée : `stop_run`, l'échéance d'un
  appel, l'arrêt du parent.

**Témoins**, rouges sur les stubs sans effet (df563ced93), verts avec le correctif (7200f155b0, lib 6e2476d41) :
- `read_pool::tests::an_interrupted_snapshot_gives_the_hand_back_and_the_other_goes_to_its_end` : deux instantanés d'un
  pool somment `x * y % 13`.
  - Le premier fait 4 milliards de lignes, soit 20 s seul ; il est interrompu à 300 ms et rend la main en moins
    d'une seconde, par une erreur. Au rouge, il allait au bout (`Ok`, 20,4 s).
  - Le second fait 100 millions de lignes ; il va au bout avec sa somme exacte.
  - Une requête suivante du premier est refusée.
- `dataflow::concurrent::tests::a_stop_runs_what_was_registered_once_and_never_after_its_guard`.

**Le témoin par le chemin de l'hôte** (branche `interrupt-e2e`), à la demande de c7 :
`backend::tests::stopping_a_call_interrupts_its_snapshot_statement_through_the_host_path`.
- **Le montage** : un nœud de lecture lente (`backend::lecture_lente`, inscrit au registre et permis aux outils sous
  `cfg(test)` seulement) somme quatre milliards de lignes sur la connexion de l'instantané. L'outil qui le porte est
  appelé par `call_in` sur la cellule du catalogue, comme le fait une session de l'hôte. Une fois le run de l'appel en
  vol, `stop_run` l'arrête.
- **Le résultat** : l'appel rend la main en moins d'une seconde, et pas avec sa somme. Sans les quatre lignes de
  `execute_plan_une_fois` (sonde `probe-interrupt-e2e-red`), il allait au bout : 20,1 s.
- **Ce témoin a trouvé un défaut que celui du pool ne voyait pas.** Les instantanés de l'hôte passent par
  `RevocableConnection` (`src/connection/lifecycle.rs`), qui relayait tous les verbes sauf `interrupt`. Elle prenait
  donc le défaut, sans effet, et un appel arrêté allait au bout (19,8 s), alors que le témoin du pool, sur une
  `Rag3dbConnection` nue, était vert. Elle relaie maintenant `interrupt` (86ce45f487).

**Une course reste** : une interruption qui tombe entre le contrôle de l'instantané (`avant`) et le début de la
requête dans le moteur est remise à zéro par le moteur, et cette requête va au bout. La fenêtre est de quelques
microsecondes. Les requêtes suivantes de l'appel sont refusées par la marque, et le run s'arrête entre deux nœuds.

