# La boîte en flux ouvert : un run lancé vit tant que son flux d'entrée est ouvert

*12 octobre 2026, session recherche, à la demande de l'orchestration. C'est le
second temps de `03-lancer-en-fond.md` (§2.7). Un agent lancé en fond reçoit
un **flux** de messages de son appelant, et sa sortie est elle-même un flux
que l'appelant lit. Il finit quand son flux d'entrée se ferme, ou quand on
l'annule. La forme avant le code, à relire par l'orchestration et par le
proto (`AgentNode`, `Decide`, la boucle d'agent).*

## 0. La règle : ce qui ne doit rien perdre passe par un port

La mesure du refoulement (page 01, §8) l'a dit en chiffres :
- **un port refoule** : un lecteur lent retient le producteur, le canal de
  64 borne son avance (65 fragments mesurés) ;
- **le bus perd** : il écrase les plus anciens. Un abonné à 1 ms par
  événement a perdu 1 932 jetons sur 2 000.

D'où la règle de cette page, et désormais de tout le chantier :

> **Ce qui ne doit rien perdre passe par un port, jamais par le bus.** Le bus
> sert à observer (la trace, un écran, `--qui`). Il peut perdre, et il le dit
> (`Overflowed(n)`). Une conversation entre un appelant et ce qu'il a lancé
> est une donnée : elle passe par des canaux.

**Ce que la règle touche aujourd'hui** (relevé sur master `c5cd38152`) :
- **Tout ce qui va d'un run à un autre passe par le bus** : la fin d'un run
  lancé (`deliver`), les résultats des outils en fond, les messages entre
  pairs (`send`). Une boîte de 64 qui déborde perd des messages.
- **La boucle d'agent saute ces pertes en silence** : `agent.rs:1287`,
  `Err(Overflowed(_)) => continue`. Un correctif qui le dit existe
  (« une boîte qui déborde le dit », `6858a9ebb`, 12 octobre, 14 h 12). Il
  est en cours sur la branche `boite-pleine`, extraite dans le worktree
  d'une autre session, et n'est pas encore sur master.
- La fin d'un run lancé reste sur le bus, où elle est petite et unique. Une
  conversation passe par des canaux (§2).

## 1. Ce qui existe, et ce qui manque

- **`DataflowRuntime::spawn`** (lot A) lance un graphe sur son propre fil,
  sous l'exécuteur concurrent. Le message initial y est **une valeur**
  (`set_initial_input`), et `Launched` ne rend que `run` et `handle`.
- **Le pont vivant des sous-graphes** (`Outer`, `execute_delegated`,
  `concurrent.rs:124`) branche les nœuds d'un graphe sur des canaux venus
  d'ailleurs. C'est exactement le mécanisme qu'il faut. Mais il partage le
  hub du parent : l'enfant meurt avec lui, et le parent le pilote depuis un
  de ses ouvriers.
- **Le détecteur d'interblocage** (`stream.rs:288`) déclare l'interblocage
  quand tous les ouvriers inscrits attendent. Un nourrisseur extérieur au
  run, qui n'est pas un ouvrier inscrit, n'y compte pas. Un lecteur
  intérieur qui attend l'appelant serait donc pris pour un interblocage.
- **Aucun nœud de production n'émet un flux `Open`** : on n'en trouve que
  dans les tests. `EventSourceNode` vide un curseur en **une** valeur.
- **`AgentNode` n'existe pas encore** (le lot 4 du proto). Les ports de sa
  page (§4.2 de la page 03) sont aujourd'hui des points d'accroche de la
  boucle (`agent_ports.rs`), pas des ports de graphe.

## 2. La forme

### 2.1 Un run lancé avec des flux : `Spawn::streams`

```rust
pub struct Streams {
    /// L'entrée que l'appelant nourrit : le port libre `(nœud, port)`, lu
    /// élément par élément (`PerElement`). Son genre est `Open` : il ne finit
    /// que quand l'appelant le ferme.
    pub input: Option<(String, String)>,
    /// La sortie que l'appelant lit : un port de flux du graphe.
    pub output: Option<(String, String)>,
}
pub struct Spawn { /* … */ pub streams: Streams }

pub struct Launched { pub run: String, pub handle: String, pub live: Option<Live> }
pub struct Live { pub input: Option<LiveIn>, pub output: Option<LiveOut> }

impl LiveIn {
    /// Bloque tant que le canal est plein : le refoulement traverse la
    /// frontière. `Err(Stopped)` : le run a fini ou est annulé.
    pub fn send(&self, v: PortValue) -> Result<(), Stopped>;
    /// Ferme le flux : l'entrée du run voit sa fin, il finit.
    pub fn close(self);
}
impl Drop for LiveIn { /* fermer : un appelant qui meurt ferme le flux */ }

impl LiveOut {
    /// `Ok(Some(v))` : un élément ; `Ok(None)` : le run a fini son flux ;
    /// `Err(cause)` : il a été annulé, et voici pourquoi.
    pub fn recv(&self) -> Result<Option<PortValue>, CancelCause>;
    pub fn recv_timeout(&self, d: Duration) -> Result<Option<PortValue>, RecvTimeout>;
}
```

- **Le mécanisme est celui du pont des sous-graphes**, avec un hub à lui.
  `spawn` crée le hub du run lancé, ses canaux d'entrée et de sortie, et un
  **ouvrier extérieur** inscrit pour l'appelant. Il passe le tout à
  `execute_concurrent` comme un `Outer`. `LiveIn` et `LiveOut` tiennent cet
  ouvrier.
- **Le détecteur d'interblocage compte l'ouvrier extérieur** : il n'attend
  que pendant un `send` ou un `recv` bloqués sur ses poignées. Entre deux
  envois, un appelant qui pense ou qui dort n'est pas en attente : l'enfant
  qui l'attend n'est pas en interblocage. Un appelant bloqué en `recv` sur
  un enfant bloqué sur lui, en revanche, est un vrai interblocage, et il est
  dit.
- **L'annulation du registre** (`cancel`) arme ce hub, comme pour un run
  lancé sans flux. Elle parvient aux deux poignées : `send` rend
  `Err(Stopped::Cancelled)`, et `recv` rend `Err(cause)`.
- **La vie** : le run vit tant que son entrée est ouverte. `close`, ou la
  chute de `LiveIn`, ferme l'entrée ; les nœuds finissent ; le run finit.
  C'est le « vit tant qu'un flux d'entrée est ouvert » de la page 03, rendu
  mécanique par l'exécuteur, sans boucle à part.
- **La fin reste dite deux fois** : `RunFinished` (avec sa cause) sur le
  bus, puis le message de fin à l'appelant, et `LiveOut` rend `None`. Le
  message de fin peut se perdre sur le bus ; la fin du flux, non.
- **`max_duration` ferme l'entrée** au lieu d'annuler. Cause `durée`, dite
  aussi sur le flux de sortie.

### 2.2 Dans un graphe : `SpawnNode` en mode flux

`SpawnNode` reçoit un réglage `stream` (faux par défaut, pour garder le
comportement d'aujourd'hui).
- **Faux** : un lancement par message, comme aujourd'hui.
- **Vrai** : **un seul** lancement au premier message. Chaque message
  suivant part sur le `LiveIn` de ce run ; la fin de son entrée `message`
  ferme le `LiveIn`. Une sortie neuve, `replies`, porte le `LiveOut`, et
  `run` porte la référence.

Le graphe appelant et le run lancé se parlent donc par deux canaux bornés,
de bout en bout.

### 2.3 Dans la boucle d'agent (chez le proto)

Le verbe `spawn` du proto peut demander les flux :
- **la boucle garde le `LiveIn`** de l'enfant par sa poignée ;
- **`send(#h, …)` vers un enfant ainsi lancé** passe par son `LiveIn`, sans
  perte, au lieu du bus ;
- **la boucle lit le `LiveOut`** de l'enfant comme le résultat d'un outil
  qui arrive par morceaux (`tool_result_in` ouvert, §4.2 de la page 03).

Quand `AgentNode` existera (lot 4), son `peer_in` ouvert est exactement ce
`LiveIn`, vu de l'intérieur. **La forme de tour reste au proto** ; ma part
est la primitive, ses poignées, et le mode flux de `SpawnNode`.

### 2.4 Les pairs qui ne sont pas des enfants

Un message d'un pair quelconque (`send` vers un nom de l'annuaire) n'a pas de
canal : il reste sur le bus, et peut se perdre, en le disant. Une
conversation qui ne doit rien perdre entre deux agents se fait donc par un
lancement en flux, dans un sens ou dans l'autre. Un « canal par paire » dans
l'annuaire serait un lot à part, si un gabarit le demande.

