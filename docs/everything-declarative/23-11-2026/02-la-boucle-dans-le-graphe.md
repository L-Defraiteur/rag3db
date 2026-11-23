# La boucle et l'événement dans le graphe : deux formes, une recommandation

23 novembre 2026, session everything declarative. Lucie, vers 23 h 30 : « même pour les événements et les boucles, ça
doit être des capacités du DAG, rien d'autre ; un event, c'est un nœud qui écoute quelqu'un émettre sur un channel,
non ? ». Cette page compare les deux formes que c7 propose pour la boucle (les primitives 1 et 2 de la page 01), et en
recommande une. Elle précède tout code.

## Ce qui est commun aux deux formes : `emit` et `listen`

Deux nœuds primitifs, chacun ne fait qu'une chose.
- **`emit(channel=…)`** : publie son entrée sur un canal nommé. Ce sont les canaux du bus (`agent`, `tokens`,
  `mailbox`…) et ceux qu'un gabarit déclare.
- **`listen(channel=…, filter=…, timeout_s=…, on_timeout=…)`** : attend un élément du canal qui passe le filtre, puis
  le rend. L'échéance tombée, il rend `on_timeout` ; l'annulation du run l'arrête. **Un déclencheur de graphe est un
  `listen` en tête** : le réveil d'une session, une ligne de boîte, une décision humaine, le résultat d'un outil en
  fond, et un rappel périodique, qui est un `listen` sans émetteur, réveillé par son échéance.

## Forme A : un nœud `loop`

`loop(guard=…)` rejoue un sous-graphe tant que la garde dit oui, et porte l'état d'un tour au suivant.
- **Pour** : l'écriture est familière, et tout le tour vit dans un seul run.
- **Contre** :
  - le moteur gagne un cycle, alors que tout ce qu'il sait (la validation, le checkpoint, la trace, l'aperçu) suppose un
    graphe acyclique ;
  - l'état du tour vit dans le nœud : il faut le sérialiser à part pour survivre à un redémarrage ;
  - la boucle ne se voit pas dans le mmd : c'est un nœud parmi d'autres ;
  - `loop` compose plusieurs choses à la fois (rejouer, garder l'état, évaluer la garde), ce que la règle de Lucie
    refuse à une primitive.

## Forme B : la ré-entrée par événement

Le graphe d'un tour finit par `emit` sur un canal que ce même graphe écoute en tête, sauf si la chaîne de gardes dit
stop. Chaque exécution reste **acyclique** ; la boucle, ce sont deux nœuds visibles dans le mmd.

```
start["listen(channel='step', filter='task')"] --> facts["StepFacts()"] --> step["…le pas…"] --> chain["FirstMatch(…)"]
chain -->|relaunch| again["emit(channel='step')"]
chain -->|wait| wait["listen(channel='mailbox', filter='reply_to', timeout_s=600, on_timeout='end')"] --> again
chain -->|end| done["emit(channel='ended')"]
```

- **Pour** :
  - le moteur reste celui d'aujourd'hui : rien à changer à la validation, au checkpoint ou à la trace ;
  - **l'état du tour voyage dans l'élément émis** (le compteur, « relancé une fois », le curseur) : il est durable si
    le canal l'est ;
  - **un redémarrage reprend au dernier élément** du canal, sans état caché ;
  - **une seule paire de primitives** sert la boucle de l'agent, celle de l'équipier, le réveil d'une session, le
    sondeur du dehors et le rappel périodique ;
  - chaque tour est une trace à lui, lisible seule.
- **Contre** :
  - chaque tour instancie le graphe : quelques millisecondes, sans poids à l'échelle d'un pas ou d'un appel de modèle
    (des secondes). Le rendu des jetons reste compilé dans tous les cas ;
  - il faut des canaux **durables** (journalisés) pour reprendre après un redémarrage, et un canal privé à une tâche ou
    à un run, pour qu'un tour ne réveille pas celui d'un voisin.

## La recommandation : la forme B

Elle tient la règle de Lucie mot pour mot : un événement est un nœud qui écoute un canal, et une boucle est un graphe qui
s'écoute lui-même. Elle ne demande qu'une paire de primitives à usage unique, et le moteur ne change pas de nature.

**Ce qu'il faut construire, dans l'ordre** :
1. **les canaux nommés et durables** : un journal par canal, la portée (tâche, run, poste) et une clé de reprise ;
2. **`emit` et `listen`**, `listen` avec son filtre, son échéance et l'annulation par le run. Le témoin : un graphe qui
   s'émet trois fois puis s'arrête sur sa garde, puis reprend après un redémarrage simulé au tour où il était ;
3. **le déclencheur** : un graphe déclaré dont la tête est un `listen` est lancé par l'hôte à chaque élément du canal,
   avec un vol unique par clé (la primitive 6 de la page 01 devient une option de `listen`) ;
4. **la chaîne de gardes de fin de pas** (avec archi), qui décide de l'`emit` suivant ;
5. **les boucles réécrites**, chacune dans son lot : le réveil de l'équipier, son pas, puis la boucle de l'agent.
