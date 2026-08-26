# 12 — Cahier des charges : ce qu'il faudrait à luciole pour remplacer tokio chez nous

26 août 2026, 4h30. Suite du [48](../23-aout-2026-20h33/48-pour-lucivy-briques-manquantes-dans-luciole.md)
(23 août) et de la nuit du [07](../vision_roadmap_08_2026/07-evenements-runs-et-boucles.md).
Le wasm est abandonné pour rag3weaver, tokio est revenu **pour attendre** ;
ce document dit ce que luciole devrait offrir pour qu'on puisse, un jour,
retirer tokio sans rien perdre — et comment on le vérifierait.

## 0. Le doc 48 est-il suffisant ?

Non, et ce n'est pas sa faute : c'est un **rapport de terrain** — ce qu'on
a heurté, vérifié dans la source, classé par valeur. Il est juste, et ses
priorités tiennent (les permis avant tout, la garde contre le `send`
bloquant, l'annulation ensuite). Mais il ne dit pas ce qu'on *exige* : ni
les garanties, ni les cas d'usage qui les motivent, ni comment on saurait
que c'est fait. Et il est antérieur à ce qui a changé depuis : un bus à
sujets, des runs qui se parlent, un réacteur, des minuteurs — tout ce qui
tourne aujourd'hui sur tokio. Un cahier des charges, c'est le doc 48
retourné : d'abord les usages, puis les primitives, puis les tests qui
disent « parité ».

## 1. Ce que veut dire « parité »

Parité avec tokio **pour nos usages**, pas avec l'écosystème async. On ne
demande pas à luciole de faire tourner `reqwest` ou `hyper` — ça exige un
réacteur d'I/O complet, et le doc 48 avait raison : c'est un projet en soi,
probablement pas le bon. On demande que tout ce que rag3weaver fait
*attendre* aujourd'hui puisse l'être sur le pool luciole, sans fil bloqué
pour rien, sans scrutation, et sans qu'un thread de calcul soit pris en
otage par de l'I/O.

Ce qu'on attend, concrètement, aujourd'hui et demain :

| Usage | Aujourd'hui | Demain |
|---|---|---|
| Le réacteur : N sujets du bus + minuteurs + arrêt, dans un fil | tokio `select` + `time::sleep` + `oneshot` | — |
| Les appels LLM cloud en SSE, annulables, plusieurs en parallèle | `ureq` bloquant sur un thread du pool (`task_pipe_to`, `Priority::Idle`) | 8 appels simultanés sans bloquer 8 threads de calcul |
| Réessais avec attente (429 : 60 s, 120 s…) | `std::thread::sleep` dans le fil bloqué | une attente qui ne coûte pas un thread |
| Les jetons du modèle vers un consommateur | `TokenSink` par rappel, `try_send` + `run_one_step` | un port de flux, avec fin et erreur |
| Un agent qui lit sa boîte entre deux tours | curseur + `try_recv` | — (déjà sans tokio) |
| Deux agents qui conversent | réacteur | — |
| Un serveur HTTP pour le portfolio (mode recherche) | rien encore | axum sur tokio, ou l'équivalent |
| Une boîte durable en base, sondée | rien encore | un minuteur, une requête, un message |

## 2. Les primitives, avec leur sémantique

Chacune est décrite par ce qu'elle garantit, pas par son API. Les noms sont
des propositions.

### P1. Attendre plusieurs sources à la fois — `select`

Attendre, depuis un thread de scheduler, **sans le bloquer**, qu'une source
parmi N soit prête : une mailbox, un signal, un minuteur, un permis. Rend
laquelle. L'attente est coopérative (le thread exécute d'autres tâches
pendant ce temps — c'est ce que `in_cooperative_wait` / `run_one_step`
savent déjà faire, mais sur une seule source, à la main). Sans ça, un
réacteur multi-sujets est une scrutation.

Garanties : équité entre sources prêtes ; réveil immédiat (pas de tick) ;
annulable (P3) ; utilisable aussi **depuis un thread hors pool** (le fil
du réacteur), par un point d'entrée bloquant.

### P2. Minuteurs — `after`, `every`

Une source qui devient prête à un instant : `after(Duration)` une fois,
`every(Duration)` en boucle. Monotone, annulable, sans thread dédié par
minuteur (une roue ou un tas, un seul point de réveil). `batch` et
`debounce` s'écrivent dessus. En test, une **horloge virtuelle**
(`pause` / `advance`) pour que « 200 ms » soit déterministe — tokio l'a
(`time::pause`), et nos tests de politiques en ont besoin pour ne pas dormir.

### P3. Annulation explicite — `CancelToken`

Un jeton en arbre (annuler le parent annule les enfants), consultable
(`is_cancelled`), attendable (une source pour P1), et **propagé** : par
`task_pipe_to`, par `execute_dag`, par les mailboxes. Un flux (P5) qu'on
annule ferme sa mailbox — l'annulation implicite d'aujourd'hui devient un
cas particulier, pas le seul chemin. Un producteur qui calcule longtemps
sans émettre doit pouvoir être interrompu entre deux pas (coopératif,
comme tout le reste). C'est ce qui rend `interrupt` (doc 07 §3.2) propre.

