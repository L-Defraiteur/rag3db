# Lancer un graphe en fond : la part de l'exécuteur

*12 octobre 2026, session recherche, à la demande de l'orchestration. La demande
de Lucie, du 11 octobre au matin : « un agent est un DAG, un DAG est
potentiellement un outil ; pas forcément attendre son retour : le lancer en
fond avec un message initial et la référence de qui je suis, qu'il puisse me
parler ». La forme d'ensemble est déjà posée par le proto, dans
`../11-octobre-2026-everything-declarative/03-les-agents-se-parlent.md` :
§2.2 les deux modes, §2.3 le verbe `spawn`, §3.3 la vie indépendante, §4.5 les
verbes posés comme nœuds, et le lot 5. Cette page ne la redéfinit pas. Elle dit
ce que l'**exécuteur** doit tenir pour qu'elle soit vraie, et partage le
travail avec le proto, qui tient `AgentNode`, `Decide`, `send` et l'annuaire.*

## 1. Ce qui existe, et pourquoi ça ne suffit pas

Relevé sur master `4960f7b24`.

- **Un outil en fond existe déjà** : `%% async: true`, `spawn_async_tool`
  (`agent.rs:1184`). Il répond tout de suite par une poignée
  (`{"handle","statut":"en cours"}`), puis il écrit le résultat dans la
  boîte de l'appelant, signé `outil #h` (`ASYNC_RESULT_FROM`, que
  `read_inbox` sait lire). **Mais il tourne sur un fil du `thread::scope` du
  run, joint à la fin du run** : il ne peut pas survivre à son appelant. Or
  l'appelant d'un chat ne vit que le temps d'un tour.
- **La commande en fond** (chantier A, `RunCommandNode::lancer_en_fond`,
  `run_nodes.rs:350`) survit au tour, garde un registre propre au processus
  (`suivies()`, par poignée), et dit sa fin dans la boîte. **Mais c'est un
  processus, pas un graphe**, et sa mise à mort est accrochée au
  `RunScope` de l'appelant.
- **Un sous-graphe meurt toujours avec son parent** : `graph_node.rs:352`
  (`ctx.cancellation().on_cancel(move || stop.cancel())`), et le chemin
  délégué partage le hub du parent.
- **L'annulation n'agit que sous l'exécuteur concurrent** : sous les
  niveaux, `CancelToken` ne s'arme jamais (`node.rs:552`). Un graphe de
  valeurs lancé en fond ne pourrait donc pas être arrêté.
- **Le bus dit `RunStarted{parent}` et `RunFinished{ok}`**, rien de plus :
  ni l'annulation ni sa cause.
- **Aucun registre des runs vivants** : seuls `suivies()` (les commandes) et
  l'annuaire (nom → run courant) existent.

## 2. La forme

### 2.1 Une primitive de l'exécuteur : `DataflowRuntime::spawn`

```rust
pub fn spawn(self, graph: DataflowGraph, spec: Spawn) -> Result<Launched, String>
```

```rust
pub struct Spawn {
    pub tool: String,              // ce qui est lancé, pour la trace
    pub message: Option<PortValue>, // posé sur l'entrée déclarée (`message`, sinon la première libre)
    pub caller: Option<Recipient>, // l'adresse de l'appelant dans l'annuaire, où la fin est écrite
    pub parent_run: Option<String>,
    pub handle: String,            // « #nom-n », la poignée que l'appelant a reçue
    pub tie: Tie,                  // Detached (défaut) | WithParent
    pub limits: Limits,            // durée, nombre à la fois par outil (§2.5)
}
pub struct Launched { pub run: String, pub handle: String }
```

- **Un fil à lui**, hors de tout `thread::scope`, avec son propre
  `DataflowRuntime`. Les services de l'appelant sont superposés, avec
  `parent_run`, `tool_handle` et l'adresse de l'appelant. Son identifiant
  vient de `new_run_id(tool)`. `RunStarted{parent}` est émis **avant** que
  `spawn` rende, pour qu'un lecteur du bus ne voie jamais une fin sans début.
- **Rendu tout de suite** : la référence du run et la poignée. Rien n'attend
  le premier nœud.
- **L'exécuteur concurrent, toujours**, pour un run lancé en fond : c'est le
  seul sous lequel l'annulation atteint chaque nœud (§1). La parité des
  graphes de valeurs entre les deux exécuteurs est prouvée depuis le lot 0,
  donc rien ne change pour ce que le graphe calcule.

  *Variante, à choisir* : armer l'annulation **aussi** sous les niveaux,
  vérifiée entre deux nœuds. C'est plus général, puisque tout run deviendrait
  arrêtable, mais ça touche le chemin de tous les graphes. Je la propose comme
  lot à part, et je ne la fais pas dans celui-ci.

### 2.2 La sortie revient par le bus, en message adressé

