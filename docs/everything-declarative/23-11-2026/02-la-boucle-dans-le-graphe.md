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

## Les conditions

Lucie, 23 h 40, précisé à 23 h 50 : « en plus, on a aussi des nœuds de condition IA ». Une condition est **un nœud
comme un autre**, utilisable partout dans un workflow, pas d'abord pensé pour arrêter une boucle. Il en existe deux
sortes, et elles ont la même sortie : un verdict borné.

- **La condition déterministe** répond à une question sur des faits : « le pas a-t-il écrit après son dernier
  outil ? », « le tour vient-il d'un humain ? ». Elle a une entrée de faits et rend `{holds, why}`. `All`, `Any` et
  `Not` les combinent, `FirstMatch` les lit dans l'ordre déclaré. Les primitives sont écrites (branche
  `ed-guard-chain`), et les conditions de fin de pas d'archi aussi (branche `step-conditions`).
- **La condition par un modèle** répond à une question de jugement : « le tour est-il fini ? », « cette réponse
  répond-elle à la question ? » (le juge du contenu), « cette commande est-elle dans l'esprit de la demande ? » (le juge
  de la porte), « ce signal est-il le même qu'un autre ? ». C'est **`Decide`** : un contexte, une question, des
  étiquettes déclarées, et un moteur déclaré (un petit modèle en sortie structurée, ou un classifieur quand il y en a
  un). Elle rend **une étiquette de la liste et un score**, jamais du texte libre, et `Route` branche dessus.

**Ce que `Decide` sait déjà** :
- des étiquettes bornées (une liste, ou `labels_from` dans l'élément) ; une étiquette hors liste est refusée ;
- le classifieur du crate (`Decider`) quand il est servi, qui rend une probabilité ; sinon le modèle de génération sous
  une sortie structurée (un schéma à `enum`) ;
- une consigne (`instruction`) et un gabarit qui rend les champs de l'élément ;
- l'élément rendu porte son étiquette dans le champ `as` ; `Route` branche sur ce champ.

**Ce qui lui manque, à ajouter avant de s'en servir comme condition** :
- **une entrée `context` en tableau**, rendue dans l'invite, à côté de la question (aujourd'hui, un seul élément) ;
- **un seuil et une sortie `unsure`** : sous le seuil, ou quand le modèle échoue ou répond hors liste, l'élément part
  sur `unsure` au lieu d'une erreur. Jamais une étiquette inventée ;
- le moteur nommé dans le nœud (`via`, un modèle de `models.llm`, un classifieur du registre), ce que `via` fait déjà en
  partie.

## La règle de validation des boucles

Une boucle est une ré-entrée : un `emit` vers un canal que la tête du même graphe écoute.

- **Si la décision de ré-entrer dépend d'un modèle** (un `Decide` en amont de l'`emit`), **le graphe déclare un
  plafond** (`max_iterations` sur le `listen` de tête). Sans lui, **le graphe est refusé à la validation**, avec un
  message qui nomme le `Decide` et le canal. Ce n'est ni un refus à l'exécution, ni un défaut silencieux.
- **Si la décision est déterministe**, le plafond n'est pas exigé. Je recommande tout de même **un plafond par
  défaut de 200 ré-entrées par clé et par run**, dit quand il tombe (une ligne `ended`, cause `max_iterations`).
  Une garde déterministe peut avoir un défaut ; chaque tour peut appeler un modèle payant ; et la boucle de l'équipier
  s'arrête aujourd'hui à 40 pas. Un graphe qui en veut plus le déclare.

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

## Brancher ses nœuds sans recopier le graphe livré

Lucie, 23 novembre : « je ne sais pas si c'est terrible de parler de hook, vu qu'en fait c'est juste des sorties de
nœud, non ? ». Dans un graphe tout déclaré, il n'y a pas de mécanisme « hook » : une vérification après la réponse est
un nœud de condition branché sur la sortie du nœud « réponse », et sa branche « pas bon » fait un `emit` vers le canal
de tête. **Ce qui reste à concevoir**, c'est la manière dont l'utilisatrice branche ses nœuds sur le graphe livré
**sans le recopier**. Une copie fige le graphe, et nos mises à jour ne lui arrivent plus.

**Ce qui existe déjà** :
- **le sous-graphe en nœud** (`GraphNode`) : un graphe entier devient un nœud, dont les ports libres sont les entrées et
  les sorties ;
- **le sous-graphe cité** (`CitedGraphNodeFactory`) : un nœud qui nomme un graphe d'un registre, au lieu de le recopier ;
- **les gabarits livrés** par `builtin:<nom>`, avec la copie locale gardée à part quand l'utilisatrice en modifie un.
  C'est le piège de la copie figée ;
- les directives du mmd (`param`, `result`, `tool`, `description`, `on`, `policy`, `choices`, `async`). **Aucune
  n'importe ni n'étend un autre graphe.**

**Ce qui manque : la surcouche.**
1. Dans un graphe livré, chaque nœud a **un nom stable**, son identifiant dans le mmd, que nos mises à jour s'engagent à
   garder ; un renommage se dit dans la nouveauté du lot.
