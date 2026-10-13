# L'annulation sous tous les exécuteurs

*12 octobre 2026, session recherche, à la demande de l'orchestration. Ce lot
ferme le ticket ouvert pendant le lancement en fond :
`docs/tickets/2026-10-12-un-graphe-sous-les-niveaux-ne-s-annule-pas.md`. Un
run en fond ne doit plus exiger l'exécuteur concurrent pour la seule raison
que l'annulation ne traverse que lui. La forme avant le code.*

## 1. Ce qui existe (relevé sur master `ccf4c49b9`)

**Deux exécuteurs** dans `runtime.rs` :
- **par niveaux** : deux boucles, `execute_inner_with_checkpoint` (687) et
  `execute_inner` (1198). Dans un niveau, un nœud seul tourne en place ;
  plusieurs tournent sous `thread::scope`, un fil chacun (`run_level`) ; des
  nœuds asynchrones tournent l'un après l'autre (`run_level_async`) ;
- **concurrent** : un fil par nœud, les canaux bornés (`concurrent.rs`).

**L'annulation n'est armée que sous le concurrent.**
- La poignée `CancelHandle` arme les hubs qui lui sont attachés.
- Le jeton d'un nœud (`NodeContext::cancellation`) est fait du hub de sa
  liaison de flux, absente sous les niveaux.
- Sous les niveaux, le jeton n'est donc jamais annulé : `on_cancel` ne fait
  rien, `wait_timeout` dort, et la sortie rend `stopped: None` (runtime.rs
  1089, 1498).

**Ce que ça casse aujourd'hui, au-delà du ticket** :
- **Un sous-graphe de valeurs ne meurt pas avec son parent sous les
  niveaux** (`graph_node.rs:352`). Le `on_cancel` du jeton parent ne fait
  rien, et le sous-graphe, lui-même sous les niveaux, ignore la poignée.
- **Les puits du modèle** (`EmitSink`, donc `LlmNode` ; `CancellableSink`,
  donc `Decide`) ne voient pas l'annulation sous les niveaux. `emit` y jette
  l'élément et continue.
- **Un enfant lancé `WithParent` par un nœud d'un run par niveaux** n'est pas
  annulé avec lui (`background.rs:300`, `Tie::with_parent_of`).
- **`spawn` force l'exécuteur concurrent** (`background.rs`) : c'est le
  contournement nommé au ticket.
- **`RunFinished.cause`** n'est posée que par un run lancé en fond. Un run
  annulé par son hôte la laisse vide.

## 2. La forme

### 2.1 Un même jeton, quel que soit l'exécuteur

Un run par niveaux crée **un hub d'annulation** : un `StreamHub` sans canal
ni ouvrier. Il est attaché à la poignée du runtime au départ et détaché à la
fin, comme le hub d'un run concurrent. Chaque `NodeContext` le reçoit dans un
champ à lui, `cancel_hub`, et non par une fausse liaison de flux : `emit`
route par la liaison, et il ne faut pas la simuler.
- `cancellation()` lit la liaison de flux si elle existe (le concurrent),
  sinon ce hub (les niveaux).