À la fin, l'exécuteur écrit **un seul message** dans la boîte de l'appelant,
par `bus.send_message(run, "outil #h", caller.run, contenu)`. Ce sont le
canal et la signature des outils en fond d'aujourd'hui : la boucle d'agent les
lit déjà comme le résultat d'un appel, sans rien de neuf chez elle. Le
contenu :

```json
{"handle": "#resume-2", "run": "resume-…", "statut": "terminé" | "échoué" | "annulé",
 "résultat": <la sortie déclarée>, "erreur"?: "…", "cause"?: "durée" | "parent" | "hôte" | "demande"}
```

Ce qui est rendu en `résultat` :
- la sortie que l'outil déclare (`answer` pour un agent, le port de sortie
  libre d'un graphe-outil) ;
- sinon, ce que `GraphTool` rend en appel synchrone pour le même outil
  (`call_with_policy`, `graph_tool.rs:1691`), pour qu'un outil rende la même
  chose attendu ou lancé.

Aucun port n'attend cette sortie : l'appelant a peut-être fini son tour.

**La fin est dite sur le bus aussi**, pour la trace et pour `--qui` :
`RunFinished{run, kind, ms, ok}` reçoit un champ `cause: Option<String>`.
Il vaut `None` pour une fin ordinaire ; pour une annulation, `"durée"`,
`"parent"`, `"hôte"` ou `"demande"`. Son JSON (`events.rs:365`) l'écrit
quand il est posé. C'est un champ de plus, pas un événement de plus. Les
trois sites qui construisent l'événement aujourd'hui (`agent.rs:1361`,
`runtime.rs:1179`, le test `events.rs:739`) reçoivent `cause: None`, et le
motif de `events.rs:365` le lit. Les lecteurs du JSON ne changent pas.

### 2.3 Vivre sans son parent, sauf déclaration

- **`Tie::Detached`, le défaut** (le choix 3 de la page 03) : rien n'est
  accroché au `RunScope` ni à l'annulation de l'appelant. Le parent qui finit
  ou qui est annulé ne touche pas l'enfant. Si l'appelant n'existe plus
  quand l'enfant finit, sa fin attend dans la boîte (le curseur nommé du
  bus) et dans la trace ; rien ne se perd en silence.
- **`Tie::WithParent`** : l'annulation de l'enfant est accrochée à celle du
  parent (`on_cancel`) **et** à la fin de son run (`RunScope::defer`). Cause
  dite : `"parent"`.
- **L'hôte qui s'arrête** annule tous les runs vivants du registre, avec la
  cause `"hôte"`, avant de rendre la main : un arrêt propre, pas des fils
  coupés en vol (la règle du WAL).

### 2.4 Le registre des runs vivants, et `--qui`

Un registre propre au processus, posé comme service, avec un défaut global
comme `suivies()`. Par run, il garde :
- le parent ;
- l'appelant (son nom dans l'annuaire) ;
- l'outil et la poignée ;
- le message initial, ses 120 premiers caractères ;
- le départ ;
- l'état (en cours, ou en arrêt avec sa cause) ;
- la poignée d'annulation et le fil.

Il offre :
- **`alive()`** : la liste, triée par départ. C'est le `--qui` du graphe :
  `run, parent, appelant, outil, âge, état, message`. Je l'expose en API et
  en ligne de journal. Le démon et le serveur MCP l'afficheront chez
  l'arbre principal (§4) ;
- **`cancel(run, cause)`** : la seule façon d'arrêter un run par sa
  référence ;
- **le retrait à la fin**, après le message de fin. Un run absent du
  registre est donc fini, et sa fin est écrite.

*Fondre `suivies()` dedans ?* Une commande en fond et un graphe en fond sont
la même chose vue de l'appelant : une poignée, une fin dans la boîte, une
mise à mort par référence. Le risque est d'avoir deux registres et deux
`--qui`. Je propose de les fondre, en gardant un `kind` (commande ou
graphe). Mais `run_nodes.rs` est au chantier A : il faut d'abord l'accord de
son porteur, sinon le lot pose le registre seul et la fonte vient après.

### 2.5 Les bornes que l'exécuteur tient (§2.6 de la page 03)

- **La durée** : un minuteur annule à l'échéance, avec la cause `"durée"`.
- **Combien à la fois, par outil** : un lancement de trop est **refusé et
  dit** à l'appelant, sans être mis en file. La file serait un choix de
  plus ; elle est ouverte en §6.
- **Qui peut lancer** et **le nombre de réveils** relèvent du manifeste et
  de la boucle d'agent : c'est chez le proto.

### 2.6 Le nœud, en plus du verbe (§4.5 de la page 03)

`SpawnNode`, un nœud qu'on pose sur un port :
- en entrée, `message` (`PerElement`) : un lancement par élément ;
- en configuration, l'outil (un graphe-outil du registre), les arguments,
  `tie` et les bornes ;
- en sortie, `run` : un flux, une référence par lancement.

