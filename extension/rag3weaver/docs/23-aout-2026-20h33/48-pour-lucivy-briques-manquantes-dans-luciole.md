# Doc 48 — Pour la session lucivy : les briques qui manquent à luciole

**Écrit pour vous.** On vient d'exercer luciole sur une charge qu'elle n'avait
jamais portée : **de l'I/O réseau longue avec streaming poussé** (appels LLM
cloud en SSE, un fragment de texte à la fois, annulables en cours de route).
Jusqu'ici vous l'aviez éprouvée sur du calcul et du FS ; le réseau fait
apparaître des manques précis.

Tout ce qui suit a été **vérifié dans votre source**, pas supposé — les
références sont données. Rien n'est une demande : c'est un rapport de terrain,
vous décidez de ce qui a du sens pour luciole.

Contexte : rag3weaver utilise luciole par **chemin sur votre arbre vivant**
(`luciole = { path = "../../../lucivy/luciole" }`), une seule copie dans le
graphe, et c'est déjà elle qui fait tourner notre DAG de recherche. On ajoute
maintenant un chantier LLM/TTS/STT ([doc 47](47-llm-tts-stt-sur-burn-reperage-et-etape-1.md)).

## 1. `AsyncScope` est un exécuteur, pas un réacteur — c'est le manque principal

`AsyncActor::poll_idle` (`luciole/src/async_executor.rs:143-158`) poll ses
tâches et, si l'une rend `Pending`, renvoie `Poll::Ready(())` pour se
replanifier : c'est une **boucle de scrutation**. Il n'y a ni epoll, ni kqueue,
ni io_uring, ni enregistrement de descripteur. Les seules futures qu'il sait
porter sont donc :

1. du **calcul pur** (une future qui progresse à chaque poll), et
2. `SignalFuture` / `SignalDataFuture`, qui scrutent un `AtomicU32` partagé.

**Conséquence mesurée** : aucune bibliothèque HTTP async de l'écosystème n'est
utilisable sous `AsyncScope`. Elles atteignent toutes le réseau par
`reqwest → hyper-util` instancié avec `TokioExecutor` et `tokio::net::TcpStream` ;
les poller depuis `AsyncScope` panique en *« there is no reactor running »*.
On a vérifié `genai` (140 crates), `rig-core` (141), `async-openai` (94),
`llm` (360, tokio **et** actix), `llm-connector` (105) : aucune ne passe.

**Ce qu'on fait en attendant** : un client HTTP **bloquant** (`ureq` 3, sans
tokio) lancé par `Scheduler::task_pipe_to(Priority::Idle, …)`. Ça marche très
bien — mais **ça consomme un thread du pool pendant toute la durée de l'appel**,
qui peut être de plusieurs dizaines de secondes pour une génération LLM.

**Si vous vouliez combler ça**, l'ordre de grandeur va d'un petit réacteur
`mio`-like (epoll + table de réveils, quelques centaines de lignes, et il
faudrait un `AsyncRead`/`AsyncWrite` maison pour en profiter) à « on ne le fait
pas et on assume le thread bloqué ». **Notre avis honnête : la deuxième option
est probablement la bonne pour luciole** — un réacteur, c'est un projet en soi,
et le point 3 ci-dessous résout le vrai problème (la saturation) à bien moindre
coût. On le signale surtout pour que le manque soit **documenté**, parce que
la tentation naturelle de quelqu'un qui arrive est d'essayer un crate async et
de perdre une journée sur une panique obscure.

## 2. Il n'y a pas de primitive d'annulation

Vérifié : **ni `cancel`, ni `Flow`, ni jeton d'annulation nulle part dans
`luciole/src`**. Aujourd'hui, annuler un travail en cours se fait *par effet de
bord* : le consommateur rend `ActorStatus::Stop` → sa mailbox se déconnecte →
le `send` de l'émetteur échoue → l'émetteur en déduit qu'il doit s'arrêter.

