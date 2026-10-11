# L'exécuteur concurrent — le pas 3 du lot 0

*11 octobre 2026, session recherche. La page avant le code, à relire par la
session du proto (le côté nœud) et par l'orchestration. Elle suit la page 04
(§2, pas 3) et la décision de Lucie : les ports portent des flux, et la
complexité n'est pas un argument. Elle part de ce qui existe sur master
(`3d2fb70b2` et la frontière des sous-graphes de `flux-executeur`).*

## 1. Ce qui existe, et ce qui en découle

- **Un nœud est synchrone** : `Node::execute(&mut self, &mut NodeContext)`
  (`node.rs:75`). `execute_async` existe, mais aucun nœud de production ne
  passe `is_async` à vrai. Un nœud qui reçoit un flux le lira donc en
  **bloquant** son fil, pas en `await`.
- **Un niveau s'exécute dans des fils de portée**, un fil par nœud
  (`run_level`, `runtime.rs:195-232`). Les sorties sont rangées à la **fin** du
  nœud (`drain_outputs`) puis livrées à l'aval. Rien ne coule pendant qu'un
  nœud tourne.
- **Aucune annulation** dans l'exécuteur : pas de jeton dans `Node`,
  `NodeContext` ni `DataflowRuntime`. Le seul point d'arrêt d'un modèle est le
  `Flow::Stop` que rend son puits de jetons (`TokenSink`, `llm.rs:220`).
- **Le flux de jetons existe sous le graphe sans le traverser** : `LlmNode`
  remplit un `StringSink` et ne publie le texte qu'à la fin
  (`llm_nodes.rs:111-135`). `ChannelSink` existe, employé seulement en test.
- **Un sous-graphe exécute un runtime imbriqué jusqu'au bout** puis recopie ses
  sorties (`graph_node.rs:180-212`).
- **Le point de reprise** a pour unité le nœud fini (`save_node_completed`,
  `runtime.rs:904`), avec ses sorties encore attendues.

D'où la forme : un fil par nœud, des canaux bornés entre eux, une lecture
bloquante, et une annulation qui naît ici.

## 2. Quand l'exécuteur concurrent prend la main

**Seulement si un port de sortie du graphe est `Finite` ou `Open`.** Un graphe
de valeurs garde l'exécuteur par niveaux, inchangé : la parité y est vraie par
construction. Le choix se fait une fois, au début du run, par un parcours des
`PortDef` (le sous-graphe republie désormais le genre de ses ports, §8). Un
drapeau de test force le concurrent sur un graphe de valeurs, pour le témoin de
parité.

## 3. La forme

### 3.1 Un fil par nœud, tous lancés ensemble

Dans une `std::thread::scope`, chaque nœud reçoit son fil et son emprunt
`&mut` (les nœuds ne quittent pas `graph.nodes`, comme dans `run_level`). Un
nœud `is_async` passe par `rt::bloquer` dans son fil. Le fil d'un nœud :

1. **attend ses entrées de départ** (§3.3), ou décide le saut ;
2. appelle `execute` avec un `NodeContext` relié aux canaux ;
3. à la fin, envoie ses sorties de valeur, ferme ses flux, vérifie ses sorties
   requises, et publie `NodeRun` puis `NodeCompleted` comme aujourd'hui.

### 3.2 Les canaux : maison, pas `std::sync::mpsc`

Une arête porte un canal **borné** (64 éléments par défaut, réglable par le
service `stream_capacity`). Le canal est écrit sur `Mutex` + `Condvar`, pour
trois raisons qu'un `mpsc` ne donne pas :

- **La fin et l'abandon sont deux états** : un flux fini (`End`) n'est pas un
  flux interrompu parce que son producteur a échoué (`Aborted`). Avec `mpsc`,
  les deux se lisent comme une déconnexion, et un aval prendrait un flux tronqué
  pour un flux complet.
- **L'annulation réveille tout le monde** : un fil bloqué en lecture ou en
  écriture se réveille dès que le run est annulé.
- **L'interblocage se détecte** (§3.6) : chaque attente s'inscrit dans un
  registre commun.

Un port de sortie branché sur plusieurs arêtes **clone** l'élément pour chaque
consommateur sauf le dernier, comme les valeurs aujourd'hui. Une entrée branchée
sur plusieurs arêtes (fan-in) reçoit les flux entrelacés ; l'ordre de chaque
producteur est gardé, et l'entrée finit quand tous ont fini.

### 3.3 La posture d'une entrée

- **`Complete`** : le runtime lit le flux **jusqu'à sa fin** et le remet au
  nœud fusionné par `merge_port_values`, comme un fan-in d'aujourd'hui. Un
  flux d'un élément rend l'élément lui-même, d'où la parité avec les valeurs.
  La lecture est faite par le fil du nœud avant `execute`, dans un tampon
  **non borné** : un lecteur `Complete` ne fait jamais refluer son producteur.
  Le refus `Complete ← Open` du lot 2 garantit que ce flux finit.