## 3. Le partage

| chez moi | chez le proto |
|---|---|
| `Streams`, `Live`, `LiveIn`, `LiveOut`, et l'ouvrier extérieur dans le hub et le détecteur | le verbe `spawn` en mode flux, `send(#h)` par `LiveIn`, la lecture de `LiveOut` dans la boucle |
| `SpawnNode` en mode flux (`stream`, `replies`) | `AgentNode` (lot 4), son `peer_in` branché sur ce mécanisme |
| les témoins de l'exécuteur, le témoin sur modèle local, la mesure | la forme de tour ; le correctif de la boîte qui déborde (`boite-pleine`, en cours) |

## 4. Les témoins, rouges d'abord

1. **Une conversation** : l'appelant envoie trois messages par `LiveIn`, et
   un graphe d'écho lancé rend trois réponses par `LiveOut`, dans l'ordre.
   `close` fait finir le run ; `LiveOut` rend `None` ; le registre le
   retire.
2. **Le refoulement traverse la frontière** : l'appelant lit lentement
   `LiveOut` ; l'enfant, rapide, est retenu (avance ≤ 64). Puis un enfant
   lent : les `send` de l'appelant bloquent, et l'avance est bornée.
3. **Pas d'interblocage pendant que l'appelant pense** : l'appelant dort
   200 ms entre deux envois, et rien n'est déclaré. **Un vrai interblocage
   est dit** : l'appelant en `recv` sur un enfant qui attend son entrée.
