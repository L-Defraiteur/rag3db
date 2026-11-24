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

- Un travail long de l'hôte (copie, indexation, ouverture d'une base, fermeture de 135 bases à une pose) se fait hors du
  fil qui sert les requêtes, et hors des verrous que ces requêtes prennent.
- Pendant ce travail, l'hôte répond : l'état (« j'ouvre la tâche : copie à 40 % »), les autres dossiers, et surtout
  l'arrêt, qui n'attend jamais.
- Un refus dit ce que l'hôte fait et pour combien de temps, jamais seulement « pas dispo ».

## Le témoin à écrire, rouge d'abord

Par un vrai hôte : une ouverture de tâche rendue lente (une copie ou une indexation retenue par un point d'arrêt de
test), et pendant qu'elle dure, `GET /ide/etat` répond en moins d'une seconde, et l'arrêt du tour est accepté.

## Où regarder

- `src/daemon_host/taches.rs` : l'ouverture d'une tâche, `copier_l_espace` et `copier_la_base_sous_la_porte`,
  appelées dans la requête qui ouvre la tâche.
- La route qui reçoit l'arrêt du tour depuis le terminal (`tty/src/vue.rs`, Ctrl-C pendant un tour), et ce qu'elle attend.