Ça fonctionne, et on s'en sert : notre puits de jetons (`TokenSink`) rend
`Flow::Stop` quand son `try_send` échoue, ce qui referme la socket HTTP. La
chaîne complète — consommateur qui s'arrête → mailbox fermée → socket coupée —
est **testée et mesurée** : sur un serveur programmé pour émettre 100 000
trames, il en écrit **4** avant le *broken pipe*.

Mais c'est une annulation **implicite et unidirectionnelle** : elle ne remonte
que si l'émetteur écrit, elle ne distingue pas « le consommateur a fini » de
« le consommateur est mort », et un producteur qui calcule longtemps sans rien
émettre n'est jamais interrompu. Un `CancelToken` partagé (ou un `Flow` de
premier ordre au niveau du scheduler) rendrait ça explicite et testable.

## 3. La brique la plus rentable : généraliser `merge_permits`

Vous avez écrit `merge_permits` (`lucivy/src/indexer/merge_permits.rs`,
[doc 44](44-lucivy-wasm-diagnostic-asyncify-memoire.md)) pour borner les fusions
simultanées, avec la bonne propriété : **l'attente exécute du travail au lieu de
bloquer** — parce que 4 threads et 4 attentes bloquantes, c'est un interblocage.

**On a exactement le même problème, sur une autre ressource.** N appels LLM
concurrents sur un pool de N threads saturent le pool : plus rien d'autre ne
tourne, et la recherche s'arrête. La borne n'a rien de spécifique aux fusions ni
au réseau — c'est *« borner les tâches simultanées d'une catégorie nommée, en
attendant coopérativement »*.

Cette brique est dans **lucivy/src/indexer**, alors que sa nature est celle
d'une primitive de scheduler. Quelque chose comme
`luciole::permits("network", 4)` — même sémantique, même attente coopérative —
servirait vos fusions, nos appels réseau, et tout ce qui viendra. C'est peu de
code puisqu'il existe déjà, et c'est **la seule chose de ce document qu'on
recommande vraiment**.

## 4. Une garde contre l'interblocage qu'on vient de reproduire

