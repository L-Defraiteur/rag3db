# L'exécuteur de flux — le lot 0 des agents qui se parlent

*11 octobre 2026, session recherche. La page avant le code, pour Lucie, pour la
session « everything declarative » (qui relit chaque pas et porte le côté
nœud) et pour l'orchestration. La décision de Lucie : les ports portent des
**flux** (des éléments dans le temps, et une fin ; une valeur est un flux de
un), « complexité on s'en fout, l'important c'est que ce soit au top ». La
spécification : `docs/11-octobre-2026-everything-declarative/03-les-agents-se-parlent.md`
§4.1 et §4.1 bis.*

## 1. Ce qui existe, relevé sur master (`a39ec19ec`)

- **Les valeurs** passent par une table `(nœud, port) → PortValue`, rangée à la
  **fin** du nœud (`ctx.drain_outputs`), livrée à l'aval par les arêtes (le
  dernier consommateur prend la valeur, les autres un clone ; plusieurs arêtes
  sur un port se fusionnent par `merge_port_values`). Une valeur par port :
  `set_output` deux fois, la dernière gagne.
- **L'ordonnancement** : à chaque tour, la vague des nœuds prêts, dans l'ordre
  topologique (`runtime.rs` ~1018). Une entrée est prête quand **tous** ses
  producteurs ont fini et, si elle est **requise**, qu'au moins un a émis.
- **Une branche non prise existe déjà, sans nom** : un producteur qui finit sans
  poser une sortie (VectorSearch en balayage, Rerank sans candidat, RenderResults,
  ScanFiles hors balayage, un nœud scripté, un sous-graphe). Sur une entrée
  **optionnelle**, l'aval tourne, port absent. Sur une entrée **requise**, c'est
  l'**interblocage** : `Err("deadlock: nodes […] cannot execute")`, le run entier
  meurt, **aucun rapport**, aucun statut par nœud. Aucun test n'exerce ce chemin.
- **Le rapport** : `NodeStatus::{Completed, Failed, Resumed}`, rien d'autre.
- **Les points de reprise** : par nœud fini (`save_node_completed`), ses sorties
  encore attendues sérialisées ; à la reprise, un nœud fini est restauré, les
  autres rejoués.
- **L'annulation** : il n'y en a **pas** dans l'exécuteur de graphes.

## 2. La forme, en trois pas

### Pas 1 — les déclarations, sans rien changer à l'exécution

- `PortDef` gagne `kind: PortKind::{Value, Finite, Open}` (le genre de ce
  qu'un port de sortie porte : une valeur, un flux qui finit, un flux qui peut ne
  jamais finir) et `posture: Posture::{Complete, PerElement}` (comment un port
  d'entrée lit : le tout, ou élément par élément). Défauts : `Value` et
  `Complete` — **le comportement d'aujourd'hui**. Constructeurs
  `PortDef::value(..)`, `.stream(PortKind::Finite)`, `.posture(..)` ; les
  littéraux existants reçoivent les défauts par script.
- Le genre et la posture **survivent** à `to_definition`, au mermaid et au point
  de reprise (le piège de `node_type`, fermé ensemble le 11 octobre) : témoin.
- **Témoin** : toute la lib verte, rien d'autre ne bouge.

### Pas 2 — le saut : une branche non prise est sautée et dite

Dans **l'exécuteur par niveaux d'aujourd'hui**, sans concurrence :

- une entrée **requise** dont tous les producteurs ont fini (ou ont été sautés)
  sans rien émettre ne fait plus mourir le run : le nœud est **sauté**
  (`NodeStatus::Skipped { reason }`, la raison nommant l'entrée et ses
  producteurs) ;
- le saut **se propage** : un producteur sauté compte comme fini sans donnée ;
- il est **écrit au rapport** (`NodeStatus::Skipped { reason }`, l'événement
  `NodeSkipped`) et **dans la sortie** (`DataflowOutput::skip_reason`) ;
- **Corrigé en codant** (la page disait « écrit au point de reprise, sans quoi
  une reprise le rejouerait ») : au point de reprise, il ne s'écrit pas — il se
  **recalcule à l'identique**. Un nœud n'est sauté qu'une fois tous ses
  producteurs finis ; à la reprise, ces producteurs sont restaurés avec les
  mêmes sorties (celles qui ont encore un lecteur), donc la même décision
  tombe, et « rejouer » un saut ne joue rien. L'écrire au store n'ajouterait
  qu'un état de plus à garder cohérent : si un producteur est rejoué (il avait
  échoué), la décision doit justement être reprise ;
