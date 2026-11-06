# Le dépôt qui sait pourquoi

Lucie, 6 novembre 2026, à propos d'un billet sur le rachat d'un « Git pour les
machines » (dépôts énormes, pushs rapides, les PR d'agents multipliées par six
en huit mois ; les chiffres du billet ne sont pas vérifiés ici) : « on ne
devrait pas imaginer un service du même style ? »

## Pas le même service

Un Git de stockage pour agents est une guerre d'infrastructure : centres de
données, latence, partenariats. Ce n'est pas la nôtre. Ce que six fois plus de
PR d'agents rendent impossible, ce n'est pas de les stocker, c'est de les
**relire** : un diff ne dit ni pourquoi l'agent a fait ça, ni ce qu'il a lu, ni
s'il s'est trompé d'outil en route, ni si c'est mieux que la fois d'avant. Le
goulot est la confiance, et c'est là qu'on est déjà.

## Ce qu'on a sans l'avoir nommé

- **Une branche de tâche porte sa preuve** : le journal du run (les appels,
  leurs retours tels que l'agent les a reçus, les durées), le plan de
  l'équipier et ses pas, les propositions, le verdict du juge sur la session
  et sur chaque outil, la tendance par rapport aux sessions précédentes de la
  même tâche. La relecture ne lit pas un diff, elle lit une tâche.
- **Le code est un graphe** : impact, usages, appelants, le plan d'un fichier,
  la relecture par le graphe que l'IDE rend ; aucun hébergeur de code ne l'a.
- **La mémoire des décisions** : fiches de session, roadmaps et phases,
  identité des agents à clé stable, sujets partagés et fusion proposée ; chaque
  changement pointe vers la raison qui l'a produit.
- **Le pont** : les agents de Claude Code, Codex, Cursor passent par notre MCP ;
  l'équipier est le nôtre ; le même hôte juge les deux.

## La forme du service

Un service **au-dessus de Git**, pas à sa place. Chaque dépôt branché reçoit son
hôte, son index (un index par dépôt, des vues par worktree : voir la vision
du même jour), son juge et sa mémoire. Les agents y ouvrent des tâches ; les
humains relisent par le graphe et par le verdict, et fusionnent. Le stockage
reste chez GitHub ou ailleurs. Ce qu'on vend, c'est la confiance dans ce que
les machines ont écrit, et la vitesse de fusion qu'elle rend.

## Deux prudences

- **Se prouver sur nous d'abord** : l'équipier et le juge sur nos propres
  dépôts pendant quelques semaines, les bilans dans la mémoire ; mesurer si la
  relecture par tâche change la vitesse à laquelle on fusionne. Sans cette
  mesure, pas de service.
- **L'hôte d'équipe d'abord** : multi-tenant, authentification, quotas, un
  index par dépôt et un routeur (vision du 18 octobre) ; c'est lui qu'on
  réveille en premier, le service en est la face publique.

Rien n'est décidé ni commencé. Cette page dit le sens ; la vision « un index
par dépôt, des vues par worktree » dit la première marche concrète.
