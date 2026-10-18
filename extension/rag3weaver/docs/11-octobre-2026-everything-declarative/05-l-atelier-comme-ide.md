# 05 — L'atelier comme IDE : la conception

*18 octobre 2026, session everything declarative. La priorité de Lucie : vision générale,
« L'atelier comme IDE des gabarits » ; buts §4 ter (b47dc6066). La première marche, précisée par
Lucie le même jour : l'explorateur et éditeur de code, avant le DAG vivant.*

Cette page dit ce qui existe, ce qui manque, et les lots dans l'ordre, chacun avec son témoin.
Les chemins de code sont relatifs à `extension/rag3weaver/`, au commit a81e398c4.

## 0. Les règles

- **La page n'est qu'une vue.** Tout ce qu'elle montre vient d'un outil que l'hôte sert déjà
  (gabarit `code`, mémoire longue) ou de son flux d'événements. Elle n'a pas d'état que l'hôte n'ait
  pas : pas de cache qui fasse foi, pas de brouillon côté navigateur.
- **Une vue, une URL.** Un fichier, un symbole, une recherche, une fiche, un voisinage, un DAG, un
  run : chacun s'ouvre dans sa fenêtre, se partage et se recharge. Plusieurs fenêtres sur le même
  hôte voient la même chose, tenues à jour par le même flux.
- **Le moteur reste dans l'hôte.** Le navigateur ne calcule rien qui fasse foi.

## 1. Ce qui existe

**L'atelier** (`src/atelier.rs`, `src/atelier/page.html`) est servi par `serve --atelier`
(`src/bin/rag3weaver-backend.rs:482`, `src/serve.rs:78`), sur la boucle locale, un serveur par
backend, sans jeton.
- **Routes** : `GET /_atelier` (la page), `/_atelier/events` (SSE), `/_atelier/state`,
  `/_atelier/graph?tool=&highlight=` (du mermaid en texte), `POST /_atelier/chat` (bloquant).
- **Le flux** ne porte que le chat (`token`, `tool_start`…) et les déclarations (`declared`,
  `refused`, `reloaded`). Aucun événement du bus n'y passe, ni `id:` ni `Last-Event-ID`.
- **La montre des déclarations** compare toutes les 500 ms et recharge à chaud.
- **La page** est en JS pur, en modules, sans bundler, avec mermaid@11 depuis jsdelivr. Elle n'a
  pas de routage par URL.

`ui/chat/` est un autre produit (le pont Python vers `rag3weaver-chat`). Il ne sert pas ici.

**L'hôte** (`src/daemon_host.rs:521-591`, `127.0.0.1:7970`) tient ouverts les backends de Lucie,
le code comme la mémoire.
- **Routes** : `/sante`, `/etat`, `/session`, `/backend/open|close`, `/describe`, `POST /call
  {manifest, name, arguments}`.
- **Accès** : jeton `Bearer` (`daemon.json`, 0600), aucun CORS, aucun flux. Le MCP est en stdio,
  client mince de l'hôte.

**Les outils du gabarit `code`** (`templates/backends/code/backend.json`) : `list_files`,
`read_file`, `grep_files`, `search_code`, `usages`, `impact`, `callees`, `schema`, `edit_file`,
`run_command`, `wait_output`, `index`, `estimate`.
- **Le JSON existe dans les nœuds, pas dans les outils.** Chaque nœud de rendu sait rendre du JSON
  riche : `format = json` (`code_nodes.rs:236`, `usage_nodes.rs:914`, `neighborhood_nodes.rs:1245`,
  `render_nodes.rs:1055`). Ce JSON porte le chemin, la ligne, l'uuid et les scopes d'un fichier avec
  leurs lignes. Mais aucun `.mmd` n'expose ce choix, et un argument inconnu est refusé : un appelant
  reçoit du markdown. Seul `search_code` rend déjà une structure : son `result` est la liste des
  résultats (uuid, `file_path`, lignes), et son markdown va à `presentation` (vu au témoin de A1).
- **Pas d'outil « symbole à cette position »** : `narrowest(scopes, line)` reste interne
  (`code_tools.rs:471`). `usages` et `impact` prennent un nom, pas un uuid.
- **L'arborescence** : `list_files` rend une liste plate par préfixe, avec `lines`, `indexed` et
  `stale`, au plus 2000 entrées.
- **L'édition** : `edit_file` écrit et réingère (`code_sync.rs:192`). La montre réindexe aussi les
  éditions venues d'ailleurs. Un backend n'a qu'une racine, confinée (`check_relative`), et
  `read_only` refuse toute écriture. **Un worktree, un manifeste.**

**La mémoire longue** (`templates/backends/memory/`).
- **Les fiches** : `Memory {claim, why, kind ∈ fact|preference|decision|pointer, origin, reach,
  state, revision, created_at, updated_at, reviewed_at}`.