Il appelle la même primitive que le verbe. Le déclaré et le choisi forment
un seul mécanisme.

### 2.7 Rester à l'écoute (§2.4 de la page 03) : ce que l'exécuteur donne

« Il vit tant qu'un de ses flux d'entrée est ouvert. » Côté exécuteur, il
faut **une boîte qui devient un flux ouvert** : un nœud source qui lit le
curseur `run.<id>.inbox` et émet chaque message sur une sortie `Open`. Il se
ferme sur une pause `leave` ou `finished`, à la durée, ou à l'arrêt de
l'hôte. Le run vit donc tant que ce nœud émet, et c'est l'exécuteur
concurrent qui le tient, sans boucle à part. **C'est le second temps du
lot** : le premier lance et rend une fin ; le second garde un run lancé à
l'écoute. L'`AgentNode` du proto s'y branche par `peer_in`.

## 3. Le partage

| chez le proto | chez moi |
|---|---|
| le verbe `spawn(tool, arguments, message)` dans la boucle d'agent : l'annonce, l'interception, la poignée rendue au modèle | `DataflowRuntime::spawn`, `Spawn`, `Tie`, `Launched` |
| `AgentNode` et son parent (lot 4), `peer_in` branché sur la boîte | le registre des runs vivants, `alive()` (`--qui`), `cancel` |
| qui peut lancer, le nombre de réveils (manifeste) | la durée, le nombre à la fois, `RunFinished.cause` |
| le message rendu au modèle (sa forme de tour) | `SpawnNode`, la boîte en flux ouvert (second temps) |
| | les témoins de l'exécuteur, le témoin sur modèle local, la mesure |

Le verbe du proto appelle ma primitive. La boucle d'agent passe par ma
relecture, comme `src/dataflow/` passe par la sienne.

## 4. Ce qui touche les autres

- **L'arbre principal** : afficher `alive()` dans `daemon_status` et dans
  l'outil MCP, et annuler les runs vivants à l'arrêt du démon (§2.3). Je
  propose l'interface, je ne touche pas à son fil.
- **Le chantier A**, si la fonte de `suivies()` est retenue (§2.4).

## 5. Les témoins, rouges d'abord (faux modèle sauf le dernier)

1. **Rendu tout de suite** : `spawn` rend sa référence pendant que le premier
   nœud de l'enfant est retenu par une poignée de main. Le test ne relâche
   l'enfant qu'après avoir reçu la référence.
2. **La fin dans la boîte** : le message signé `outil #h` arrive dans
   `run.<appelant>.inbox`, avec `statut: terminé` et le résultat attendu.
   `RunStarted{parent}` précède `RunFinished`.
3. **Il survit à son parent** : le run parent finit, et même est annulé ;
   l'enfant, encore retenu, finit ensuite et écrit sa fin.
4. **`WithParent`** : le parent annulé annule l'enfant, avec `statut: annulé`
   et `cause: parent`.
5. **Annulé par sa référence** : `cancel(run, "demande")` arrête un enfant
   dont un nœud attend sans fin. La fin est dite, et le fil se termine.
6. **`--qui`** : pendant la vie de l'enfant, `alive()` le liste avec son
   parent, son appelant et son message ; après sa fin, il ne le liste plus.
7. **Les bornes** : la durée annule (`cause: durée`) ; un lancement au-delà
   du nombre permis est refusé et dit.
8. **Le vrai modèle** (protocole) : qwen3-1.7b sous `llama-server`, le
   harnais de `jetons_tests.rs`. Un graphe-outil à `LlmNode` (« résume ce
   texte ») est lancé en fond ; la fin arrive dans la boîte de l'appelant
   avec le résumé. Quand le verbe du proto sera là, un agent lancera
   lui-même cet outil et lira la fin à son tour suivant.

## 6. La mesure

- **Le coût d'un lancement** : de l'appel à `spawn` jusqu'à la référence
  rendue, puis jusqu'au `RunStarted`, puis jusqu'à l'exécution du premier
  nœud. Sur 1 000 lancements d'un graphe d'un nœud, j'en donne la médiane et
  le 99e centile, et les fils et la mémoire par run vivant.
- **Le temps d'aller-retour** : de la fin du dernier nœud de l'enfant
  jusqu'au message lu dans la boîte de l'appelant.

## 7. Les questions ouvertes

- L'exécuteur concurrent pour tout run en fond, ou l'annulation armée sous
  les niveaux (§2.1) ? Je recommande le concurrent dans ce lot ; la
  variante peut faire un lot à part.
- Fondre `suivies()` dans le registre (§2.4) ? Je le recommande, avec
  l'accord du chantier A.
- Au-delà du nombre permis : refuser (ce lot) ou mettre en file ?
- La boîte d'un appelant disparu : jusqu'où garder la fin ? Le curseur du
  bus est en mémoire ; la boîte durable entre processus reste hors chantier
  (page 03).