- **`PerElement`** : le nœud lit lui-même, par `ctx.next(port)`. Avant de
  lancer `execute`, le runtime attend le **premier** élément de chaque entrée
  `PerElement` **requise**, ou sa fin. Cet élément est gardé et rendu par le
  premier `next`.

**Le saut, même règle qu'au pas 2** : une entrée requise dont le flux finit sans
aucun élément fait sauter le nœud (`Skipped`, la raison nomme l'entrée et ses
producteurs). Un nœud sauté ferme ses sorties sans rien émettre, et le saut se
propage par les canaux, sans point fixe à calculer.

### 3.4 Ce que gagne `NodeContext`

- `emit(port, valeur) -> Result<(), Stopped>` pose un élément sur un port de
  flux. `Err(Stopped)` veut dire que **tous** les lecteurs ont lâché (ils ont
  fini avant la fin du flux) ou que le run est annulé. Le producteur doit
  s'arrêter ; ce n'est pas une faute.
- `next(port) -> Result<Option<PortValue>, Cancelled>` rend l'élément suivant,
  `None` à la fin du flux, et `Err(Cancelled)` si le run est annulé ou si le
  producteur a avorté. Le `?` du nœud propage l'annulation.
- `next_any() -> Result<Option<(String, PortValue)>, Cancelled>` rend le
  prochain élément de **n'importe quelle** entrée `PerElement`. C'est la
  lecture sûre d'un nœud qui a plusieurs entrées en flux (§3.6).
- `close_input(port)` dit qu'on n'en lira pas plus : le producteur voit
  `Stopped` dès que plus personne ne lit.
- `is_cancelled()` est le jeton, pour un nœud qui calcule longtemps sans lire ni
  émettre.
- `set_output` reste « un flux de un » : sur un port `Value`, la valeur part à
  la fin du nœud, et c'est la dernière qui gagne, comme aujourd'hui. Sur un port
  de flux, `set_output` est refusé, nommément : on émet.

Un adaptateur `ctx.token_sink(port)` donne un `TokenSink` qui émet chaque jeton
et rend `Flow::Stop` sur `Stopped` ou sur annulation. C'est le branchement de
`LlmNode` (côté proto) et le premier point où l'annulation atteint un modèle.

### 3.5 Échec et annulation

- **Un nœud échoue** : il est `Failed`. Le run passe à « annulé », ses flux
  ouverts passent à `Aborted`, toutes les attentes se réveillent. Les autres
  nœuds en cours finissent par `Cancelled` (nouveau statut au rapport, distinct
  de `Failed` : seule la cause est une faute). Le run rend l'erreur du premier
  nœud en échec, comme aujourd'hui, et le rapport dit tout le monde.
- **Un lecteur qui lâche** (`close_input`, ou un nœud qui finit sans tout lire)
  n'annule rien : son producteur voit `Stopped` quand **plus aucun** lecteur ne
  lit, et finit normalement. Ce qu'il a émis jusque-là compte.
- **Le join** attend tous les fils. Un fil qui ne regarde jamais le jeton
  retarde la fin du run d'autant : c'est dit au rapport par la durée, pas
  caché.

### 3.6 L'interblocage, détecté et nommé

Des canaux bornés peuvent s'interbloquer. Le cas type est un losange
`A → B → D` et `A → C → D`, où D lit B jusqu'au bout avant C : C remplit son
canal, A se bloque sur C, B n'avance plus. Deux réponses :

- **`next_any`** est la lecture qui ne s'interbloque pas, et la page des nœuds
  la recommande dès qu'il y a deux entrées en flux ;