### P4. Permis nommés — `permits("network", 4)`

Le point 3 du doc 48, inchangé : borner les tâches simultanées d'une
catégorie, en attendant coopérativement (jamais en bloquant un thread). Le
code existe (`merge_permits`) ; sa nature est celle d'une primitive de
scheduler. Un permis est une source pour P1.

### P5. Un port de flux de premier ordre

Un `PortValue` qui transporte un flux : émetteur `try_send` + attente
coopérative quand c'est plein (jamais un `send` bloquant — la règle du doc
48 §4 devient **impossible à violer**, pas seulement documentée), **fin de
flux** explicite, **erreur** explicite, contre-pression bornée, et
`on_finish` qui est un envoi comme un autre. Un nœud de DAG peut alors
être « ponctuel » ou « streamant » sans que chacun réinvente la fin. C'est
le « streaming » dont on parlait : pas l'I/O réseau, la *forme* du flux
dans le graphe.

### P6. Une voie pour l'I/O bloquante

Deux options, et il faut en choisir une :

- **(a) une voie dédiée** : un ensemble de threads séparé du pool de calcul,
  réservé à l'I/O bloquante (`ureq`, un fichier, une base), avec P4 pour le
  borner. Les threads de calcul ne sont jamais pris ; l'attente côté
  demandeur passe par P1. C'est simple, c'est probablement suffisant, et
  c'est l'avis du doc 48.
- **(b) un petit réacteur d'I/O** (epoll/kqueue + table de réveils), avec
  un `AsyncRead`/`AsyncWrite` maison. Ce n'est plus « un thread par appel »
  mais c'est un projet, et il n'ouvre pas l'écosystème pour autant (celui-ci
  veut tokio, pas un réacteur quelconque).

**Ce qu'on demande : (a)**, et que (b) reste une porte documentée.

### P7. Pub/sub à sujets, avec débordement visible

Ce que fait notre bus (`async_broadcast` : sujets créés à la demande,
tampon par sujet, le plus ancien écarté en cas de débordement, et le
récepteur **le sait** — `Overflowed(n)`). Si luciole l'offre, le bus n'a
plus de dépendance externe et un récepteur est une source pour P1. Sinon on
garde `async_broadcast`, qui n'est pas tokio.

### P8. Concurrence structurée

`scope` existe. Il manque : un groupe de tâches qui s'annule quand on le
lâche, qui remonte la première erreur, et qu'on peut attendre **avec un
délai** (P1 + P2). C'est ce qui empêche une conversation de deux agents de
durer toujours autrement que par un compteur.

### P9. Les gardes

Le point 4 du doc 48 : un `send` bloquant depuis un thread de scheduler est
une erreur, pas un gel — `debug_assert!`, ou une API qui ne permet pas de
l'écrire. Et le `WaitGraph` qui existe déjà devrait voir les minuteurs et
les permis, pour que « pourquoi c'est bloqué » ait une réponse complète.

## 3. Les tests qui disent « parité »

Cinq scénarios ; tant qu'ils ne passent pas sur luciole seul, tokio reste.

1. **Le réacteur, réécrit sur P1 + P2 + P3, sans tokio** : les tests
   existants passent tels quels — un lot de cinq en une exécution
   (`batch 40`), `each` immédiat, `debounce` après le calme, la trace en
   fil pendant qu'un agent travaille (11 lignes, pas une de plus), deux
   agents qui se répondent six fois sous un budget.
2. **Huit appels SSE simultanés sur un pool de quatre threads**, la
   recherche continue de répondre sous 50 ms pendant ce temps (P4 + P6).
3. **Annuler un appel LLM en cours** : moins de N trames écrites par le
   serveur après l'annulation (aujourd'hui 4 sur 100 000, par l'effet de
   bord de la mailbox fermée — il faut au moins ça, par P3).
4. **Le pool d'un thread et la mailbox de un** : un producteur qui bloque
   sur un `send` est une **panique avec un message**, pas un gel de 3 s.
5. **Un `batch 200` sous horloge virtuelle** : le test avance le temps, ne
   dort pas, et le résultat est déterministe.

## 4. Ce qu'on ne demande pas

- La parité avec l'écosystème async (`reqwest`, `hyper`, `axum`). Le jour
  où on veut un serveur, il tourne sur tokio dans son propre fil et parle
  au reste par le bus — tokio attend, luciole calcule, les deux cohabitent
  ([07](../vision_roadmap_08_2026/07-evenements-runs-et-boucles.md) §6).
- Le wasm : abandonné pour rag3weaver ; ce que luciole fait pour lucivy
  dans le navigateur ne nous concerne plus.
- Que ce soit fait vite. Rien ne bloque : tokio fait le travail, et
  l'adaptation, si elle vient, portera sur du code éprouvé.

## 5. Par où commencer, si luciole s'y met

Dans l'ordre du rendement, comme le doc 48 : **P4** (le code existe),
**P9** (une ligne), **P1 + P2** (c'est ce qui rend tout le reste possible,
et c'est ce que tokio nous donne aujourd'hui), **P3**, **P5**, puis **P6a**.
P7 et P8 viennent naturellement après P1.
