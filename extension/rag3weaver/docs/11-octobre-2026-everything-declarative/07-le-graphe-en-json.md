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
3. **Le nom d'un nœud est celui des `NodeRun`** du sujet `dataflow`. C'est ce qui allumera un
   nœud dans le dessin vivant. Les événements ne portent pas le nom de l'outil (`RunStarted.name`
   est la liste des nœuds) : un dessin reconnaîtra son run par cette liste. Le dire dans
   l'événement est un changement du moteur, laissé au lot du dessin s'il le faut.
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

## Témoins

- `the_graph_of_a_tool_or_a_component_is_served_as_json`. Il couvre :
  - un outil : nom, type, ligne, config, schéma, résultat ;
  - un composant à graphe de clic : nœuds, lignes, arête, et un composant sans graphe ;
  - les trois refus, et la cible donnée deux fois ;
  - l'outil `graph`, absent sans le drapeau, puis le même JSON qu'en direct.
- Le témoin de l'aperçu à chaud joue la route sur un vrai hôte. Il demande le graphe de `usages`
  du gabarit `code` (chaque nœud a sa ligne et son schéma) et celui d'un composant sans graphe de
  clic, puis vérifie le refus d'une demande sans backend.

## La suite

Le dessin vivant dans la page de l'IDE. Il dessine ce JSON, s'abonne au sujet `dataflow` et
allume les nœuds par leur nom, au run reconnu.