4. **L'annulation par référence** : `cancel` donne `recv` → `Err(cause)` et
   `send` → `Err(Stopped)`. La fin est dite avec sa cause.
5. **L'appelant qui tombe** : `LiveIn` lâché sans `close` ferme l'entrée,
   et l'enfant finit au lieu de vivre à vide.
6. **La durée** ferme l'entrée (cause `durée`), et la sortie le dit.
7. **`SpawnNode` en mode flux** : un flux de trois messages, un seul
   lancement, trois réponses sur `replies`.
8. **Le vrai modèle** (protocole) : qwen3-1.7b sous `llama-server` (le
   harnais `start_llama`). Un graphe à `LlmNode` lancé en flux reçoit trois
   questions l'une après l'autre et rend trois réponses, sans rien perdre.

## 5. La mesure

- **Le coût d'un message à travers la frontière** : de `LiveIn::send`
  jusqu'à la réponse lue sur `LiveOut`, médiane et 99e centile, sur 1 000
  messages d'écho.
- **Le refoulement de bout en bout** : l'avance maximale de chaque côté, sur
  les deux cas du témoin 2.
- **Le même échange par le bus** (`send` puis la boîte), pour comparer : le
  coût, et les pertes au-delà de 64 en attente.

## 6. Les questions ouvertes

1. **Le mode flux de `SpawnNode` dans le même nœud** (un réglage `stream`),
   ou un nœud à lui (`ConverseNode`) ? Je recommande le réglage : le verbe
   et le nœud restent un seul mécanisme.
2. **`max_duration` ferme l'entrée** (une fin propre) **ou annule** ? Je
   recommande de fermer : ce qui est en cours se termine, et la sortie dit
   la cause.
3. **Le correctif de la boîte qui déborde** (`6858a9ebb`, en cours sur
   `boite-pleine`) : il touche `agent.rs`. Cette page en dépend pour que
   les pertes du bus soient dites ; elle ne le refait pas.
