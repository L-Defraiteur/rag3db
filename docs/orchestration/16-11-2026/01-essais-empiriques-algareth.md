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
