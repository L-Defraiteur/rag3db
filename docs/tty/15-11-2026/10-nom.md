# tty-18 : `/nom`, et le nom de l'agent sur ses lignes

Session tty, 15 novembre 2026, lot demandé par c7. La surcharge du nom par tâche est d'ED, déjà sur master : `Tache::nom_de_l_agent`, et `agent_name` à la création, par `POST /ide/taches`.

## Ce qui change

- **`/nom <x>`** nomme l'agent de la tâche par `POST /ide/teammate/name {backend, task, agent_name}`. La forme a été convenue avec ED, qui code la route.
  - Réponse : `{ok, result: {agent_name}}`. Un nom vide rend celui du backend.
  - Le registre se relit à chaque tour, sans réouverture.
  - `/nom -` envoie un nom vide : l'agent reprend le nom de son dépôt.
  - `/nom` seul dit le dernier nom vu sur les lignes de l'agent.
  - Un refus se dit tel quel.
  - Tant que l'hôte n'a pas la route, il répond « route inconnue » en 404. Le terminal dit alors « cet hôte ne sait pas encore renommer l'agent ». Un autre 404, une tâche inconnue par exemple, se dit tel quel.
- **Le nom sur ses lignes** : une ligne de l'équipier qui porte `name` s'en-tête par lui, `[Algareth · compte rendu]`. Sans `name`, ou avec un nom vide, elle garde « équipier ». Le `name` d'une ligne d'un autre auteur ne change rien. Le lot d'archi pose `name` à côté de `from` ; il pousse après 96.

## Les témoins

Quatre tests, vus rouges sur leurs assertions, sur des coquilles vides (366bf87c1) :
- `/nom` lu par les commandes ;
- l'en-tête par `name` (`boite`) ;
- le dernier nom vu, retenu par la vue ;
- ce que `/nom` dit : le nom gardé, celui du dépôt, un refus, la route absente, un autre 404.

Le crate passe 109 et 20 tests, et clippy `-D warnings` ne signale rien.

## Ce qui attend

- La route d'ED.
- Le `name` des lignes, chez archi.

Le lot ne va sur master qu'au mot de c7.