- un graphe-outil dont le nœud de résultat est sauté rend une **erreur nommée**,
  `GraphToolError::NotTaken { node, port, reason }` (« le port de résultat n'a
  pas été produit : la branche qui y mène n'a pas été prise »), distincte de
  `NoResult` (le nœud a tourné sans poser son résultat : une faute) — demande
  de la session du proto, dont les ports d'agent lisent les deux comme « retenu » ;
- un flux vide n'est pas un échec : `Skipped` et `Failed` restent deux états.

**Le prix, dit** (relecture de la session du proto) : une entrée requise jamais
servie était une erreur **bruyante** (le run mourait) ; elle devient un saut
qu'on ne voit que si on le lit. Pour un graphe-outil, c'est dit : le résultat
sauté rend `NotTaken`. Pour un `execute()` nu, seuls l'événement `NodeSkipped`,
le rapport et `DataflowOutput::skip_reason` le disent. C'est ce qu'on veut —
une branche non prise n'est pas une faute — mais un **défaut** qui ressemble à
une branche non prise (un nœud qui oublie d'émettre une sortie que l'aval
requiert) se tait désormais de la même façon. D'où le défaut signalé au §3 :
le drapeau `required` d'une sortie n'est vérifié nulle part ; le vérifier
séparerait « non prise » (sortie optionnelle non posée) de « oubliée »
(sortie requise non posée). À décider après mesure : combien de nœuds
déclarent une sortie requise qu'ils ne posent pas toujours.

**La parité est vraie par construction** : seul change le chemin qui, aujourd'hui,
tue le run ; tout graphe qui réussit aujourd'hui ne le prend jamais.

**Témoin rouge** : un graphe à deux branches dont une n'est pas prise — sur
master, « deadlock » et pas de rapport ; après, la branche est sautée, dite au
rapport, et le run finit ; en mode point de reprise, la reprise ne la rejoue pas.

### Pas 3 — l'exécuteur concurrent, seulement quand un flux existe

- Un graphe **sans aucun port de flux** (`Finite` ou `Open`) garde l'exécuteur par
  niveaux, **inchangé** : la parité des graphes de valeurs est vraie par
  construction, pas par témoin. C'est la même forme que `run_level_auto`.
- Un graphe **avec** un port de flux est exécuté par un exécuteur qui lance ses
  nœuds **ensemble**, reliés par des canaux bornés : l'aval tourne pendant que
  l'amont émet (avec des niveaux, un canal borné s'interbloque dès qu'il est
  plein).
- `NodeContext` gagne l'émission d'un élément (`emit(port, élément)`) et la
  lecture élément par élément (`next(port)` qui rend l'élément suivant ou la
  fin) ; `set_output` reste « un flux de un ».
- **Témoin** : un flux de jetons lu élément par élément, dans l'ordre, par un nœud
  `PerElement` pendant que le producteur émet encore ; et le même graphe en
  valeurs rend le même rapport sous les deux exécuteurs.

## 3. Les cinq pièges, et ce que j'en fais

1. **Les niveaux ne tiennent pas un flux** (un canal borné plein interbloque si
   l'aval n'a pas commencé) → l'exécuteur concurrent pour les seuls graphes à
   flux (pas 3) ; les niveaux, inchangés, pour les autres.
2. **Le point de reprise d'un flux à moitié émis** → l'unité de reprise est la
   **fin** d'un flux : un producteur n'est « fini » que quand son flux l'est ; à
   la reprise, on rejoue le producteur et tout ce qui a lu son flux partiel.
   Persister les éléments avec leur rang viendra si un graphe réel le réclame.
3. **Sauté n'est pas « pas joué »** → `Skipped` au rapport et au point de reprise,
   propagé par les entrées requises ; flux vide ≠ échec (pas 2).
4. **L'annulation au milieu d'un flux** → elle **lâche les récepteurs** ; un
   producteur lit l'envoi refusé comme « arrêté », pas comme une erreur ; la
   portée du run se vide avant le join, comme aujourd'hui. L'exécuteur de graphes
   n'a aujourd'hui **aucune** annulation : elle naît au pas 3, pour les flux.
5. **Le genre d'un port survit** à `to_definition`, au mermaid et au point de
   reprise (pas 1).

Et trois défauts d'aujourd'hui que la carte a relevés, dits sans les corriger
ici : une fusion impossible sur un port devient `Trigger` **en silence**
(`runtime.rs:690`, `1119`) ; le drapeau `required` d'une **sortie** n'est vérifié
nulle part (sauf par les nœuds scriptés) — avec le saut, une sortie requise non
émise sauterait l'aval : le pas 2 le **signale au rapport** sans le refuser, et
le refuser est à décider après mesure ; un nœud qui échoue arrête le run et
tait les résultats déjà rendus du même niveau.

