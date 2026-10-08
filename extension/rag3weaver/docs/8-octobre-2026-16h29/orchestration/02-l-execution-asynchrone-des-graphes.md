# L'exécution asynchrone des graphes — la voie retenue

8 octobre 2026, orchestration, à la demande de Lucie. Rien n'est codé. C'est la
page qu'une session lit avant de prendre le chantier C du plan de reprise
(`01-plan-de-reprise.md`).

## 1. D'où ça vient

Trois décisions de Lucie, le 8 octobre :

1. **Le navigateur (WASM) est abandonné pour l'instant** : « chiant de se
   passer d'async partout ». Le jour où il revient, ce sera un projet pour
   améliorer luciole, pas des restrictions dans rag3weaver. Plus rien ne
   justifie donc de tenir le cœur des graphes hors de tokio.
2. **L'exécuteur de commandes doit savoir lancer en fond**, et l'agent doit
   pouvoir consulter la sortie pendant qu'elle s'écrit (vision du produit
   code, §0).
3. **Pas d'asynchrone forcé** : « une variante async plutôt que forcé ; ça
   devient l'option par défaut mais pas obligé, pour les nœuds comme pour
   `execute` ». C'est le nœud qui dit s'il bloque, pas le runtime qui devine.

## 2. Ce qui est vrai aujourd'hui (lu, 8 octobre)

- `Node::execute(&mut self, ctx) -> Result<(), String>` est synchrone
  (`src/dataflow/node.rs`) ; 78 `impl Node` ; le runtime l'appelle en un seul
  endroit (`src/dataflow/runtime.rs`) ; 77 appels directs ailleurs, presque
  tous dans des tests.
- Les services sont synchrones et **bloquent** : `Embedder::embed`,
  `Llm::generate`, `Reranker::rerank` (GPU burn, HTTP). `Llm::generate` est
  synchrone exprès pour être appelable depuis `execute` ; les jetons sortent
  par un puits (`TokenSink`), qui est aussi le point d'annulation.
- La boucle d'agent (`src/agent.rs`) est un fil ; ses outils asynchrones sont
  des fils de portée (`std::thread::scope`) qui postent leur résultat dans la
  boîte de l'agent ; le réacteur (`src/dataflow/reactor.rs`) est un fil qui
  attend sur le bus.
- `async-trait` et tokio (`sync`, `rt`, `macros`, `time` ; `rt-multi-thread`
  pour les binaires) sont déjà des dépendances. Un seul fichier de `src/` a
  un `async fn`. Six `block_on` : un dans `reactor.rs`, cinq dans
  `postgres_connection.rs`.
- L'exécuteur de commandes dérive déjà chaque flux vers un journal, et `wait`
  attend un motif dedans (`src/dataflow/run_nodes.rs`, `src/commande.rs`).

## 3. La forme

### 3.1 Le trait : deux méthodes, une seule à écrire par nœud

```rust
#[async_trait]
pub trait Node: Send {
    // inchangé : les nœuds d'aujourd'hui ne bougent pas
    fn execute(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        Err(format!("{} : ni execute ni execute_async", self.name()))
    }

    // la variante : par défaut, elle joue `execute` en disant à tokio
    // que ce fil va bloquer
    async fn execute_async(&mut self, ctx: &mut NodeContext) -> Result<(), String> {
        tokio::task::block_in_place(|| self.execute(ctx))
    }
}
```

- Un nœud existant implémente `execute` : il bloque, tokio sort le fil du
  pool le temps de l'appel, le résultat est le même qu'aujourd'hui.
- Un nœud nouveau implémente `execute_async` et laisse `execute` au défaut.
- **Un seul des deux par nœud** ; l'autre reste au défaut. Un test du
  registre vérifie qu'aucun nœud n'implémente les deux (par lecture du
  schéma ou par un marqueur `is_async()` sur le nœud).
- Le runtime n'appelle plus que `execute_async`.

Pourquoi `block_in_place` et pas `spawn_blocking` : `spawn_blocking` exige
`'static` et ne peut donc emprunter ni `&mut self` ni le contexte ;
`block_in_place` garde l'emprunt et prévient l'exécuteur. Il demande le
runtime multi-fil (déjà le cas) et **n'est pas permis depuis un `block_on`
imbriqué** : c'est pour cela que les six `block_on` sont à traiter dans le
même lot.

