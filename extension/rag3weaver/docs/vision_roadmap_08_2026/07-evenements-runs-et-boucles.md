# 07 — Événements, runs et boucles : comment les graphes et les agents se parlent

26 août 2026, 2h. Une vision à part entière, née d'une question de trace
(« comment savoir exactement ce qu'un agent a fait ? ») qui s'est généralisée
en trois pas : un bus à sujets, l'identité des runs, et la question de
**quand** une boucle peut réagir à ce qu'on lui dit.

## 1. Les invariants

Cinq règles, et tout le reste en découle.

1. **Fire and forget, toujours.** Publier ne bloque jamais : un tampon
   plein écarte le plus ancien (et le récepteur le saura), un sujet sans
   abonné jette tout. Une boucle ne ralentit jamais pour son observateur.
2. **Un bus, plusieurs sujets, créés à la demande.** Un sujet est un nom ;
   chaque sujet a son canal et son tampon. Un consommateur choisit ses
   sujets — c'est ce qui empêche l'écho par construction : le graphe de
   trace lit `agent` et `dataflow`, écrit dans le catalogue qui publie sur
   `catalog`, que personne dans cette boucle n'écoute.
3. **L'identité d'un run est son adresse.** Chaque exécution — de graphe,
   d'agent — a un `run_id`, porté par chacun de ses événements, publié sur
   `run.<id>` en plus du sujet de catégorie, et lié à son parent quand un
   outil lance un graphe. On parle à un run par `run.<id>.inbox`.
4. **Le bus est vif et local ; la base est durable.** Un run terminé
   n'écoute plus : sa trace se *lit* (entité `Trace`, filtrée par run). Un
   run dans un autre processus ne s'atteint pas par le bus : il faut une
   boîte aux lettres en base, lue par un nœud de même forme.
5. **Une boucle réagit à sa frontière, pas au milieu.** Un agent lit sa
   boîte **entre deux tours** ; un graphe lit la sienne **quand un nœud
   s'exécute** ; un réacteur lance un graphe **quand un événement arrive**.
   Personne n'est interrompu au milieu d'un appel — sauf à le demander
   explicitement, et alors on perd l'appel.

## 2. Le vocabulaire

| Mot | Ce que c'est |
|---|---|
| **événement** | un fait publié : « tel nœud a fini en 12 ms », « tel appel d'outil a échoué ». N'attend rien de personne. |
| **message** | un événement **adressé** (`from`, `to`, `content`), publié sur la boîte du destinataire. N'attend rien non plus : un accusé est un second message. |
| **sujet** | un nom de canal. Catégories (`catalog`, `search`, `agent`, `dataflow`, `messages`), runs (`run.<id>`), boîtes (`run.<id>.inbox`), et n'importe quoi d'autre à la demande. |
| **curseur** | un récepteur nommé, gardé par le bus : un nœud construit plus tard retrouve par son nom un récepteur ouvert plus tôt, et ne rate pas l'intervalle. |
| **run** | une exécution identifiée : un graphe (`execute`), une boucle d'agent (`Agent::run`). A un `run_id`, un parent éventuel, et une trace. |
| **boîte** (inbox) | le sujet `run.<id>.inbox`. Relatif dans une fiche : `topics='inbox'` se résout contre le run courant. |
| **réacteur** | la boucle qui attend sur un sujet et lance un graphe par événement. C'est ce qui rend un DAG « événementiel » : un DAG n'a pas de boucle, le réacteur en est une. |

## 3. Les trois boucles, et quand chacune écoute

### 3.1 Le graphe : il lit quand un nœud s'exécute

Un DAG est une exécution : il commence, il finit. Il n'a pas de boucle,
donc il ne « s'abonne » pas au sens d'un rappel. Il lit sa boîte là où sa
topologie le dit : un `EventSourceNode(topics='inbox')` quelque part dans
le graphe, qui draine ce qui attend **à ce moment-là**. Un message arrivé
après ce nœud sera vu par le run suivant — ou par personne, si le curseur
n'est pas gardé. D'où le curseur : `run.<id>.inbox@<nom>` survit au run, et
c'est le réacteur (§3.3) qui décide qu'il y a un run suivant.

Il écrit avec `SendMessageNode(to=$run_id, content=$…)` : `emit_on(run.<to>.inbox, Message { from: mon run })`. Pour trouver `to`, il regarde la
base : la liste des exécutions du checkpoint store, ou une ligne `Trace`
`RunStarted` — l'id est une donnée comme une autre, cherchable.

### 3.2 L'agent : il lit entre deux tours

C'est la question posée, et la réponse est dans ce qu'on a déjà :
`final_nudge` pousse un tour `user` **avant** le dernier appel au modèle.
La boîte se lit exactement au même endroit. À chaque itération, avant
`generate` :

```
drainer run.<mon id>.inbox
pour chaque message : turns.push(Turn::user("[message de <from>] <content>"))
```

Le modèle voit les messages comme des tours, dans l'ordre d'arrivée, avec
l'historique intact. Un message arrivé pendant un appel d'outil est vu à
l'itération suivante. **La latence est un tour**, et c'est le bon compromis
par défaut : un tour, c'est la granularité à laquelle l'agent raisonne.

Deux niveaux au-dessus, à la demande :

- **`interrupt`** : un message marqué urgent fait répondre `Flow::Stop` au
  puits pendant la génération en cours. L'appel est perdu (le texte
  partiel est gardé, l'appel d'outil éventuel est refermé — c'est le chemin
  `closed_orphans` qui existe déjà), et le message est vu tout de suite.
  À réserver à « arrête » et « change de mission ».