- **Les liens** : `Subject` par `ANCHORED_TO`, une remplaçante par `REPLACES`, une `Ref {genre,
  valeur, dit_comme, …}` par `CITE`.
- **Citer un fichier** : une ref de forme normale `path:<chemin relatif>` (`schemas/ref_type.json`),
  pas une arête vers le `File` de la base de code. Ce sont deux bases séparées.
- **Les outils** : `recall` et `recall_subjects` (recherche structurée, JSON avec uuid),
  `get_memory`, `put_*`, `anchor`, `supersede_link`, `add_ref`, `create_ref_type`.

**Le moteur, pour le DAG vivant** :
- **La structure** : `GraphDefinition {nodes, edges}`, sérialisable (`dataflow/checkpoint.rs:304`),
  avec les arêtes nommées par port. Les ports (`PortDef`), leurs genres (`PortKind`) et leurs
  postures ne le sont pas. Les genres se résolvent par `DataflowGraph::resolved_kinds()`.
- **Sur le bus** : `RunStarted`, `RunFinished {cause, cancelled_by}`, `NodeRun` (un nœud, émis à
  sa fin), `Token`, `ToolCallStarted/Finished`.
- **Au runtime seulement** : `NodeStarted`, `NodeSkipped` et `NodeCancelled` existent mais restent
  locaux, sans run id. Les comptes par arête ne remontent qu'en fin de run, et les taps par élément
  sont sans run ni horodatage.
- **Hors d'atteinte** : un outil appelé passe par `execute_definition_as`, qui garde son runtime
  pour lui. L'annulation de l'extérieur ne vaut que pour les runs en fond (`background.rs:249`).

## 2. L'architecture proposée

