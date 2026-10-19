# Le graphe d'un outil ou d'un composant, en JSON (C1)

19 octobre 2026, everything declarative. Le socle du DAG vivant : une route de l'IDE et un outil
d'agent qui rendent le même JSON, celui du graphe d'un outil, d'une réaction ou d'un composant.

## Décisions

1. **Une seule fonction, deux portes.** `PreparedBackend::graph_json(GraphTarget)` rend le JSON.
   La route `GET /ide/graphe?backend=<id>&tool=…` (ou `reaction=`, `component=`) le sert à la
   page, et l'outil `graph` à l'agent. Ce que l'une montre, l'autre le dit.
2. **La forme est celle de `GraphDefinition`**, déjà sérialisée par `validate_search` et par les
   points de reprise :
   - `nodes` : `name`, `node_type`, `config` telle qu'écrite (avec ses `$param`) ;
   - `edges` : `from_node`, `from_port`, `to_node`, `to_port`.

   Elle est enrichie de deux choses :
   - `line`, la ligne de déclaration du nœud dans son `.mmd`, pour mener du dessin à la source ;
   - `schema`, le schéma du type d'un nœud de flux : ses ports et ses paramètres, la même forme
     que les nœuds de `describe_backend`.

   Un outil ajoute `inputSchema` et `result`, une réaction `result`.
3. **Le nom d'un nœud est celui des `NodeRun`**. C'est ce qui allumera un nœud dans le dessin
   vivant. Le run, lui, se reconnaît par `RunStarted.target` (décision 7). Où passent ces
   événements : décision 9.
4. **Un composant donne son graphe de clic**, relu sur le disque comme l'aperçu
   (`components_on_disk`), dans la forme annoncée par `dataflow/clic.rs` : `node_type` est le
   genre (`Evenement`, `Pur`, `Effet`…). Les genres ne sont pas des nœuds du registre, et ces
   nœuds n'ont donc pas de `schema`. Un composant sans graphe de clic rend un graphe vide
   (`source: null`) et non un refus : c'est un composant sans comportement déclaré.
5. **L'outil `graph` est opt-in** (`"graph_tool": true` dans le manifeste), comme
   `search_graphs`. Il change la liste d'outils que voit un agent, et donc ses invites. Le gabarit
   `code` l'active. La route de l'IDE sert tout backend ouvert, sans drapeau. Le nom `graph` est
   réservé quand le drapeau est posé.
6. **Identifiants en anglais** (`graph`, `tool`, `reaction`, `component`), selon la règle du
   dépôt. Le français reste pour les messages et les docs.

7. **Un run dit ce qu'il exécute** (au début de C2, accord de c7) : `RunStarted` gagne `target`,
   de la même forme que les arguments de l'outil `graph` (`{"tool": …}`, `{"reaction": …}`,
   `{"component": …}`), `null` quand on ne sait pas. `run`, `parent` et `name` gardent leur sens.
   - La cible est liée à l'id du run : qui lance le graphe d'un outil ou d'une réaction choisit
     l'id, puis pose le service `run_target` `{run, target}` ; `emit_run_started` ne le lit que
     si l'id est le sien. Un sous-graphe, un graphe lancé ou un graphe de port héritent des
     services, mais ont leur propre id : ils ne s'annoncent pas sous le nom de leur parent.
   - Ce sont les noms du manifeste, pas l'en-tête `%% tool:` des fiches. Pour une réaction, les
     deux sont le même nom : le chargement l'exige, la clé est le curseur du réacteur.
   - Une recherche composée, un crochet, une semence : pas de cible.
   - `GraphTarget` vit avec les événements, et le backend le réexporte : un seul type pour ce
     dont on demande le graphe et ce qu'exécute un run.
8. **L'enveloppe commune est celle du flux** (accord avec la mémoire, pièce 1) :
   `{id, sujet, backend, at_ms, evenement}`. `at_ms` est l'instant du relais. L'événement garde
   sa forme : `kind`, `run` quand il en a un, ses champs à plat. Le bus ne sait pas de quel
   backend il est, et il garde ses `Event`.

9. **Un appel d'outil et une réaction publient leurs runs** (décision de c7, 19 octobre). Le
   témoin de la décision 7 a montré que ni l'un ni l'autre ne publiait rien : `build_services` ne
   posait pas `event_bus`, que seules les ingestions du catalogue posent (sur `catalog`). Le DAG
   vivant n'aurait rien eu à allumer, et cette page le supposait à tort.
   - Un appel par l'hôte publie sur `dataflow`, dans la cellule de l'appel. Le graphe de trace
     livré l'enregistre : c'est son rôle.
   - Un run de réaction publie sur un sujet à lui, `reactions`. **Le chargement refuse
     `%% on: reactions`**, et le montage (`Reactor::watch`) aussi, avec un message qui dit
     pourquoi. Une réaction ne peut donc pas se relancer sur ses propres runs, par construction.
   - Le flux de l'IDE relaie `reactions` (`SUJETS`).
   - La trace dit ce qu'un run exécute : un `RunStarted` à cible remplit le champ `tool` (pour un
     outil) et son détail finit par « — outil dis », « — réaction sonde » ou « — composant … ».
     Son témoin l'a montré : sans cela, elle enregistrait le run sans dire de quel outil.
   - Effet de bord voulu : les nœuds d'un appel d'outil ont maintenant le bus. Un
     `SendMessageNode` y fonctionne, une décision y dit ses alertes, et une suppression y émet son
     `EntityDeleted` sur `catalog`, comme dans une ingestion.

## Témoins

- `the_graph_of_a_tool_or_a_component_is_served_as_json`. Il couvre :
  - un outil : nom, type, ligne, config, schéma, résultat ;
  - un composant à graphe de clic : nœuds, lignes, arête, et un composant sans graphe ;
  - les trois refus, et la cible donnée deux fois ;
  - l'outil `graph`, absent sans le drapeau, puis le même JSON qu'en direct.
- `un_run_d_outil_puis_un_run_de_reaction_s_annoncent_par_leur_nom` : l'outil `dis` (fiche
  `autre_nom`) s'annonce `{"tool": "dis"}` sur `dataflow`, la réaction `sonde`
  `{"reaction": "sonde"}` sur `reactions`, et elle ne se relance pas (elle écoute `catalog`).
- `une_reaction_qui_ecoute_les_runs_des_reactions_est_refusee` : le refus au chargement.
- `la_trace_livree_enregistre_un_appel_d_outil_sans_se_relancer` : la fiche `trace` livrée,
  montée comme dans l'e2e, écrit le `RunStarted` de l'outil, et son nombre de runs ne bouge plus
  ensuite.
- Le témoin de l'aperçu à chaud joue la route sur un vrai hôte. Il demande le graphe de `usages`
  du gabarit `code` (chaque nœud a sa ligne et son schéma) et celui d'un composant sans graphe de
  clic, puis vérifie le refus d'une demande sans backend.

## La suite

Le dessin vivant dans la page de l'IDE. Il dessine ce JSON, s'abonne au sujet `dataflow` et
allume les nœuds par leur nom, au run reconnu.