### 3.2 Le runtime

- `DataflowRuntime::execute` devient `async` (et ses variantes
  `execute_with_report`, `execute_with_checkpoint`). Un `execute_blocking`
  reste pour les appelants synchrones et les tests, par `block_on` sur un
  runtime dédié — jamais depuis une tâche tokio.
- L'ordre topologique ne change pas dans ce lot : un nœud à la fois,
  `await` sur chacun. Le parallélisme des nœuds indépendants est une étape
  d'après, pas celle-ci.
- Les rapports, les points de reprise et `undo` ne changent pas.

### 3.3 La boucle d'agent et le réacteur

- La boucle d'agent devient une tâche ; ses outils asynchrones deviennent des
  `tokio::spawn` dont la poignée est gardée par le run (joints à sa fin,
  comme les fils de portée aujourd'hui : aucun résultat ne survit à l'agent
  qui l'a demandé). Le protocole ne change pas : un accusé « en cours » dans
  le tour, le vrai résultat plus tard dans la boîte.
- Le réacteur devient une tâche qui attend sur le bus ; son `block_on` disparaît.
- `Llm::generate` reste synchrone dans ce lot : appelé depuis un nœud qui
  implémente `execute` (donc sous `block_in_place`), il bloque comme
  aujourd'hui. Un `Llm::generate_async` viendra avec les ports-flux (§4).

### 3.4 Les six `block_on`

- `reactor.rs` : disparaît avec §3.3.
- `postgres_connection.rs` (cinq) : le client postgres est asynchrone et
  était ponté vers du synchrone ; sous un nœud `execute` joué par
  `block_in_place`, un `block_on` imbriqué panique. Deux issues, à trancher
  par la session en lisant le code : un runtime dédié au client postgres
  (`Handle` propre, `block_on` dessus), ou une connexion vraiment asynchrone
  derrière `execute_async` pour les nœuds qui l'utilisent.

## 4. La suite, hors de ce lot

Dans l'ordre, chacune une page courte avant de coder :

1. **Les ports-flux** : un port peut être un canal ; l'amont pousse, l'aval
   consomme pendant que l'amont travaille. Le schéma ne change pas, c'est le
   moment où chaque valeur arrive qui change. Premier client : `LlmNode`
   (« le streaming existe dans le trait, pas encore au travers du graphe »).
2. **Les puits d'affichage** : un nœud qui rend un flux ligne à ligne (page,
   journal, résumé pour l'agent) ; plusieurs par graphe. Une donnée, deux
   rendus.
3. **Le run en fond avec sa poignée** : un graphe lancé en fond rend `run:…`,
   l'agent lit l'état du run (nœuds finis, en cours, ports rendus), la fin
   arrive dans sa boîte. La commande en fond (vision du produit code, §0) en
   est un cas : son journal est un port-flux.
4. **Le parallélisme des nœuds indépendants** dans le runtime, quand un
   graphe réel le réclame et qu'on sait le mesurer.

## 5. Ce que le lot C doit rendre

- Le trait et son pont, le runtime, la boucle et le réacteur en tâches, les
  six `block_on` traités ; les 78 nœuds inchangés.
- Un nœud témoin qui implémente `execute_async` (un `sleep` tokio suffit), et
  le test « un seul des deux par nœud ».
- Toute la batterie du crate verte une fois, à la fusion (unitaires, e2e dont
  `e2e_code`, `e2e_dataflow_observe`, `e2e_checkpoint`, les suites du
  backend et du chat), sous `poste lourd`.
- Une mesure avant/après sur le banc de recherche et sur une passe d'agent du
  backend de code : l'asynchrone ne doit rien coûter de visible
  (`block_in_place` a un coût par appel, à mesurer, pas à supposer).

Fichiers : `src/dataflow/node.rs`, `src/dataflow/runtime.rs`,
`src/dataflow/reactor.rs`, `src/agent.rs`, `src/postgres_connection.rs`, les
appelants de `execute` dans les tests. Il ne touche ni `catalog`, ni
`code_sync`, ni les nœuds.