- **Le réacteur d'agent** : un agent **terminé** n'écoute plus, sauf si une
  boucle externe attend sur sa boîte et relance `Agent::run` à chaque
  message, en gardant l'historique. C'est ainsi que deux agents
  conversent : chacun est un réacteur sur sa boîte, un message déclenche un
  run, le texte final part en message vers l'autre. Ce qui borne ça, c'est
  ce qui borne tout le reste : des `AgentLimits` sur le nombre d'échanges,
  et un protocole de fin (`done`) qui n'est qu'un message de plus.

Ce que ça ne fait **pas** : injecter au milieu d'une génération, ou
modifier un tour passé. L'historique reste bien formé, ce qui est la
condition de tout le reste (rejouabilité, `orphan_tool_calls`).

### 3.3 Le réacteur : il lance quand un événement arrive

La fiche d'un graphe-outil déclare à quoi elle réagit :

```
%% tool: trace
%% on: agent, dataflow          -- les sujets
%% policy: batch 200ms           -- each | batch <durée> | debounce <durée>
```

Le réacteur est un objet, `Reactor::new(bus)`, à qui on confie des fiches.
En natif, il tourne dans un fil (`recv_blocking` sur ses curseurs — le bus
le permet) ; en wasm, pas de fil : l'hôte appelle `reactor.pump()` sur un
minuteur, et c'est la même boucle. Par événement (ou par lot), il
instancie le graphe avec l'événement en paramètre (`$event`, ou le lot sur
un port) et l'exécute avec **ses** services — ce qui, encore une fois,
décide de ce que ce run publie (`event_bus`) et lit (`events`).

Le graphe de trace d'aujourd'hui est un réacteur sur `agent, dataflow` en
politique `batch`, qu'on pompe à la main. Rien d'autre.

## 4. La trace, avec identité

Chaque ligne `Trace` porte `run_id`, `parent_run_id`, `kind`, `agent`,
`tool`, `call_id`, `node`, `ok`, `ms`, `tokens`, `at_ms`, et un `summary`
cherchable. Deux questions deviennent des recherches :

- **« qu'est-ce qui s'est passé dans le run X ? »** : `Trace` filtré par
  `run_id`, et par `parent_run_id` pour descendre sous les outils ;
- **« qu'est-ce que j'ai déjà essayé ? »** : la même, `run_id = le mien`,
  posée par l'agent lui-même avec `search(target="Trace")`. La trace n'est
  plus seulement pour l'humain qui relit : c'est une mémoire de travail.

Ce que l'arbre donne : agent → appel d'outil (arguments exacts, résultat,
durée, réessais) → run de graphe → nœuds (durée, erreur). Ce qu'on ne met
pas dedans : les instantanés de ports du runtime (`DataflowEvent`), qui
restent son flux propre pour une interface — une trace ne transporte pas
des résultats entiers.

## 5. Ce qui existe, ce qui manque

| | Où on en est |
|---|---|
| Bus à sujets, curseurs, `emit`/`emit_on` | fait (`events.rs`) |
| Événements `LlmCall`, `ToolCall*`, `NodeRun`, `Message` | faits |
| L'agent publie (`with_events`), le runtime publie (`event_bus`) | fait |
| `EventSourceNode(topics, cursor)`, `TraceSinkNode`, entité `Trace`, graphe `trace.mmd` | faits, sur sujets et curseurs ; second drain à 0 (pas d'écho) |
| `run_id` partout, `run.<id>`, `parent`, champs `Trace` | fait : `RunStarted`/`RunFinished`, `ctx.run_id()`, `execute_as`, `ToolBox::call_in` + `ServiceRegistry::layered` pour le parent, `AgentRun.run` |
| `SendMessageNode`, `inbox` relatif, `Agent::with_inbox` (lecture entre tours) | à faire — l'étape suivante |
| `interrupt` | à faire, petit (le puits sait déjà arrêter) |
| `%% on:` / `%% policy:`, `Reactor` natif + `pump` | à faire |
| Boîte durable (`Message` en base, `MessageSourceNode`) | plus tard, même forme de nœud |

L'ordre : identité des runs, puis messages et boîte de l'agent (avec le
test où un graphe envoie un message à un agent par son id et l'agent le
voit au tour suivant), puis le réacteur. Un commit par étape.

## 6. Ce qui gêne, et ce qu'on a tranché

- **Pas de `getrandom` en wasm** : les ids sont un compteur atomique plus
  un horodatage (`run-<n>-<ms>`), déterministes à la demande par blake3.
- **`execute()` n'a pas d'id aujourd'hui** ; il en génère un. Le checkpoint
  garde le sien.
- **Le lien parent** : `execute_definition` tourne sous un appel d'outil ;
  le `ServiceRegistry` de l'appel porte le run de l'agent (`"run"`).
- **Deux clés pour deux rôles** : `"event_bus"` = ce runtime publie ;
  `"events"` = ce graphe lit. C'est celui qui monte les boucles qui décide,
  et c'est ce qui empêche un graphe de trace de se retracer.
- **Un curseur ne voit que ce qui suit sa création.** On l'ouvre avant ce
  qu'on veut observer ; le réacteur les ouvre pour ses fiches à
  l'enregistrement.
