# Ce qu'on annonce, et ce qu'il reste pour le tenir

Lucie, 23 novembre 2026, au soir : « maintenant faut respecter ce qu'on annonce ». La présentation de rag3weaver
annonce sept apports. Ce document dit, pour chacun, ce qui existe, ce qui manque, et qui le porte. Il se tient à jour :
une ligne ne passe à « tenu » que jouée par l'orchestration, pas sur parole.

L'ordre vient des deux lignes directrices : d'abord l'agent de code qui va au bout, puis le service sans écran
(« packager pour mieux régner »), puis ce tableau.

| Apport annoncé | Ce qui existe | Ce qui manque pour le tenir | Porteurs |
|---|---|---|---|
| **Respawn Agent** : rappeler un agent ou sa mémoire, depuis n'importe quel agent | Clé d'agent stable, mémoire de la conversation, reprise d'une séance (`--continue`) | Un outil pour interroger la mémoire d'un autre agent par sa clé ; relancer un agent arrêté depuis un autre agent ; la reprise au chargement (`resume_on_load`) ; `sessions.resume` du service | archi, mémoire, tty |
| **Code-First Search** | Index, recherche hybride, `usages`, `impact`, `callees`, `outline`, vues par branche | Une mesure sur un dépôt Python et TypeScript étranger (coût CPU, qualité) ; la liste des langages réellement couverts, prouvée | recherche, codeparsers, banc |
| **Shared Long-Term Memory** | Tours, fiches, apports, sujets, sources ; tours cherchables par le sens | Le rappel automatique posé (`recall_for_request`) ; les filtres `author` et `swarm` sur toutes les lectures ; le seuil de rappel mesuré | mémoire, banc |
| **Built-in Tickets** | La conception (champs, miroir en `.md`, liens, pièces jointes, `my_tickets`) | Tout le code : données de l'hôte et miroir, outils de lecture et d'écriture, assignation, lecture par l'orchestrateur | archi (à lancer) |
| **SWARMS** | Des agents qui se parlent et en lancent d'autres | Le swarm comme objet (titre à base et suffixe, membres, rôles) ; sa mémoire et ses tickets ; le super-orchestrateur ; le mode sans humain digne de confiance | archi, mémoire |
| **Hooks & Events** | Graphes et gabarits déclarés, garde des commandes, juge, bus d'événements | Des points de crochet déclarés dans la boucle de l'agent (l'inventaire du passage en DAG, phases 0 à 2) ; les événements du dehors (une PR qui touche à ton code, un conflit avec ta branche) et leur réveil | everything declarative, archi |
| **Host Independence & Multi-Hosting** | Deux hôtes reliés par tunnel, l'hôte lancé par socket | Le service sans écran `/v1`, l'image de conteneur, le registre de modèles ; **une session qui suit l'utilisatrice d'un hôte à l'autre** (à concevoir : rien n'existe) | archi, G, tty, lucivy |

## Ce qui est le plus loin de l'annonce

1. **Les tickets** : rien n'est codé.
2. **Une session qui suit d'un hôte à l'autre** : pas même conçue.
3. **Les événements du dehors** : le bus existe, aucun connecteur vers un dépôt distant.

## Une graine notée le même soir

Un connecteur pour créer des sous-domaines à la volée (un fournisseur de DNS et de tunnels), pour qu'une application
faite par un swarm ait tout de suite son adresse. À rapprocher de la vision du mini-hôte par application.