**La page est servie par l'hôte, pas par `serve --atelier`.** La première marche lit deux backends
à la fois (le code d'un worktree et la mémoire longue), et seul l'hôte les tient ouverts ensemble,
pour toutes les sessions. Servir la page sur la même origine que `/call` évite le CORS. L'atelier
garde son rôle (le chat de déclaration d'un backend) : la marche 3 le rejoindra dans l'IDE.

- **`GET /ide/…`** : la page, un fichier HTML et des modules JS embarqués. Une seule page lit son
  URL et montre la vue qu'elle nomme (§3).
- **L'entrée par jeton** : `rag3weaver ide` imprime une URL `http://127.0.0.1:7970/ide/entrer?jeton=…`.
  L'hôte y pose un cookie `HttpOnly; SameSite=Strict` et redirige vers `/ide/`. Toutes les fenêtres
  du navigateur l'ont : rien n'est gardé par onglet. Le jeton ne passe pas dans l'URL des vues.
- **`POST /ide/call`** : le même que `/call`, authentifié par le cookie. La page n'a que cette
  porte vers les outils.
- **`GET /ide/flux?sujets=…`** : un SSE de l'hôte qui relaie le bus avec un `id:` croissant, et un
  tampon court pour rejouer depuis `Last-Event-ID`. Les sujets :
  - `files` (`FileChanged`) ;
  - la fin d'une réindexation et le retard ;
  - `dataflow` et `run.<id>` (`RunStarted`, `RunFinished`, `NodeRun`, `Token`).

  Une fenêtre s'abonne aux sujets de sa vue.

**Les bibliothèques** : en modules ES depuis jsdelivr, comme mermaid aujourd'hui, sans bundler.
- **CodeMirror 6** : la lecture et l'édition, la coloration.
- **Cytoscape.js** : le voisinage (le graphe d'un symbole ou d'une fiche).
- **elkjs** : la mise en page en couches avec ports, pour le DAG.

Pas de React pour la page elle-même : la marche 3 fera de React un *gabarit* compilé par l'hôte.
L'IDE ne doit pas dépendre de ce qu'il apprend à éditer.

## 3. La marche 1a — naviguer en lecture

**Les vues et leurs URL** :

| vue | URL | outils |
|---|---|---|
| l'arbre d'un dossier | `/ide/code/<backend>/arbre/<dossier>` | `list_files(path_prefix)` en JSON, plié en arbre par la page |
| un fichier | `/ide/code/<backend>/fichier/<chemin>#L<n>` | `read_file` en JSON (texte, scopes et leurs lignes, `stale`, `content_hash`) |
| un symbole | `/ide/code/<backend>/symbole/<nom>?path=<fichier>` | `usages`, `impact`, `callees` en JSON |
| une recherche | `/ide/code/<backend>/recherche?q=…&mode=…` | `search_code` (plein texte, vecteurs) |
| les fiches d'un chemin | `/ide/memoire/<backend>/chemin/<chemin>?q=&sujet=&genre=` | §3.2 |
| une fiche | `/ide/memoire/<backend>/fiche/<uuid>` | `get_memory`, ses sujets, ses refs |
| le voisinage | `/ide/graphe/<backend>/<symbole ou fiche>` | `impact` et `callees` en JSON ; sujets et refs d'une fiche |

**Les clics, comme dans un IDE.** Le fichier ouvert connaît ses scopes et leurs lignes (`read_file`
en JSON). Un clic sur un identifiant prend le scope le plus étroit qui contient la ligne, puis
ouvre le symbole par son nom et son fichier. Il n'y a pas besoin d'un outil « à cette position »
tant que les scopes sont rendus. Il en faudrait un pour les symboles d'un autre fichier par import :
c'est un lot plus tard, si le besoin se montre. Sur le symbole : définition, usages, impact,
appelés, liens (le crochet `links` de `search_code`), chaque ligne étant un lien vers
`fichier/<chemin>#L<n>`.

**Fluide pendant que la montre écrit** : c'est ce que donnent les lectures sans attente de l'arbre
principal (L4 à L6, un instantané par appel d'outil). La vue s'abonne à `files`. Un fichier changé
se recharge, ou se marque « changé sur le disque » s'il est en cours d'édition (§4).

### 3.1 Ce qu'il manque pour 1a

1. **Le JSON par l'outil.** Un paramètre `format` (défaut `markdown`) dans les `.mmd` de lecture
   (`read`, `list`, `grep`, `usages`, `impact`, `callees`) ; `search_code` n'en a pas besoin. C'est
   un changement de gabarit, pas de code. Le défaut reste le markdown, et la réponse d'un agent
   ne change pas. Sa fiche d'outil porte pourtant un paramètre de plus : un manifeste ne sait pas
   encore cacher un paramètre à un modèle.
2. **Le socle de l'hôte** : `/ide/…`, l'entrée par jeton, `/ide/call` et `/ide/flux` (§2).
3. **Les fiches d'un chemin** (§3.2).

### 3.2 Les fiches qui parlent d'un fichier, d'un dossier, d'un symbole

**Ce que la mémoire longue sait déjà rendre** :
- **L'adresse** : une ref de genre `path` et de valeur `path:<chemin relatif>` (et tout genre
  déclaré, par exemple un symbole, si on le crée par `create_ref_type`).
- **La date** de chaque fiche : `created_at`, `updated_at`, `reviewed_at`.
- **Son genre** (`kind`), **ses sujets** (`ANCHORED_TO`), **son état**.
- **La recherche structurée** (`recall`) a un `filter_condition` qui traverse une relation
  (`"Entity.champ"` devient un MATCH, `src/filter.rs:3-9`) et sait faire `StartsWith`.

« Les fiches qui citent ce chemin ou ce qui est dessous » s'écrirait donc
`{"Ref.genre": "path", "Ref.valeur": {"starts_with": "path:<dossier>/"}}`, avec
`{"Subject.name": …}` et `{"kind": …}` pour les filtres. **Ce n'est pas encore éprouvé** : c'est le
premier témoin du lot.

**Ce qui manque** :
- **L'ordre « les plus récentes d'abord »** : la recherche range par score. Il faut un ordre par
  champ dans `SearchOptions` (`order_by: updated_at desc`), ou un outil de liste.
- **Un outil de la mémoire nommé pour ce besoin** : `fiches_du_chemin(chemin, sous: bool, q?,
  sujet?, genre?, limit)`, un graphe dans `templates/backends/memory/graphs/`. Il compose le filtre
  ci-dessus, l'ordre, et la recherche plein texte sur ces seules fiches quand `q` est donné. Même
  forme pour `fiches_du_symbole`.
- **La racine d'un `path:`** (question pour Lucie) : la mémoire longue sert plusieurs dépôts, et
  `path:src/lib.rs` ne dit pas lequel. `origin.rs` nomme déjà un fichier par ses coordonnées git
  (dépôt, chemin). Faut-il que la forme normale de `path` les porte, ou qu'une fiche porte son
  dépôt ?

## 4. La marche 1b — éditer

1. **L'humain** : CodeMirror sur la vue fichier.
   - L'enregistrement passe par `edit_file` avec l'empreinte lue (`content_hash`). L'outil refuse si
     le fichier a changé depuis, et la page montre la différence au lieu d'écraser. C'est un
     paramètre `expected_hash` à ajouter à `edit_file`.
   - La montre réindexe, et les autres fenêtres le voient par `files`.
2. **L'agent, dans un worktree** : un backend `code` par worktree (le modèle existe : un manifeste,
   une racine). L'agent édite par les mêmes outils. La page a une vue « différence du worktree »
   (`/ide/code/<backend>/difference`), par un outil `diff` au graphe fixe (`git diff` sur la
   racine, en lecture). La fusion reste humaine, après relecture.

## 5. La marche 2 — le DAG vivant d'un outil

**Les vues** : `/ide/outil/<backend>/<outil>` (le DAG et son gabarit à côté, en lecture seule) et
`/ide/run/<run>` (le même DAG, vivant). Ce qu'il manque, dans l'ordre :
1. **Le graphe d'un outil en JSON** : les nœuds avec leurs ports d'entrée et de sortie (type, genre
   résolu, posture, `follows`, requis) et les arêtes. On rend `PortDef`, `PortKind` et `Posture`
   sérialisables, puis une fonction depuis `build_definition`, `resolved_kinds` et `schema_for`.
   Les sous-graphes restent des nœuds opaques au premier lot.
2. **Les événements de nœud sur le bus, avec leur run** : `NodeStarted`, `NodeSkipped {reason}` et
   `NodeCancelled {cause}`, publiés comme `NodeRun` sur `run.<id>`. Puis les comptes par arête,
   émis par paquets (toutes les 250 ms) et non par élément.
3. **La vue** : elkjs en couches, les ports sur les bords. Les états sont colorés (en attente, en
   cours, fini, sauté, annulé, en échec). Les arêtes portent leur compte et changent d'épaisseur
   avec le débit.
4. **L'annulation depuis la page** : pour un run en fond, `Registry::cancel` existe. Pour un outil
   appelé, il faut que l'appel s'inscrive au registre le temps de son run.

## 6. La marche 3 — l'agent qui édite des gabarits

Le chat de l'atelier (`declare`, `refused`, `reloaded`) entre dans l'IDE comme une vue. L'agent
propose, la déclaration valide ou refuse, et la page montre la différence du gabarit et son DAG
avant et après. Un composant React est un gabarit compilé par l'hôte et rechargé à chaud (la graine
des applications empaquetées). Cette marche se conçoit après 1 et 2.