2. **Une surcouche est un petit mmd** qui commence par `%% extends: builtin:<graphe>`. Ses arêtes citent les nœuds du
   livré par leur nom :
   ```
   %% extends: builtin:agents/chat/turn.mmd
   %% between: reply.text
   reply -->|text| check["Decide(question='la réponse répond-elle à la demande ?', labels='yes|no', threshold=0.7)"]
   check -->|no| again["emit(channel='turn')"]
   ```
   `%% between: <nœud>.<port>` interpose la surcouche entre cette sortie et son aval ; sans elle, les nœuds s'ajoutent à
   côté de l'aval existant.
3. **L'hôte compose à l'ouverture** le livré et la surcouche en un seul graphe, puis le valide entier, règle des boucles
   comprise. **Une surcouche qui cite un nœud ou un port disparu est refusée**, en nommant le nœud et le graphe livré, et
   le livré seul tourne. Ce n'est jamais un défaut silencieux.
4. **Les mots** : dans les textes pour les utilisateurs, « hook » reste le mot. Dans le code et les gabarits, on dit
   **sortie de nœud** et **surcouche** (`extends`).

**Le témoin** : une surcouche qui s'interpose après `reply` relance le tour ; un livré dont le nœud est renommé fait
refuser la surcouche, avec son nom. La surcouche vient en phase 1, après `emit`, `listen` et la chaîne de gardes,
puisqu'elle les compose.

## La surcouche par sélecteur

Lucie, 23 novembre (à réfléchir, pas à coder) : « c'est du méta-DAG : tu te branches sur un autre DAG, quelques nœuds
en override, ou sur des fonctionnements du DAG : "sur chaque boucle for de ce DAG", "sur tous les nœuds qui répondent à
telle condition", "sur tel nœud, overrider une autre sortie vers ça" ».

**Les sélecteurs**, du plus étroit au plus large :
- **par nom** : un nœud du graphe livré (`%% between: reply.text`), ce que la section précédente prévoit ;
- **par genre de nœud** : `%% on: kind=Decide`, `kind=LlmNode` (« tout appel au modèle »), `kind=RunCommandNode`
  (« toute commande »), `kind=ForEach` ;
- **par prédicat** sur les paramètres déclarés ou sur un port : `%% on: port=text`, `%% on: kind=LlmNode, via=vertex`.
  Ce n'est qu'une égalité sur ce que le nœud déclare, jamais du code ;
- **l'override d'une sortie** d'un nœud nommé : `%% redirect: check.no -> again`, qui branche une sortie ailleurs que
  là où le livré l'envoie.

**Ce que ça ouvre** : les préoccupations transverses, sans toucher un seul graphe.
- tracer chaque appel au modèle ;
- garder chaque commande (une condition avant chaque `RunCommandNode`) ;
- mesurer chaque nœud ;
- relire chaque écriture.

C'est « brancher ses propres hooks n'importe où », sans qu'un seul graphe livré change, et nos mises à jour continuent
d'arriver.

**Les garde-fous, pour que ça reste lisible** :
- **le graphe composé s'affiche toujours** (`rag3weaver graph --composed`, et dans l'IDE), avec, sur chaque nœud
  inséré, la surcouche qui l'a mis ;
- **chaque surcouche dit ce qu'elle a touché** (les nœuds sélectionnés, les arêtes ajoutées ou redirigées). Elle
  **échoue si elle ne touche rien**, sauf si elle est déclarée `optional` ; un sélecteur qui ne trouve plus rien après une
  mise à jour se voit donc ;
- **l'ordre de plusieurs surcouches est déclaré**, et chacune ne sélectionne que les nœuds du graphe tel qu'il était
  avant elle, jamais ceux qu'elle insère : pas d'auto-application, pas de récursion ;
- **la validation porte sur le graphe composé**, règle des boucles comprise.

**Ce que je refuserais** :
- **une surcouche qui sélectionne une garde dure** (le masquage des secrets, le registre et le plafond des dépenses, les
  accords de dossiers, la signature humaine, l'octroi d'une commande) : ces nœuds sont **scellés**. Un sélecteur qui les
  atteindrait est refusé à la composition, en les nommant ;
- **un prédicat en code** (un script dans le sélecteur) : le sélecteur reste une égalité sur ce qui est déclaré, pour que
  « qu'est-ce que cette surcouche touche ? » se lise sans exécuter quoi que ce soit ;
- **retirer un nœud du livré** : une surcouche insère, enveloppe ou redirige, elle ne supprime pas. Pour faire moins, on
  redirige une sortie vers un nœud qui ne fait rien, et ça se voit dans le composé ;
- **les sélecteurs sur le genre de nœud dans les chemins chauds** (par jeton, par appel d'outil quand il y en a des
  centaines) sans une mesure avant et après.

**Le coût** : la composition est une passe sur les nœuds, faite une fois à l'ouverture, donc négligeable. Ce qui coûte, ce
sont les nœuds insérés eux-mêmes, à chaque exécution : une trace sur chaque appel au modèle ajoute un nœud par appel.
Le vrai risque est la lisibilité, d'où les garde-fous.

**Par où commencer** :
1. **par nom**, avec `between` et `redirect` : c'est ce dont la relance après une réponse a besoin tout de suite ;
2. **par genre**, d'abord sur des genres à faible fréquence (`Decide`, `RunCommandNode`, `ForEach`), avec le composé
   affiché et le « n'a rien touché » ;
3. **par prédicat** sur les paramètres, en dernier, quand les deux premiers ont servi.