## 4. La frontière de fichiers

| à moi (l'exécuteur) | à la session du proto (le côté nœud) |
|---|---|
| `port.rs` : `PortKind`, `Posture`, les champs de `PortDef` et les constructeurs | la vérification commune du graphe : refuser un `Complete` branché sur un `Open` |
| `runtime.rs` : le saut, l'exécuteur concurrent, l'annulation | les premiers nœuds qui émettent, dont `LlmNode` (jetons) |
| `report.rs` : `NodeStatus::Skipped`, ce qui a coulé | `decide`, `route` (lot 2 de sa page) |
| `checkpoint.rs`, `checkpoint_store.rs` : `Skipped`, l'unité de reprise | les points d'accroche d'`agent.rs` (son lot 1) |
| `node.rs` : `emit` et `next` sur `NodeContext` | |
| `rt.rs` : le pont | |

Chacun relit l'autre avant master ; `graph.rs` (la validation) est partagé :
le genre et la posture y entrent par moi, le refus par elle.

## 5. Fait, dans l'ordre

- **Le saut** (master `522fa6780`) : une branche non prise est sautée et dite
  (`NodeStatus::Skipped`, `NodeSkipped`, `DataflowOutput::skip_reason`,
  `GraphToolError::NotTaken`), propagé au point fixe, en direct et avec points
  de reprise. Ticket fermé.
- **Les déclarations et les sorties requises** (master `feeb1932b`) :
  - `PortKind::{Value, Finite, Open}` et `Posture::{Complete, PerElement}` sur
    `PortDef`, avec les constructeurs `PortDef::value(..)`, `.stream(..)`,
    `.posture(..)`. Les 187 littéraux du dépôt sont passés au constructeur
    (défauts `Value` / `Complete` : rien ne change à l'exécution).
  - **Le prix du saut, réglé par construction** (orchestration) : le drapeau
    `required` d'une sortie, déclaré partout et vérifié nulle part, est
    RESPECTÉ. Un nœud qui finit sans poser une sortie requise est une faute
    nommée, pas un saut ; seule une sortie optionnelle vide fait sauter l'aval.
    Le saut n'est donc jamais ambigu. Mesuré : toute la lib reste verte, aucun
    nœud exercé ne déclare une sortie requise sans la poser.
  - Un **sous-graphe** (`GraphNode`) reprend le `required` des sorties libres
    de ses nœuds intérieurs : si sa sortie libre est requise à l'intérieur et
    que la branche qui y mène n'est pas prise, la faute est nommée sur le nœud
    intérieur et le sous-graphe échoue avec elle — le bon comportement : une
    sortie requise ne peut pas finir vide, même à travers un sous-graphe
    (relecture de la session du proto).
  - `RunStarted` dit la cellule de **sa requête** (le service `scope`) avant
    celle du bus, pour la marche 1 bis de l'arbre principal.
  - Témoins, rouges d'abord : `une_sortie_requise_non_posee_est_une_faute_du_noeud`,
    `run_started_dit_la_cellule_de_sa_requete`.
- **La frontière d'un sous-graphe garde le genre et la posture** (branche
  `flux-executeur`) : `GraphNode` republiait ses ports libres par
  `PortDef::value(nom, type, required)`, si bien que le genre (`Open`,
  `Finite`) et la posture s'y perdaient. Un flux ouvert enveloppé dans un
  sous-graphe échappait alors au refus `Complete ← Open` du lot 2 (relevé à
  la relecture de ce lot). Le port intérieur est désormais republié entier
  sous son nom externe. Témoin, rouge d'abord (`Value` au lieu d'`Open`) :
  `graph_node_keeps_kind_and_posture_of_free_ports`.
- **Un piège pour qui aiguille** (relevé par la session du proto au lot 2) :
  l'aval d'une branche non prise n'est sauté que si son entrée est
  **requise**. Un nœud dont l'entrée est optionnelle tourne sans valeur.
  `RhaiNode`, dont l'entrée `value` est optionnelle, posé derrière un `Route`,
  rend donc une erreur (« requires JSON value ») au lieu d'un saut. Derrière
  un `Route`, il faut un nœud à entrée requise, par exemple un nœud scripté
  qui déclare son entrée `required`.
- **Ensuite** : l'exécuteur concurrent, pour les seuls graphes à port de flux.
