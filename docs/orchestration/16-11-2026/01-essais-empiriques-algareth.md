# Essais empiriques d'Algareth, par c7

Lucie, 16 novembre 2026, 0 h 5x : « joue mon propre rôle jusqu'à ce qu'on sache vraiment s'en servir comme
autre session ; fais-le sans moi, ça devrait être ça vos tests : pas unitaires, empiriques ; contrôler chaque
outil de recherche, s'il a bien marché, etc. »

Protocole : une tâche à c7 sur le backend de Lucie (`c7-essai`, agent Algareth), le terminal en mode `--plain`
avec `--demande` et `--journal`, une demande à la fois ; pour chaque demande, l'attendu est écrit **avant**, puis
les appels d'outils relevés dans le journal (nom, arguments, retour) et la réponse ; le verdict est VERT si
chaque outil a fait ce qu'il dit et si la réponse est juste, ROUGE sinon, avec la cause et le ticket chez la
session qui porte la couche. Les demandes vont du plus simple (une recherche) au plus long (un témoin rouge puis
vert sur un défaut cadré).

| n° | demande | attendu | outils relevés | réponse | verdict |
|---|---|---|---|---|---|
| D1 | « Dans quel fichier et à quelle ligne est définie `valider_le_nom_de_l_agent` ? chemin relatif et ligne, rien d'autre » | `extension/rag3weaver/src/backend.rs:995` par `search_code` ou `usages` (l'index est prêt), un ou deux appels | grep_files ×5, read_file ×4, run_command `git rev-parse --show-toplevel` (accord demandé, resté en attente : « en attente du superviseur depuis 00:32 »), list_files ×1 ; **aucun search_code, outline ni usages**, alors que la tâche les a | « Je suis Algareth. extension/rag3weaver/src/backend.rs:995 » ; l'hôte dit « le pas 1 s'est terminé sans step_done » | **VERT sur la réponse, ROUGE sur la voie** : 11 appels dont 9 lectures brutes pour une définition que l'index donne en un ; une commande en lecture seule bloquée à l'accord en mode plain ; se présente dans une réponse qui ne le demande pas. Tickets : archi (l'index d'abord, grep en repli ; le pas sans step_done dit comme un défaut) ; porte (un `git rev-parse` en lecture seule passe en auto, vision point 6) |
