# L'éclaireur de l'index : la recherche faite pour l'agent, au début du tour

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
2. **Les requêtes** : un appel court au modèle, à sortie structurée, tourne la demande en deux ou trois requêtes de
   `search_code` (et un nom à passer à `usages` s'il en voit un).
3. **Les recherches** : chacune par `CallTool`, en parallèle.
4. **La synthèse** : un appel au modèle, à sortie structurée, met la demande en regard de tous les résultats :
   - `orientation` : comment le projet est organisé pour cette demande, où sont ses primitives, ce qui s'y rattache ;
   - `relevant` : les résultats les plus utiles, par leur numéro dans les recherches, avec `chemin:ligne`, le symbole
     et une raison en une ligne ;
   - `queries` : les requêtes jouées, pour que l'agent sache ce qui a déjà été cherché.
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

## Bornes

- Le modèle des deux appels courts : le plus léger, déclaré par modèle (le juge des lots a retenu Gemini 3.1
  Flash-Lite). Ce sont deux appels par tour ouvert par une demande, pas par pas.
- Sortie bornée (jetons, nombre de résultats) ; chaque `chemin:ligne` vient d'un résultat de recherche, jamais du
  modèle : un nœud déterministe écarte une ligne qui n'est pas dans les résultats, comme le filtre des extraits du
  juge des lots.
- Coupable sans bâti, par la table de l'utilisatrice.

## La mesure

- Sur l'invite d'étude de Lucie et sur la demande Houdini dans `tisserande` : le nombre de fichiers lus en entier avant
  d'agir, avant et après ; les noms absents du code dans ce qu'il écrit ; le temps du tour.
- Sur le rejeu humain de tty : le « bonjour » ne change pas de durée ; le petit tour de code garde ses durées.
