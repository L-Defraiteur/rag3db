# Le chat qui code dans une tâche, sur un dépôt étranger

Le lot de la nuit du 25 octobre 2026. c7 : « l'IDE sur un dépôt étranger avec le chat qui y
code, c'est bien ce que Lucie veut voir ».

## Ce qui existe déjà

- L'IDE ouvre un dépôt d'à côté (marche 3). Une tâche y ouvre un worktree et une branche
  `agent/<nom>`, avec son propre backend : son workspace a pour racine ce worktree, et ses
  déclarations sont en `Auto` (`taches.rs`, `Surcharges`).
- Le chat de l'atelier (K1, K2) tourne sur le backend qu'on lui donne, avec **tous** ses outils
  (`BackendTools`, aucun filtre) plus ceux des déclarations. Sur une tâche, il a donc déjà
  `edit_file`, qui écrit dans le worktree.
- La garde de l'agent (`Backend::refus_de_l_agent_hors_tache`) refuse une écriture sur le
  principal **seulement si le manifeste dit `task_writes`**. Sans ce drapeau, le chat d'un
  principal peut écrire l'arbre principal.
- Le message système ne parle que des déclarations.

## Ce que le lot pose

1. **Le chat n'écrit jamais hors d'une tâche**, quel que soit le manifeste. Hors d'une tâche, les
   outils d'un backend qui écrivent les fichiers (`policy.write_files`) ne sont pas proposés au
   modèle, et un appel par leur nom est refusé en disant d'ouvrir une tâche. Les outils qui
   lancent des commandes (`policy.run_commands`) ne le sont nulle part dans le chat : coder, ici,
   c'est lire et éditer.
2. **Un seul message système.** Le verbe des déclarations suit le mode, comme avant. Une seule
   phrase sur le code suit le fait d'être, ou non, dans une tâche :
   - dans une tâche : lire et chercher (outline, grep, search, read), puis éditer avec
     `edit_file`, sur la branche de la tâche, que la personne relit puis fusionne ;
   - hors d'une tâche : le code se lit, il ne s'écrit pas, et on demande d'ouvrir une tâche.
3. **Un tour qui a écrit le dit.** La réponse d'un tour sur une tâche porte `tache` (nom,
   branche) et `fichiers`, les chemins que ses `edit_file` réussis ont touchés, lus dans les
   tours d'outil de ce tour-ci. La bulle mène à la relecture de la tâche, où se voient diff,
   témoins et fusion.

## Les témoins

- **Le principal intact, octet pour octet** : un dépôt de banc. Une tâche est ouverte, et un faux
  modèle appelle `edit_file` dans le chat de la tâche. Le fichier change dans le worktree. Dans
  le dépôt principal, chaque fichier hors `.git` garde son contenu, et `HEAD` ne bouge pas.
- **Le chat du principal n'écrit pas** : sur le principal d'un manifeste **sans** `task_writes`,
  `edit_file` n'est pas dans les outils proposés, et un appel par son nom est refusé avec « ouvrez
  une tâche ». Le principal est intact.
- **Dans la page** : le tour de la tâche montre son appel `edit_file`, puis un lien vers la
  relecture de la tâche, avec le fichier touché.
- **Gemini, jugé sur le résultat** (#[ignore], comme K3) : un petit dépôt, une tâche, une demande
  de changement. Le fichier attendu change sur la branche, le principal est intact, et la mesure
  du tour est donnée (durée, appels du modèle, appels d'outils).

## Ce que le lot ne fait pas

- Le chat n'ouvre pas de tâche lui-même : c'est la personne qui l'ouvre, depuis l'IDE. C'est la
  voie suivante si Lucie la veut.
- Pas de commandes dans le chat.
