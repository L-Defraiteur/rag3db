# L'éclaireur de l'index : la recherche faite pour l'agent, au début du tour

- **État** : ouvert (24 novembre 2026, orchestration).
- **Gravité** : fonction neuve — l'agent n'utilise pas l'index sur un sujet neuf.
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (l'équipier, at_turn_start).
- **Famille** : produit
- **Pour** : rag3weaver archi (28 novembre, branche c7/24-11-2026-eclaireur à reprendre).
- **Signalé par** : Lucie.

Ouvert le 24 novembre 2026, 12 h 40. Proposition de Lucie, le même jour : « peut-être on devrait avoir un mini agent qui
lance deux trois recherches, ensuite sur toutes les recherches un appel structuré qui met en relation la question
de l'utilisatrice et explique à l'agent le code qu'il a trouvé ».

## Le constat

Le 24 novembre, dans `tisserande` indexé, l'agent reçoit « le dossier a son index de code : sers-t'en d'abord ». Il
reçoit aussi une invite qui le lui demande, et des descriptions d'outils qui disent ce que l'index trouve. Sur une
demande réelle (des graphes de géométrie à la Houdini, avec leur interface), il n'appelle pourtant ni `search_code`, ni
`usages` : il liste et lit des fichiers. Depuis le 23, sur toutes les mesures, il ne cherche dans l'index que quand on le
lui demande mot pour mot. Le réflexe ne viendra pas par l'invite seule.

## Ce qui est proposé

L'index sert l'agent sans attendre qu'il le choisisse. Au début d'un tour ouvert par une demande, dans le graphe
`templates/teammate/graphs/at_turn_start.mmd` (le rappel de mémoire y est déjà) :

1. **Une condition** : une demande sur le code, dans un dossier dont l'index est prêt. Un « bonjour » ne lance rien.
2. **Les requêtes** : un appel court au modèle, à sortie structurée (pas un mini-agent : Lucie et c7, 24 novembre
   2026), reçoit la demande, le contexte récent de la conversation et le plan du projet que l'index connaît (ses
   dossiers de tête, les noms de ses modules), et rend trois requêtes de `search_code` (et un nom à passer à `usages`
   s'il en voit un).
3. **Les recherches** : chacune par `CallTool`, en parallèle.
4. **La synthèse** : un appel au modèle, à sortie structurée, met la demande en regard de tous les résultats :
   - `orientation` : comment le projet est organisé pour cette demande, où sont ses primitives, ce qui s'y rattache ;
   - `relevant` : les résultats les plus utiles, par leur numéro dans les recherches, avec `chemin:ligne`, le symbole
     et une raison en une ligne ;
   - `queries` : les requêtes jouées, pour que l'agent sache ce qui a déjà été cherché.
   - `follow_up` : zéro à deux requêtes de plus, quand un résultat ouvre une piste (un nom à suivre par `usages`, une
     interface dont il faut les implémentations). **Une seule relance** : ces requêtes jouées, la synthèse est refaite,
     et c'est fini (une boucle à `max_iterations: 1`, déclarée, comme toute boucle arrêtée par un modèle).
5. **Le rendu** : un bloc « Ce que l'index a trouvé pour ta demande », avec la synthèse et les lignes `chemin:ligne`.
   Il passe d'un pas à l'autre comme un fait (`carry_over`).

## Quand l'envoyer : en parallèle, jamais en attente

Recommandation de c7 : **le tour part tout de suite**, et l'éclaireur tourne en fond, lancé au début du tour. Son
bloc arrive **dès qu'il est prêt**, par la boîte : une ligne de l'hôte, que la boucle insère déjà en tête du prochain
retour d'outil (« Pendant tes appels d'outils, la boîte a reçu… »). Ce mécanisme existe déjà, aucun nœud neuf n'est
nécessaire.

- Le premier appel au modèle ne paie pas l'attente (deux appels courts et trois recherches : quelques secondes).
- Si le tour finit avant, le bloc n'est pas perdu : il entre au tour suivant, ou il est jeté s'il ne sert plus. C'est
  à trancher.
- Plus tard, le nœud d'attente par événement (ticket du travail lourd) permettra d'écrire ce lancement et cette
  attente en graphe pur.

## Le score de connaissance et le mode bloquant (Lucie, 24 novembre 2026, 12 h 50)

- **Le premier appel rend aussi `knows`, un score de 0 à 1** : « l'agent semble-t-il déjà avoir, d'après le contexte
  récent, les connaissances requises pour répondre à cette demande ? ». Il ne coûte rien de plus (même appel, un champ
  de plus). Sous un **seuil déclaré** dans la table, les recherches partent ; au-dessus, rien (l'agent a déjà lu ce
  qu'il faut ce tour-ci ou au précédent). Le score et la décision sont écrits dans la trace.
- **Trois modes, déclarés dans la table** : `parallel` (le défaut : le tour part, le bloc arrive dès qu'il est prêt),
  `blocking` (le premier appel au modèle de l'agent attend le bloc, borné par une durée, au-delà il part sans), `off`.
- Un réglage qui combine les deux, à mesurer : bloquant quand le score est très bas (l'agent ne sait rien du sujet,
  souvent le premier tour dans un dossier), parallèle sinon.
- **Une réserve** (c7) : un score qu'un modèle se donne est mal calibré. Le seuil se règle sur le banc (l'invite d'étude
  de Lucie, la demande Houdini, le rejeu de tty), pas à l'œil, et la trace garde score et décision pour le régler.

## Bornes

- Le modèle des deux appels courts : le plus léger, déclaré par modèle (le juge des lots a retenu Gemini 3.1
  Flash-Lite). Ce sont deux appels par tour ouvert par une demande, pas par pas.
- Sortie bornée (jetons, nombre de résultats) ; chaque `chemin:ligne` vient d'un résultat de recherche, jamais du
  modèle : un nœud déterministe écarte une ligne qui n'est pas dans les résultats, comme le filtre des extraits du
  juge des lots.
- Coupable sans bâti, par la table de l'utilisatrice.

## La mesure

- Avec et sans la relance, sur les mêmes demandes : si elle ne rapporte rien, elle se coupe dans la table.

- Sur l'invite d'étude de Lucie et sur la demande Houdini dans `tisserande` : le nombre de fichiers lus en entier avant
  d'agir, avant et après ; les noms absents du code dans ce qu'il écrit ; le temps du tour.
- Sur le rejeu humain de tty : le « bonjour » ne change pas de durée ; le petit tour de code garde ses durées.
