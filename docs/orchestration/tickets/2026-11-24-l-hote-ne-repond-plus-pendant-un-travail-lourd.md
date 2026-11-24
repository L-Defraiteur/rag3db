# L'hôte ne répond plus pendant un travail lourd

Ouvert le 24 novembre 2026, 11 h 40.

## Le constat

Le 24 novembre à 11 h 11, le terminal lancé depuis `~/git_workspaces` (un dossier sans git qui contient tous les dépôts)
a ouvert une tâche. L'hôte a copié 106 Go dans le dossier de la tâche (`taches/51226d87d415/24-11-11h11/espace`), en
5 minutes environ. Pendant ce temps :

- le terminal a reçu des refus « hôte pas dispo » à chaque demande ;
- l'arrêt par Ctrl-C a lui-même été refusé.

La copie est retirée par le lot `c7/24-11-2026-sans-copie` (un dossier sans git est travaillé en place). Le défaut de
fond reste : **un travail lourd de l'hôte le rend sourd**.

## Ce qui est attendu

Lucie, 24 novembre 2026 : « pas normal que l'hôte sache pas répondre quand il fait quelque chose de lourd ».

L'indexation tourne déjà en fond : l'outil `index` rend la main tout de suite, et la recherche répond avec ce qui est
prêt. Deux travaux bloquent encore la requête qui les lance :

- **l'ouverture d'une tâche** : la copie du dossier (retirée par le lot `c7/24-11-2026-sans-copie`) et la copie de la
  base (`copier_la_base_sous_la_porte`), faites dans la requête même ;
- **la réindexation d'un fichier après une écriture de l'agent** : `code_sync::edit_file_shared_with_hash` prend le
  verrou du catalogue et attend la fin de `reingest_file`, dont le plein texte rendu durable (1 à 5 s par petit
  fichier, mesuré le 24 novembre).

**Une seule abstraction, facile à réutiliser** (Lucie, 24 novembre 2026 : « ça doit être une abstraction facile à
réutiliser pour tout autre truc async ») : un travail de fond de l'hôte se déclare une fois et se lance par elle, quel
qu'il soit (copie, indexation, ouverture, fermeture, réindexation, un outil asynchrone d'agent). Elle porte d'elle-même :

- le travail hors du fil qui sert les requêtes, et hors des verrous qu'elles prennent ;
- un état lisible à tout moment (ce qu'il fait, son avancement, depuis quand) ;
- l'arrêt, accepté tout de suite ;
- la fin publiée sur le flux, pour qu'un agent ou le terminal attende un événement au lieu d'interroger ;
- un refus qui dit ce que l'hôte fait et pour combien de temps, jamais seulement « pas dispo ».

**Elle existe déjà dans les graphes, il faut s'en servir** (relevé le 24 novembre 2026) :

- `dataflow::background` (`DataflowRuntime::spawn`, le registre `Registry`) : un run sur un fil à lui, sous l'exécuteur
  concurrent (où l'annulation atteint chaque nœud), sa référence rendue tout de suite, sa fin écrite une fois dans la
  boîte de l'appelant et dite sur le bus (`RunFinished`), lié ou non à son parent (`Tie`), borné (`Limits`) ;
- `SpawnNode` : ce lancement en fond posé comme un nœud, dans n'importe quel graphe ;
- `%% async: true` : un graphe-outil entier qui rend une poignée tout de suite et son résultat plus tard (seul
  `run_bg.mmd` s'en sert).

Ce qui manque pour que ce soit pratique :

- **un nœud qui attend la fin d'un run lancé**, par événement (le nœud `listen` sur un canal de la page 03 d'everything
  declarative), et une jointure de plusieurs ;
- **marquer un seul nœud asynchrone** dans un graphe (`%% async` par nœud), comme sucre sur `SpawnNode` + l'attente ;
- **les travaux lourds de l'hôte en graphes**, lancés par ce mécanisme, plutôt qu'en Rust dans la requête.

Partir de ce qui existe déjà pour l'indexation (le journal suivi par `wait`, `lancer_l_index`) plutôt que d'écrire un
second mécanisme. Elle sert aussi au chantier « outils asynchrones et réveils » de l'agent.

## Le témoin à écrire, rouge d'abord

Par un vrai hôte : une ouverture de tâche rendue lente (une copie ou une indexation retenue par un point d'arrêt de
test), et pendant qu'elle dure, `GET /ide/etat` répond en moins d'une seconde, et l'arrêt du tour est accepté.

## Où regarder

- `src/daemon_host/taches.rs` : l'ouverture d'une tâche, `copier_l_espace` et `copier_la_base_sous_la_porte`,
  appelées dans la requête qui ouvre la tâche.
- La route qui reçoit l'arrêt du tour depuis le terminal (`tty/src/vue.rs`, Ctrl-C pendant un tour), et ce qu'elle attend.

## La forme du nœud d'attente (Lucie et c7, 24 novembre 2026, 11 h 50)

- **Une entrée `channels`** : la sortie d'un nœud asynchrone branchée directement (sa poignée, ou une liste), ou des
  noms de canaux en texte. Une poignée est un nom de canal (`run/<poignée>`) : le nœud ne distingue pas les deux.
- **Des ports de sortie déduits du câblage, à la validation** (Lucie : « le listener obtient auto des poignées d'output
  compatibles avec le handle ») : une poignée porte le type du résultat de son nœud ; pour chaque poignée branchée, un
  port de ce type, nommé d'après le nœud source, plus `errors`, typé. Un branchement de type faux est refusé au
  chargement du graphe, jamais à l'exécution. Un canal donné en texte rend un port non typé, sauf s'il est déclaré
  avec son type dans le graphe.
- **`mode`** : `all` (la jointure, un port par poignée), `any` (la première) ou `each` (une à une) ; `any` et `each`
  rendent un port de l'union des types. **`timeout`**, borné, qui expire en erreur typée.
- **`%% async` sur un nœud** : du sucre qui relie sa poignée à un nœud d'attente là où son résultat est lu.
- Le morceau neuf du moteur : des ports déduits du câblage à la validation (aujourd'hui, chaque nœud fixe les siens).
  Il servira aussi à la jointure et aux sous-graphes qui exposent ce qu'ils rendent.

