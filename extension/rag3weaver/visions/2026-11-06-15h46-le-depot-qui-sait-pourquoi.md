# Le dépôt qui sait pourquoi

Lucie, 6 novembre 2026, à propos d'un billet sur le rachat d'un « Git pour les
machines » (dépôts énormes, pushs rapides, les PR d'agents multipliées par six
en huit mois ; les chiffres du billet ne sont pas vérifiés ici) : « on ne
devrait pas imaginer un service du même style ? »

## La thèse

Relecture extérieure du 6 novembre, reprise parce qu'elle est plus juste que
la nôtre : la valeur de ce système **augmente à mesure que les agents
deviennent interchangeables**. La mémoire conserve la compréhension
structurelle du dépôt, les décisions accumulées sur des mois, l'histoire des
transformations de chaque symbole, les réussites et les échecs des agents, et
les relations entre intentions, modifications et résultats ; un modèle neuf
hérite du contexte opérationnel des précédents, et la valeur accumulée
appartient à la mémoire, donc à celui qui possède le dépôt, pas à l'agent qui
l'a produite. Le centre n'est pas l'assistant : c'est **le programme et son
évolution**. Un humain doit pouvoir s'en servir sans aucune IA.

Ce que cette liste nomme existe déjà, pièce par pièce : la compréhension
structurelle est l'index en graphe ; les décisions sont les fiches, les
roadmaps et les identités ; les réussites et les échecs sont les bilans du
juge ; les relations intentions → modifications → résultats sont le plan de
l'équipier, les apports `diff` et le verdict sur la même fiche de session.
La seule ligne qui manque est l'histoire des transformations de chaque
symbole : c'est le prototype vertical du 9 novembre.

Deux tests en découlent, à tenir à chaque lot :
- **sans modèle** : tout ce qui n'est pas génération (recherche, graphe,
  vues, relecture, `find_tasks`, la mémoire) marche sans aucun modèle
  déclaré, et les témoins le jouent ainsi ; seuls le chat, l'équipier et le
  juge en demandent un ;
- **sans repartir de zéro** : une tâche commencée par un agent (Gemini,
  Claude Code par le pont, un modèle local) est reprise par un autre à partir
  de la fiche de session, du plan et des bilans, et le juge compare les deux
  sur la même fiche ; témoin à écrire quand l'équipier et les hooks seront là.

La question qui en sort n'est pas « qui est le plus visionnaire », mais
« pourquoi un fournisseur de modèles construirait-il volontairement une
infrastructure qui le rend moins indispensable ? ». C'est une divergence
d'objectifs, pas de moyens, et c'est elle qui laisse la place à une
architecture indépendante. Ce qu'on en fait : ne pas chercher à bâtir le
meilleur outil pour les agents de 2026, mais une représentation du
développement logiciel qui reste juste quand ces agents seront obsolètes.

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
- **Un changement est lisible comme une fiche** : chaque version de scope qui
  diffère dans une tâche est un apport `diff` sur la fiche du fichier, du
  symbole et de la session ; « qu'est-ce qui change dans cette PR » se lit dans
  la mémoire, groupé par sujet, par un agent comme par une personne.
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

## Retrouver une PR par ce qu'elle est

Lucie : « un agent veut trouver rapidement quelle PR a été faite par tel
auteur, parle de tel truc dans sa description, et touche tel scope ». Tout y
est déjà cité, donc c'est une seule requête sur la mémoire, pas trois
systèmes : l'auteur est la Ref `agent:<clé>` de la fiche de session ; la
description est le texte de cette fiche (la demande, le plan), rappelé en
plein texte et en dense comme toute fiche ; le scope touché est la Ref
`symbol:<scope>` (ou `path:`) des apports `diff` de la session. La requête :
les fiches de session citées par `symbol:<scope>` à travers leurs apports,
dont l'auteur est `agent:<clé>`, classées par le rappel de « tel truc » ; la
réponse rend la tâche, sa branche, et ses apports `diff` groupés par fichier.

En forme produit : un Plan déclaré, l'outil `find_tasks { author, about,
touches }`, rendu en Cypher par l'IR comme les autres Plans, les signaux de
recherche visibles, la vue en paramètre comme toute lecture. Un humain pose
la même question dans l'IDE, par la vue « mes recherches » ou la fiche d'un
symbole (« qui l'a touché, quand, pourquoi »).

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