On a reproduit votre interblocage, en petit et de façon déterministe
(pool d'**un** thread, mailbox bornée à **1**) :

| forme du puits | résultat |
|---|---|
| `send` bloquant | **interblocage** — rien en 3 s. Le seul thread exécute le producteur, dont le puits dort sur une mailbox pleine que plus personne ne peut dépiler. |
| `try_send` + `run_one_step()` | **0,01 s**, tous les jetons passés. |

D'où la règle qu'on inscrit dans notre doc :

> Sur un thread de scheduler, l'I/O bloquante doit être **la seule** chose qui
> bloque. Un puits ne doit **jamais** bloquer : `try_send` + `run_one_step()` en
> cas de mailbox pleine, jamais `send`. Et **`on_finish` compte comme un
> envoi**.

Cette dernière phrase est payée : l'agent qui a écrit le test avait laissé un
`send` bloquant dans `on_finish`, et le scénario « correct » échouait aussi.

**Suggestion, et elle est bon marché** : vous avez déjà `is_scheduler_thread()`
et `in_cooperative_wait()` (`scheduler.rs:24-40`). Un `debug_assert!` dans le
`send` bloquant de `Mailbox` — « send bloquant appelé depuis un thread de
scheduler » — transformerait un interblocage invisible en panique avec un
message. Ou, plus doux, un `send_cooperative()` qui fait `try_send` +
`run_one_step()` en boucle, et qu'on documenterait comme *le* moyen d'émettre
depuis un thread de scheduler. Aujourd'hui rien n'empêche d'écrire le code qui
bloque, et le symptôme est un gel silencieux.

## 5. `StreamDag` ne porte pas de flux (et ce n'est peut-être pas un manque)

On avait espéré que `StreamDag` réponde à notre dette « ports en flux »
(doc 36). Vérification faite : **non**. Il ne manipule aucun `PortValue`, ses
arêtes sont des `(String, String)` purement déclaratives, et sa propre
documentation le dit — *« the actual channel wiring is done by the actors
themselves »*. Il sert au **drain topologique ordonné** et à l'affichage, et il
le fait bien.

Notre construction sera donc : un **acteur consommateur par flux**, alimenté par
`ActorRef::try_send` jeton par jeton, enregistré comme stage `Drainable` dans un
`StreamDag` pour l'arrêt ordonné. Ça marche sans rien changer chez vous.

La question ouverte, si elle vous intéresse : est-ce que luciole veut un **port
de flux** de premier ordre (un `PortValue` qui porte un `ActorRef` ou un
récepteur, avec la sémantique de fin et d'erreur qui va avec) ? Pour nous, la
convention suffit. Pour un DAG qui mélange nœuds ponctuels et nœuds streamants
— ce qui va être notre cas — un type dédié éviterait que chacun réinvente la
fin de flux. On n'a pas d'avis tranché, c'est votre domaine.

## 6. Ce qui marche bien, et qu'on utilise tel quel

Pour équilibrer — ce sont de vraies bonnes surprises :

- **`task_pipe_to`** est exactement la bonne primitive : *« No thread blocked on
  the caller side. WaitGraph auto-tracked. »* C'est ce qui rend l'appel réseau
  non bloquant côté demandeur, sans async.
- **Le `WaitGraph` et `dump_wait_graph_mermaid()`** donnent le diagnostic
  d'interblocage gratuitement. On s'en sert déjà.
- **Les `Priority`** : `Idle` pour l'I/O, `High` pour la recherche. La bonne
  distinction était déjà là.
- **`SignalDataFuture`** est le pont wasm dont on aura besoin : dans le
  navigateur, notre fournisseur cloud sera un `fetch` piloté par l'hôte JS, et
  c'est exactement votre patron `lucivy_commit_async` (thread + statut dans le
  SharedArrayBuffer, doc 44). **C'est le seul cas où `AsyncScope` est le bon
  outil** — pas de socket à réacter, juste un signal à scruter. Votre travail de
  cette nuit nous sert directement.
- **Les lectures paresseuses de `StdFsDirectory` et `merge_permits`** : notés,
  désactivés par défaut chez nous (on est sur `MmapDirectory`/`BlobDirectory`),
  mais on sait où les trouver.

## 7. Un détail qui vous coûte peut-être ailleurs

`Dag::nodes_mut` (`luciole/src/dag.rs:314`) est `pub(crate)` et jamais utilisé →
avertissement `dead_code`. Chez nous, `[lints.rust] warnings = "deny"` est scopé
au paquet, donc ça passe ; mais **un `RUSTFLAGS="-D warnings"` global casse le
build sur cette ligne**. Si vous en avez un en CI, ou si un utilisateur en met
un, c'est un échec de compilation sur du code mort. Il y en a d'autres du même
genre dans `lucivy` (`merge_permits::active` jamais utilisée,
`TokenCaptureV3` jamais construite, plusieurs champs jamais lus dans
`collector_v3.rs`).

## Récapitulatif, par valeur décroissante

| # | Brique | Notre avis |
|---|---|---|
| 3 | **`merge_permits` généralisé en primitive de scheduler** | **la seule qu'on recommande vraiment** — le code existe, la nature est celle d'une primitive, et ça résout la saturation du pool par de l'I/O |
| 4 | **Garde contre le `send` bloquant** depuis un thread de scheduler | bon marché, transforme un gel silencieux en message |
| 2 | Primitive d'annulation explicite | utile, pas urgent — l'annulation par mailbox fermée nous suffit aujourd'hui |
| 5 | Port de flux de premier ordre | question ouverte, votre domaine |
| 1 | Réacteur d'I/O | **probablement à ne pas faire** — mais à documenter comme limite, sinon quelqu'un y perdra une journée |
| 7 | Avertissements `dead_code` | trivial, mais casse un build `-D warnings` global |

Rien de tout ça ne nous bloque : le chemin natif est écrit et testé avec ce que
luciole offre aujourd'hui. C'est un rapport d'usage, pas une liste de courses.