- **un détecteur** : chaque attente (lecture, écriture, attente de départ)
  s'inscrit avec ce qu'elle attend. Quand **tous** les fils vivants attendent,
  le run est annulé avec une erreur qui nomme le cycle (« D attend B.out, B
  attend A.out, A attend l'écriture sur C.in, plein »). Pas de délai arbitraire :
  la condition est exacte, puisque tout passe par nos canaux.

### 3.7 Le point de reprise

L'unité reste **le nœud fini**. Un nœud n'est fini que quand tous ses flux le
sont ; à ce moment, ce qu'il a émis est **fini** aussi, même sur un port `Open`
(un flux ouvert qui s'est fermé). `save_node_completed` reçoit, pour un port de
flux, la **liste ordonnée** de ses éléments encore attendus : `CheckpointPortValue`
gagne `elements: Option<Vec<CheckpointPortValue>>`. Un ancien point de reprise
se relit sans changement, puisque le champ est absent.

À la reprise, un nœud fini est restauré en **rejouant sa liste** dans ses
canaux. Les autres sont rejoués, y compris tout lecteur d'un flux partiel. La
page 04 le disait déjà (§3, piège 2), et persister les éléments n'est que la
façon de ne pas rejouer le producteur.

### 3.8 Le rapport et les événements

- `NodeStatus::Cancelled` s'ajoute ; `Skipped` et `Failed` gardent leur sens.
- Ce qui a coulé : par port de flux, le **nombre d'éléments** émis et lus, et
  qui a lâché. C'est ce que la page 04 annonçait sous « ce qui a coulé ».
- Les `taps` (observe) voient chaque élément.
- Les événements restent les mêmes, dans le même ordre pour un nœud donné ;
  entre deux nœuds, l'ordre suit les fils et n'est plus topologique. Un lecteur
  du bus ne doit pas compter sur l'ordre entre nœuds d'un graphe à flux.

## 4. Les sous-graphes

`GraphNode` exécute un runtime imbriqué jusqu'au bout. En deux temps :

- **3a, dans ce lot** : un sous-graphe dont un port libre est `Finite` le rend
  **à la fin**, élément par élément, dans l'ordre. C'est juste, mais pas vivant.
  Un port libre `Open` est **refusé au montage** (le genre traverse
  désormais la frontière), parce qu'il ne finirait jamais.
- **3b, à part** : le pont vivant, où le runtime imbriqué reçoit les émetteurs
  du parent. Il lève le refus de l'`Open`.

## 5. Ce qui ne change pas

- Un graphe de valeurs ne passe jamais par ici.
- Les nœuds d'aujourd'hui tournent sans modification sous le concurrent : leurs
  ports sont `Value`, `set_output` y garde son sens.
- `execute`, `execute_with_report`, `execute_with_checkpoint_mode` gardent leurs
  signatures ; le choix de l'exécuteur est interne.

## 6. Les témoins, rouges d'abord

1. **Ça coule** : un producteur émet trois éléments et ne rend le troisième
   qu'après que l'aval `PerElement` a lu le premier (une barrière). Il passe
   seulement si l'aval lit pendant que l'amont émet. Sous les niveaux, c'est un
   interblocage.
2. **La parité** : un graphe de valeurs forcé sous le concurrent rend les mêmes
   sorties et les mêmes statuts que sous les niveaux.
3. **`Complete` lit le tout** : un lecteur `Complete` d'un flux `Finite` de trois
   éléments reçoit leur fusion.
4. **Le saut par un flux vide** : un flux `Finite` vide sur une entrée requise
   donne `Skipped`, avec la raison, et l'aval est sauté à son tour.
5. **La sortie requise** : un port de flux requis qui finit vide est une faute
   nommée du nœud.
6. **L'échec annule** : un producteur `Open` sans fin et un lecteur qui échoue au
   deuxième élément. Le run finit (garde de temps dans le test), le lecteur est
   `Failed`, le producteur `Cancelled`.
7. **Un lecteur qui lâche** : l'aval lit deux éléments puis `close_input`. Le
   producteur voit `Stopped` et finit `Completed`.
8. **L'interblocage nommé** : le losange lu port par port rend l'erreur qui
   nomme le cycle ; lu par `next_any`, il finit.
9. **La reprise** : une panne injectée (`fail_node`) après la fin du producteur ;
   à la reprise, le producteur n'est pas rejoué et l'aval reçoit les mêmes
   éléments, dans l'ordre.
10. **Le sous-graphe** : un `Finite` traverse la frontière dans l'ordre ; un
    `Open` libre est refusé au montage en le nommant.
11. **Le puits de jetons** : `ctx.token_sink(port)` sous un `CallbackLlm` qui
    émet cinq jetons ; un lecteur qui lâche au deuxième fait rendre `Flow::Stop`
    au puits.

## 7. Le découpage en commits

1. `NodeContext` (emit, next, next_any, close_input, is_cancelled,
   token_sink), le canal maison et son détecteur, avec des témoins unitaires
   sur le canal (fin, abandon, réveil, détecteur).
2. L'exécuteur concurrent et son choix, avec les témoins 1 à 8.
3. Le point de reprise des flux, avec le témoin 9.
4. Le sous-graphe 3a, avec le témoin 10.
5. Le témoin 11 vit chez moi ; `LlmNode` qui émet vient chez le proto.

Chaque commit passe toute la lib et `scripts/check_features.sh`, et la batterie
complète passe une fois avant la fusion.

## 8. Les questions ouvertes

- **La capacité par défaut** (64) est un choix sans mesure. La première
  mesure viendra avec `LlmNode` qui émet.
- **Un fil par nœud** coûte un fil système par nœud du graphe, ce qui est bien
  pour les graphes d'agent (dizaines de nœuds). Pour un graphe de centaines de
  nœuds en flux, un ordonnanceur à pool serait la suite. Rien ne l'exige
  aujourd'hui.