## 7. Les lots, dans l'ordre

| lot | contenu | témoin |
|---|---|---|
| A1 | le JSON des outils de lecture : `format` dans six `.mmd` | chaque outil en `format=json` rend chemin, ligne et uuid ; sans le paramètre, la réponse entière est celle de `format=markdown` ; `search_code` rend déjà sa liste structurée |
| A2 | le socle de l'hôte : `/ide/` statique, entrée par jeton (cookie), `/ide/call`, `/ide/flux` (relais du bus, `id:`, rejeu) | deux abonnés reçoivent le même `FileChanged` avec le même id ; un abonné reconnecté avec `Last-Event-ID` reçoit ce qu'il a manqué ; sans cookie, 401 |
| A3 | la page 1a : arbre, fichier, symboles cliquables, recherche, une URL par vue | un test de contrat : chaque appel que la page fait existe dans `/describe`, avec ses paramètres ; puis la relecture de Lucie |
| A4 | les fiches d'un chemin, d'un dossier, d'un symbole : le filtre éprouvé, l'ordre par date, `fiches_du_chemin` | une fiche qui cite `path:src/a/b.rs` sort pour `src/a/b.rs` et pour `src/a/`, pas pour `src/ab/` ; la plus récente vient d'abord ; le filtre de sujet et de genre retire les autres |
| A5 | le voisinage en graphe (Cytoscape) | la vue d'un symbole connu montre ses appelants et appelés, chacun lien vers son fichier |
| B1 | l'édition humaine : CodeMirror, `expected_hash` | une écriture sur une empreinte périmée est refusée et rien n'est écrit ; la fenêtre voisine voit le changement par le flux |
| B2 | l'agent dans un worktree, la vue différence | l'agent édite dans son worktree ; la différence rendue est celle de `git diff` |
| C1 | le graphe d'un outil en JSON, avec ses ports | pour un outil connu, ports et genres égaux à ceux du runtime (`resolved_kinds`) |
| C2 | `NodeStarted`, `NodeSkipped` et `NodeCancelled` sur le bus, les comptes par arête | un run de trois nœuds dont un sauté émet, dans l'ordre, ses événements avec le run id |
| C3 | la vue DAG vivante | relecture de Lucie sur un run réel |
| C4 | l'annulation depuis la page | un run annulé par la page finit `cancelled_by` avec la cause |

**La première chose livrable** : A1, puis A2, puis A3. L'arbre, le fichier et les symboles
cliquables, sur le worktree de Lucie, dans plusieurs fenêtres. A4 suit aussitôt.

## 8. Les questions pour Lucie

1. **La racine d'un `path:`** dans la mémoire longue, qui sert plusieurs dépôts (§3.2).
2. **L'IDE servi par l'hôte** (une porte, le jeton par cookie), plutôt que par `serve --atelier` ?
3. **Les bibliothèques depuis jsdelivr**, comme mermaid aujourd'hui, ou embarquées dans le binaire
   pour un poste hors ligne ?