- Le même jeton sert sous les deux exécuteurs : `is_cancelled`, `cause`,
  `wait_timeout` (qui se réveille à l'annulation) et `on_cancel` (appelé une
  fois, tout de suite si c'est déjà fait).
- Sans ouvrier inscrit, le détecteur d'interblocage n'a rien à conclure sur
  ce hub (`detect_deadlock` sort quand il n'y a pas d'ouvrier).

### 2.2 Vérifié entre deux nœuds

La boucle par niveaux regarde le hub :
- **en tête de chaque niveau**, c'est-à-dire avant de lancer ses nœuds,
  qui partent ensemble, et après le niveau précédent (§7).

Annulé, le run s'arrête :
- les nœuds non lancés ne le sont pas, et chacun reçoit l'événement
  `NodeCancelled`, comme sous le concurrent ;
- la sortie rend ce qui était fait, avec `stopped()` posé (`Some(cause)`) ;
- le run rend `Ok`. C'est la règle du concurrent : un graphe arrêté par son
  hôte n'est pas une faute.

### 2.3 Dans les attentes longues

Ce qui attend longtemps regarde le jeton :
- **Le modèle** : `EmitSink::on_token` rend `Flow::Stop` dès que le jeton est
  annulé, sous les deux exécuteurs. `LlmNode` (par `AnswerSink`) et `Decide`
  (par `CancellableSink`) s'arrêtent donc au fragment suivant, ou au réessai
  suivant.
- **Un sous-graphe** : `graph_node.rs` accroche déjà la poignée de son
  runtime intérieur au jeton du parent. Avec le jeton armé et le sous-graphe
  annulable, la chaîne parent → enfant tient sous les niveaux.
- **Un enfant lancé `WithParent`** : son lien (`on_cancel`) tient,
  puisque le jeton du parent est armé.
- **`WaitOutputNode`** (l'attente d'une ligne dans le journal d'une commande
  en fond) dort par `cancel.wait_timeout` au lieu de `thread::sleep` : il se
  réveille à l'annulation.

### 2.4 La cause dite

`RunFinished.cause` est posée pour tout run arrêté, et plus seulement pour un
run lancé en fond :
- `demande` : la poignée de l'hôte ;
- `échec de <nœud>` : un nœud a échoué sous le concurrent, et le run est
  annulé avec lui ;
- `interblocage` : le détecteur.

Un run lancé en fond garde ses causes à lui (`durée`, `parent`, `hôte`,
`demande`), posées par le registre.

### 2.5 `spawn` n'impose plus le concurrent

Un run lancé **sans flux** prend l'exécuteur que son graphe demande, les
niveaux pour un graphe de valeurs, et reste annulable. Un run lancé **avec
flux** garde le pont vivant (`execute_spawned`), qui est concurrent par
nature.

## 3. Ce qui reste non annulable, et comment il se borne

Un appel bloquant qui ne regarde pas le jeton finit ce qu'il fait. Le run
s'arrête ensuite, avant le nœud suivant.

| où | quoi | sa borne |
|---|---|---|
| `run_nodes.rs` `RunCommandNode` | une commande au premier plan | son `delai_s` (le délai de la commande) ; une commande longue se lance en fond, et `tuer_en_fond` l'arrête |
| `record_nodes.rs` `embedder.embed` | un lot d'embarquement | le délai du client du service ; le lot suivant ne part pas |
| `generic_search_nodes.rs` `reranker.rerank` | un reclassement | le délai du client |
| le client du modèle | l'attente du **premier** fragment (avant toute réponse HTTP) | le délai HTTP du client ; dès le premier fragment, le puits s'arrête |
| `react_nodes.rs` | la boucle d'agent d'un nœud React | ses bornes à elle (itérations, durée) ; elle ne regarde pas encore le jeton (lot du proto, s'il le veut) |
| les requêtes au moteur | une instruction Cypher | elle finit ; le moteur n'a pas d'interruption exposée ici |
| `reindex_nodes.rs`, `catalog_read.rs` | des attentes courtes par sommeil (2 s au plus) | leur durée |

**La règle de borne** : un run en fond garde sa durée (`max_duration`) et sa
grâce. L'annulation à l'échéance arrête tout ce qui coopère ; ce qui ne
coopère pas finit son appel, et le run s'arrête au nœud suivant. Son délai
réel est donc la durée, plus la grâce, plus le plus long appel non annulable
en cours. La page le dit, et `--qui` montre l'âge du run.

## 4. Les témoins, rouges d'abord (par exécuteur)

1. **Niveaux, en séquence** : deux nœuds enchaînés ; le premier attend sur
   `cancellation().wait_timeout`. L'hôte annule à 50 ms : le run rend `Ok`
   avec `stopped() == Some(External)`, le second nœud n'a pas tourné, et
   `RunFinished.cause` vaut `demande`. Rouge aujourd'hui : le run va au bout.
2. **Niveaux, en parallèle** : deux nœuds d'un même niveau (deux fils)
   attendent sur le jeton. Les deux s'arrêtent.
3. **Un nœud qui ne coopère pas** (un sommeil de 300 ms) suivi d'un second :
   annulé à 50 ms, le premier finit son sommeil, le second ne part pas.
4. **Un sous-graphe de valeurs** sous un parent par niveaux : l'annulation du
   parent arrête le nœud intérieur qui attend.
5. **`LlmNode` sous les niveaux** : un `CallbackLlm` lent ; annulé, il
   produit moins de fragments que sa réponse n'en a.
6. **`spawn` sans le concurrent forcé** : les témoins du lancement en fond
   restent verts. L'un d'eux vérifie qu'un graphe de valeurs lancé passe
   bien par les niveaux, et s'annule.
7. **La parité** : la lib entière. Aucun graphe de valeurs non annulé ne
   change, ni de sortie ni d'exécuteur.

## 5. La mesure, après : dix runs en fond

Le coût d'un run en fond quand dix tournent : le registre (`alive`, `qui`,
`cancel`), les fils, la mémoire résidente par run. Le but est que les bornes
par défaut (`max_alive`, `max_duration`, `grace`) aient un chiffre. C'est un
lot à part, après celui-ci.

## 6. Les questions ouvertes

1. Les noms des causes (`demande`, `échec de <nœud>`, `interblocage`) : je
   les propose ; ils sont lisibles par `--qui` et par la trace.
2. `react_nodes.rs` (la boucle d'agent dans un nœud) : regarder le jeton
   entre deux tours. C'est un changement de la boucle, donc chez le proto, et
   je ne le fais pas ici.

## 7. Les décisions (orchestration, 12 octobre)

**Les causes, en identifiants**. Chacune porte qui l'a demandée
(`RunFinished.cancelled_by`, et `cancelled_by` dans le message de fin d'un run
lancé en fond).

| cause | quand | `cancelled_by` |
|---|---|---|
| `requested` | la poignée d'arrêt du runtime ; `Registry::cancel` par le parent ou l'appelant | l'appelant, ou `host` pour la poignée |
| `parent_finished` | le parent d'un enfant `WithParent` a fini ou a été annulé | le run parent |
| `timeout` | la durée (l'entrée fermée), puis la grâce (l'annulation) | `host` |
| `failed:<nœud>` | un nœud a échoué sous le concurrent, et le run est annulé avec lui | — |
| `deadlock` | le détecteur d'interblocage | — |
| `host_stopped` | `Registry::cancel_all`, l'hôte qui s'arrête | `host` |

Les causes en français du lot A (`demande`, `durée`, `parent`, `hôte`) sont
remplacées par celles-ci ; les constantes sont dans
`dataflow::background::cause`.

**`react_nodes.rs` est au proto** : la boucle d'agent dans un nœud ne regarde
pas encore le jeton. Le geste est donné : `ctx.cancellation().is_cancelled()`
entre deux tours, ou `on_cancel` accroché à l'arrêt de l'agent. **C'est un
reste de cette page jusqu'à son lot.**

**Où la boucle regarde** : en tête de chaque niveau. Les nœuds d'un niveau
partent ensemble, et cette tête est aussi « avant de lancer chacun de ses
nœuds » et « après le niveau précédent ». Un run annulé n'a pas tout fini, et
ce n'est pas une faute : le contrôle « max iterations exceeded » l'excepte.

**Les quatre défauts du relevé** (§1) ont chacun un ticket, ouvert et fermé
dans le même commit que l'original (`docs/tickets/closed/`).
