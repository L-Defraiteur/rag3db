# L'équipier délègue à un sous-agent : la conception de `delegate`

15 novembre 2026, soir. Lucie, en testant son harnais : « j'ai pas l'impression qu'il sache lancer
de sous-agent ni leur parler en temps réel » (c7). C'est le point suivant de la
[vision du 15 novembre](../../orchestration/visions/15-11-2026-l-agent-est-un-agent-de-code-qui-gere-ses-worktrees.md)
(« il prend tel worktree »). Page avant le code ; le nom d'Algareth passe devant dès qu'ED a
atterri.

## Ce qui existe

- La tâche, son worktree et sa branche (`ouvrir_une_tache`) ; le registre, avec `parent_task`, qui
  vaut `null` tant que personne ne délègue.
- La boucle de l'équipier et sa boîte par tâche (demandes, réponses, accords, `stop`), avec les
  gardes de ce soir : la demande qui attend une ligne, l'attente bornée.
- tty, abonné à `mailbox`, `agent` et `tokens`.

## La conception, en quatre lignes

1. **Quatre outils de l'équipier, déclarés par la boucle** (comme `report` et `step_done`) :
   - `delegate {name, request, expects?}` ouvre une sous-tâche dont `parent_task` est la tâche
     courante. Son worktree part du commit de la branche du parent, sur une branche
     `agent/<parent>--<name>` : git refuse `agent/<parent>/<name>` à côté de `agent/<parent>`
     (vérifié). La demande y est postée, avec ses `expects`, et l'équipier de la sous-tâche est
     lancé. L'outil rend tout de suite : il ne bloque pas.
   - `reply {name, text, reply_to?}` écrit dans la boîte de la sous-tâche, au nom du parent
     (une `answer` ou un `message`).
   - `stop {name}` y écrit `stop`.
   - `wait {name, timeout_s?}` attend la fin de la sous-tâche, au plus son délai. Il rend son
     compte rendu final (le dernier `report`, la fin, le diff de sa branche contre celle du
     parent), ou « toujours en cours » avec la dernière ligne.
2. **Le dialogue en temps réel passe par la boîte du parent.**
   - Chaque `report`, `question` ou `approval` de la sous-tâche est recopiée chez le parent, au fil
     de l'eau, en `from: "subagent"` avec `name` et `task`. Ce n'est jamais `from: "teammate"` :
     sinon les gardes du parent (la demande qui attend sa ligne, la question ouverte) les
     prendraient pour les siennes.
   - Une ligne de sous-agent est une nouvelle pour le parent : elle le réveille à sa frontière, et
     sa question attend la réponse du parent (par `reply`), pas celle de l'utilisateur.
   - **À la fin**, `integrate: "merge"` (le défaut) fusionne la branche du sous-agent dans celle du
     parent, et dit le commit de fusion. Un conflit ne fusionne rien : il est rendu au parent comme
     une proposition, avec les fichiers en conflit. `integrate: "propose"` ne fusionne jamais.
3. **Les bornes, réglées** dans `teammate.delegation` :
   - `max_depth`, 2 par défaut (un sous-agent peut déléguer une fois) ;
   - `max_children`, 3 sous-agents vivants à la fois par tâche ;
   - `wait_s`, 1800, le plafond d'un `wait` ;
   - `reply_wait_s`, l'attente bornée de ce soir, s'applique aussi au sous-agent, et son échec nommé
     remonte au parent.
   - Au-delà d'une borne, `delegate` refuse en la nommant. Un sous-agent arrêté (`stop`, ou la fin
     du parent) ferme sa tâche, et sa branche reste pour la relecture.
4. **tty montre les sous-agents, et les témoins sont rouges d'abord.**
   - tty liste dans la zone vivante les enfants de la tâche suivie (nom, état, dernière ligne) ;
     `Ctrl-O` sur l'un ouvre sa boîte, et sa saisie y répond comme `reply`.
   - Les témoins, sur le faux modèle :
     - un parent délègue, le sous-agent pose une question, le parent répond par `reply`, le
       sous-agent écrit un fichier et finit ; `wait` rend son report et son diff, et la branche est
       fusionnée dans celle du parent ;
     - la profondeur et le nombre au-delà de leur borne sont refusés en les nommant ;
     - `stop` arrête le sous-agent ;
     - une ligne de sous-agent ne satisfait pas la garde de la demande du parent ;
     - un conflit rend une proposition, sans rien fusionner.

## Vérifié avant le premier rouge (c7 a lu la page : d'accord)

- **Une tâche sur le backend d'une tâche** : `ouvrir_une_tache` prend tout backend hébergé pour
  parent, donc une tâche aussi. Le dépôt et la base de départ se lisent sur ce backend : son worktree,
  dont `git worktree add` partage le dépôt. Deux corrections, chacune avec son témoin :
  - la branche est `agent/{nom}` (`taches.rs:634`) : une sous-tâche prend `agent/<parent>--<nom>` ;
  - le cas n'a pas encore de témoin.
- **`parent_task`** est écrit en dur à `null` (`daemon_host.rs:1645`) : il se tire du registre (la
  tâche parente d'une sous-tâche), pour que tty trouve les enfants.
