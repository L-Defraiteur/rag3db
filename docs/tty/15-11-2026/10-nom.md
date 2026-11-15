# tty-18 : `/nom`, et le nom de l'agent sur ses lignes

Session tty, 15 novembre 2026, lot demandé par c7. La surcharge du nom par tâche est d'ED, déjà sur master : `Tache::nom_de_l_agent`, et `agent_name` à la création, par `POST /ide/taches`.

## Ce qui change

- **`/nom <x>`** nomme l'agent de la tâche par `POST /ide/teammate/name {backend, task, agent_name}`. La forme a été convenue avec ED, qui code la route.
  - Réponse : `{ok, result: {agent_name}}`. Un nom vide rend celui du backend.
  - La route met à jour le registre, puis recharge le `prepared` du backend ouvert avec la nouvelle surcharge (le geste d'`apply_reload`, sans rouvrir la base) : les surcharges n'entrent qu'à l'ouverture (`surcharges_de`). Le témoin d'ED vérifiera que l'invite suivante porte le nouveau nom.
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

## Sur master

La route d'ED est sur master (a178fb923) : un refus y arrive en 200 avec `ok: false`, que le terminal dit tel quel. Le `name` des lignes de l'agent est chez archi ; d'ici là, les lignes gardent « équipier ».

## L'essai réel, et tty-19

Joué le 16 nov. contre l'hôte 7970, sur 4802a147a, sur une tâche d'essai à moi (`essai-nom-tty`, rejetée ensuite), avec le binaire de master sous tmux :
- `/nom Ombre` → « [Ombre · compte rendu] Je m'appelle Ombre, votre coéquipier autonome… » ;
- `/nom -` → « Algareth », et la ligne suivante porte `name: Algareth`.

L'essai a montré trois défauts.

1. **Juste après un renommage, `/nom` seul ne savait pas le nom.** La vue ne retenait que le `name` vu sur les lignes. Elle retient maintenant aussi le nom que rend la route (`nom_garde`).
2. **Une demande s'écrivait deux fois** : « › … » à l'envoi, puis « [toi · demande] … » quand la boîte la rendait. Ce qui est envoyé d'ici est retenu, et sa copie de la boîte ne se réécrit pas. Une ligne de toi venue d'ailleurs (l'IDE) s'écrit toujours.
3. **Chez archi** : après la fin d'un pas, une consigne (« Redis ton nom… ») n'a reçu qu'un `step_done`, sans compte rendu ni relance de l'hôte. L'écran dit, à bon droit, « il a fini sans répondre ».

Témoins de 1 et 2 : `the_name_kept_by_the_host_is_the_one_the_view_keeps` et `what_you_send_is_written_once_even_when_the_mailbox_gives_it_back`, vus rouges. Le crate passe 111 tests.

Une espace dans un nom (« Om bre ») est acceptée par l'hôte ; si ce n'est pas voulu, c'est chez ED.
