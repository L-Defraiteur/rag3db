# L'heure d'un appel d'outil est celle de l'insertion du tour, pas celle de l'appel

- **État** : ouvert (7 novembre 2026), pour everything declarative — lot « le chat tient l'ordre ».
- **Gravité** : réponse fausse (une heure enregistrée qui ne dit pas quand l'outil a été appelé).
- **Atteignable en service** : oui, tout tour du chat ou d'un équipier qui appelle des outils.
- **Touche rag3weaver** : oui (la boucle de l'agent, le journal des runs, le rendu du chat).

## Ce que c'est

Lucie, sur l'IDE posé à 17 h 22 : « le temps enregistré au moment de l'appel d'un outil doit être
en mémoire vive avant d'arriver dans la base, pour ne pas être le temps à l'insertion du tour ».

## Ce qui est là

Un `Turn` (`src/llm.rs:353`) ne porte aucune heure. Les tours d'un tour de chat sont écrits dans
le journal du run à sa fin (`run_chat_turn`, `ajouts["tours"]`) : ce qui les date ensuite, c'est
l'insertion. Seuls les événements du bus (`ToolCallStarted`, `ToolCallFinished`) sont datés au
moment du fait.

## Ce qu'il faut pour le fermer

L'heure d'un appel d'outil est prise **au moment de l'appel**, gardée en mémoire avec l'appel
(côté hôte ou run), puis écrite telle quelle avec le tour (un champ `at_ms` par appel et par
résultat, facultatif pour les tours d'avant), et c'est elle que le chat rend et trie. Témoin par le
chemin de l'hôte : un tour avec deux outils puis la réponse ; les heures enregistrées des appels
précèdent celle du tour et se suivent dans l'ordre des appels.
