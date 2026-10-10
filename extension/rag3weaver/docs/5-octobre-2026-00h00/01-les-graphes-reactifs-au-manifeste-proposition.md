# Proposition — une section du manifeste pour les graphes réactifs

5 octobre 2026, session mémoire longue. Une demi-page avant le code, demandée
par l'orchestration, qui a tranché la forme : une section **à part** des outils,
sa propre liste blanche de nœuds (`EventSourceNode` compris), montée au
chargement du backend par `Reactor::watch`. Générique : rien qui nomme la
mémoire.

## Pourquoi pas les outils — ce que le code dit déjà

Un outil est appelé par l'agent ; une réaction par un événement. Le moteur fait
déjà respecter la distinction, et c'est lui qui m'a appris que je l'avais
enfreinte : déclarer la fiche réactive du lot 4 dans `tools` faisait **refuser le
manifeste entier**, en le disant — « l'outil « review_anchored » contient
EventSourceNode, qui n'est pas un nœud d'outil de backend ». La liste blanche
vit dans `backend_code.rs` (`BASE_NODES` plus une liste par capacité).

Deux faits vérifiés qui cadrent la pièce :

- **`Reactor::watch(GraphTool)` existe** (`dataflow/reactor.rs:146`) et
  **personne ne l'appelle** dans tout `src/`. Il n'y a aujourd'hui aucun chemin
  par lequel un backend déclare ses graphes réactifs ;
- le réacteur a déjà son arrêt : `spawn() → ReactorHandle`, `stop()` qui rend le
  `Reactor`, et un `impl Drop` sur la poignée. Il n'y a donc pas de cycle de vie
  à inventer, seulement à brancher.

## La section

`"reactions"`, à côté de `"tools"` dans le manifeste. Même forme d'attachement —
un graphe et ses liaisons — parce qu'un gabarit qui sait déclarer un outil doit
savoir déclarer une réaction sans apprendre une seconde grammaire :

```json
"reactions": {
  "review_anchored": {
    "graph": "graphs/review_anchored.mmd",
    "bindings": { "source": "Subject", "target": "Memory",
                  "relation": "ANCHORED_TO", "transition": "review" }
  }
}
```

Ce que la fiche porte en plus d'un outil, et qu'elle porte **déjà** (`%% on:`,
`%% policy:`) : les sujets du bus et la politique (`each` / `batch <ms>` /
`debounce <ms>`). Rien à ajouter à la grammaire des fiches.

**Sa liste blanche est la sienne** : `BASE_NODES` plus `EventSourceNode` et
`ReactTransitionNode`. Et pas l'inverse — j'avais mis `ReactTransitionNode` dans
la liste des outils, je l'ai retiré : il lit ses uuid par un **port** qu'un
paramètre d'outil ne peut pas alimenter, donc il n'y servait à rien. **Une liste
de sécurité ne s'élargit pas « au cas où ».**

## Ce qu'un manifeste invalide dit

Trois refus, chacun au **chargement** et chacun nommant le remède — la règle du
dépôt, et la seule qui rende un refus utile à un modèle comme à une personne :

1. **un nœud hors de la liste réactive** : « la réaction « X » contient `N`, qui
   n'est pas un nœud de graphe réactif » — et, si `N` est un nœud d'outil, le
   dire : « c'est un nœud d'outil : déclare-le dans `tools` » ;
2. **une fiche sans `%% on:`** : le réacteur le refuse déjà, mot pour mot
   (« rien à surveiller — la fiche n'a pas de `%% on:` ») ; il suffit de laisser
   remonter ce refus avec le nom de la réaction ;
3. **un nom partagé entre `tools` et `reactions`** : refusé. Deux choses
   différentes sous un nom servent surtout à se tromper, et le curseur du
   réacteur **est** ce nom (`bus.cursor(topic, tool.name())`) : un homonyme
   partagerait silencieusement un curseur.

Et une quatrième qui n'est pas un refus : **une réaction dont la cible n'a pas
la transition déclarée**. Elle ne se voit qu'à l'exécution, puisque le nœud lit
la machine à états dans le catalogue ; elle doit donc arriver dans le reçu
d'ouverture, pas dans un flux que personne ne lit.

## Ce qui se passe à l'arrêt du backend

Le backend garde la `ReactorHandle` et l'arrête **avant** de lâcher le
catalogue : un graphe réactif écrit par les verbes du catalogue, et un réacteur
qui survivrait à sa base écrirait dans le vide. `stop()` rend le `Reactor`, donc
l'arrêt est **attendu** et non espéré.

Trois choses à tenir, et la première est la seule qui m'inquiète :

- **ce qui n'a pas été drainé n'est pas perdu** : le curseur garde sa place, la
  réaction reprendra à la prochaine ouverture. C'est déjà le contrat du curseur
  nommé ; le témoin doit le prouver, pas le supposer ;
- **un arrêt pendant une réaction** : la transition est écrite ou non, et la
  garde de cycle de vie rend l'écriture idempotente — rejouer la même transition
  sur une ligne déjà passée est un `out_of_state`, compté et non refusé ;
- **les erreurs d'une réaction se lisent** : `ReactorHandle::errors()` existe.
  Sans quoi on écrit une quatrième fois le défaut de la semaine — une
  information qui existe et que rien ne consulte.

## Les deux témoins exigés, et ce qu'ils doivent faire tomber

1. **Par le backend chargé depuis le manifeste**, pas par un graphe construit à
   la main. C'est exactement ce qui m'a manqué : mon témoin du lot 4 construit
   son graphe en structure, donc il contourne la politique des outils — il était
   vert pendant que le gabarit ne chargeait plus du tout.
2. **Un test qui charge chaque gabarit livré.** Il n'existe pas, et son absence
   a laissé passer la régression pendant une journée. Il se parcourt :
   `templates/backends/*/backend.json`, chargé, puis les outils et les réactions
   construits. **Un gabarit qui ne charge plus doit rougir la lib, pas attendre
   une sonde.**

Et un troisième que je propose en plus, parce qu'il coûte trois lignes : **le
nombre de nœuds de chaque liste blanche**, épinglé comme `BUILTIN_NODE_COUNT`
l'est. Une entrée ajoutée sans intention fait alors tomber un test au lieu
d'élargir une surface de sécurité en silence.

## Ce que je ne sais pas

- **Qui porte le réacteur quand plusieurs backends vivent dans un processus** :
  un réacteur par backend, ou un pour tous ? Le `Reactor` prend un bus, un
  registre de nœuds et des services — donc techniquement un par backend est le
  plus simple, et je ne sais pas ce que ça coûte en fils.
- **Si une réaction doit pouvoir être appelée à la main.** La fiche de trace dit
  « sans réacteur, s'exécuter à la main quand on veut », et c'est utile pour
  déboguer — mais l'exposer comme outil ramène la confusion qu'on vient de
  séparer. Je pencherais pour un verbe d'administration (`run_reaction <nom>`),
  hors du jeu d'outils du modèle, et je ne l'écrirais pas avant qu'on en ait
  besoin.
