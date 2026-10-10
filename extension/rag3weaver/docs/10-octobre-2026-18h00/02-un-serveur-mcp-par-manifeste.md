# Un serveur MCP par manifeste — la page avant le code

10 octobre 2026, session mémoire longue. Chantier H, demandé par Lucie : « des
outils MCP sur ce projet de code pour chercher dedans via rag3weaver », puis
« une mémoire long terme accessible à Claude », puis « limitée à ce projet, ou
globale ». Une page avant le code, et trois corrections de cadrage obtenues par
lecture.

## La bonne nouvelle : la moitié est déjà écrite

**`rag3weaver-backend` est déjà un serveur sur stdio.** Boucle ligne à ligne sur
`stdin`, JSON par ligne, `{"op":"describe"}` et `{"op":"call","name","arguments"}`,
une réponse JSON par ligne. Il porte déjà tout le cycle de vie qu'un serveur MCP
devra porter : le choix du tampon, le service d'embarquement si le backend en a
besoin, l'ouverture de la base, `shutdown` qui attend le point de reprise, et le
protocole de réouverture (`must_reopen` → `EXIT_MUST_REOPEN`).

**Et `describe()` rend déjà la forme de `tools/list`.** Pour chaque outil
déclaré il émet `{"name", "description", "inputSchema"}` — le nom exact que MCP
attend pour le schéma d'entrée. Les schémas viennent de `tool_schemas`, c'est-à-
dire de `NodeSchema`/`ConfigParam` par `tools.rs`, la même source que les formes
OpenAI et Anthropic.

**Conclusion de cadrage, et c'est ma première recommandation** : une
**sous-commande** `rag3weaver-backend mcp`, pas un binaire neuf. Un second
binaire dupliquerait l'ouverture, l'embarqueur, le tampon et surtout le
protocole de réouverture — soit un second endroit où ce protocole devra être
maintenu juste, et c'est le genre de duplication qu'on paie six mois plus tard.
`src/mcp.rs` porte le protocole ; le bin lui passe un backend déjà ouvert.

## La correspondance schéma → outil MCP

Rien à inventer, et c'est le point : MCP demande exactement ce que `tools.rs`
produit déjà.

| MCP | d'où ça vient |
|---|---|
| `tools[].name` | la clé de l'outil au manifeste, déjà validée comme identifiant |
| `tools[].description` | `%% description:` de la fiche, ou l'override du manifeste |
| `tools[].inputSchema` | `tool_schemas[name]` — un JSON Schema `type: object` |
| `properties`, `required` | `params_object_schema` depuis les `ConfigParam` |
| `enum` d'un paramètre | `Choices` résolu contre le catalogue au rendu |

Deux réserves honnêtes :

- **`ConfigParamType::Json` devient un objet libre** — c'est la dette déjà
  nommée dans `tools.rs`. Un client MCP affichera donc « object » sans guide
  pour les paramètres de filtre. Supportable pour chercher, gênant pour écrire ;
- **les liaisons du manifeste sont retirées des propriétés** avant publication,
  comme pour les autres transports. C'est volontaire : un appelant ne doit pas
  pouvoir réécrire une liaison, et c'est déjà refusé par son nom
  (« cannot override binding »).

## Ce qu'on expose : tout outil déclaré, n'importe quel manifeste

Un serveur = **un manifeste**, quel qu'il soit. C'est la même mécanique pour les
deux cas de Lucie, et aucune ligne ne les distingue :

- `templates/backends/code` → chercher dans un dépôt (`search`, `usages`, …) ;
- `templates/backends/memory` → `put_memory`, `get_memory`, `put_subject`,
  `recall`, `recall_subjects`, `add_ref`, `create_ref_type`.

Les sept outils de mémoire deviennent des outils MCP **sans une ligne de plus**,
puisque la liste vient des schémas. C'est l'argument de cette architecture, et
c'est aussi son épreuve : si un gabarit demandait du code dans `mcp.rs`, le
principe serait déjà cassé.

`tools/call` passe par `Backend::call`, donc par `run_tool` : le même chemin que
l'agent, avec la politique de nœuds de l'outil, le bac à sable du workspace et
le rapport d'exécution. Ce qui est refusé l'est par son nom — un outil absent du
manifeste, un chemin hors du workspace —, et le refus remonte tel quel dans le
`content` de la réponse MCP plutôt que d'être réécrit.

## Crate ou JSON-RPC à la main : à la main, et voici pourquoi

`rmcp` est le SDK officiel (3.1.0 aujourd'hui), avec un transport stdio tout
fait ; `mcpkit` est une alternative à macros. Et **`tokio` est déjà une
dépendance dure** de la crate (ligne 38 du `Cargo.toml`) — donc l'argument
habituel « un SDK amène un runtime » ne tient pas ici. Je l'ai vérifié avant de
l'écrire parce que c'est l'argument que j'allais donner.

Je recommande quand même **le JSON-RPC à la main**, pour une raison qui vient de
notre code et non du goût :

**le catalogue est `Arc<Mutex<Catalog>>`, et le fan-out de cellules bascule un
état partagé.** `Catalog::rechercher` le dit lui-même : chercher dans plusieurs
cellules se fait « autour du graphe, en changeant la cellule du catalogue le
temps de l'appel », et son commentaire nomme la limite — « ce n'est pas plus sûr
qu'avant face à une recherche concurrente sur le même catalogue ». Un serveur
synchrone qui traite **un appel à la fois** rend cette limite inatteignable par
construction. Un serveur async la rendrait atteignable, et il faudrait sérialiser
à la main ce que la boucle actuelle sérialise gratuitement.

Ce qu'on accepte en échange : **la conformité devient la nôtre**. `initialize`,
la négociation de `protocolVersion`, les `capabilities`, les codes d'erreur
JSON-RPC, la notification `notifications/initialized` — quatre méthodes et une
notification, sur du JSON délimité par retours à la ligne (MCP sur stdio, pas
d'en-têtes `Content-Length` comme LSP). C'est court, et le risque réel n'est pas
la longueur mais la **dérive** : MCP bouge, et Claude Code est un client strict.
Deux parades : tout le protocole dans `src/mcp.rs` et nulle part ailleurs, pour
qu'un passage à `rmcp` reste local ; et un test qui **parle le protocole** sur
stdio contre le backend de code de ce dépôt, qui est la seule façon de savoir
qu'on est conforme.

## La portée : la cellule existe, et « globale » n'est pas un chantier

`Scope { org, project }` existe, avec ses colonnes système `_org` / `_project` sur
toute table de données, ses index à part, et le fan-out `SearchOptions.scopes`
qui fusionne par rang (RRF) — **branché**, pas déclaré : `catalog.rs:8044`.

Donc, par lecture :

- **« ce projet »** = la cellule `(org, project)` du projet ;
- **« globale »** = une cellule convenue, `(org, "global")` par exemple ;
- **« les deux d'un coup »** = `options.scopes = [projet, global]`, projet
  d'abord, ce que le fan-out fait déjà.

Et sa limite, dite par le code : les scores ne sont **pas comparables** entre
cellules (IDF distincts), ce que `meta.warnings` rappelle — d'où la fusion par
rang et non par score. Un `recall` à deux cellules rendra donc un ordre
raisonnable, pas un score interprétable. À dire dans la description de l'outil,
sinon un modèle lira les scores comme s'ils voulaient dire quelque chose.

Le serveur reçoit la portée du manifeste, et la ligne de commande la surpasse —
c'est la forme habituelle ici (`RAG3DB_BUFFER_POOL_SIZE`, `RAG3WEAVER_EMBED_SERVICE`).

## Une base à cellules, ou une base de mémoire globale à part ?

La question ouverte de la vision générale §8. **Je recommande une seule base à
cellules**, et pas par élégance :

- le fan-out multi-cellules **existe et est branché** ; le fan-out multi-**bases**
  n'existe pas du tout. Une mémoire globale dans une base séparée demanderait
  d'ouvrir deux `Database`, de fusionner deux réponses hors du catalogue, et de
  dupliquer l'embarqueur. C'est un chantier, contre un paramètre ;
- un `Ref` est `hashsafe` sur `(genre, valeur)` : dans **une** base, deux
  projets qui citent la même chose convergent sur le même nœud. Dans deux bases,
  ils ont deux nœuds que rien ne rapproche — et c'est précisément l'inverse de
  ce qu'une mémoire globale doit faire ;
- un seul écrivain par base est la règle du moteur. Une base commune à plusieurs
  projets veut dire **un** serveur MCP pour la mémoire globale, partagé, pas un
  par session. C'est le vrai coût de mon choix, et il est structurant.

**Le cas qui me ferait changer d'avis est celui que Lucie a nommé** : des projets
sur des disques différents. Une base commune suppose un chemin accessible à
tous ; si ce n'est pas vrai, aucune élégance ne le rend vrai. Je proposerais
alors non pas deux bases fusionnées à la lecture — ça, c'est le chantier —, mais
**la mémoire globale servie par un serveur MCP à part**, que chaque session
déclare en plus du sien dans son `.mcp.json`. Deux serveurs, deux jeux d'outils
nommés, et c'est le client qui choisit où il écrit. Ça ne demande rien de neuf.

## La session

Un processus par client, l'état en base, et rien en mémoire entre deux appels —
c'est déjà le modèle du bin. Deux conséquences à écrire quelque part :

- **`must_reopen` doit devenir un refus MCP nommé**, pas une sortie silencieuse.
  Aujourd'hui le bin répond puis `exit(EXIT_MUST_REOPEN)` ; un client MCP verrait
  son serveur disparaître. Il faut un message d'erreur qui dit quoi faire, puis
  l'arrêt ;
- **plusieurs sessions Claude Code sur le même projet** = plusieurs serveurs sur
  la même base = plusieurs écrivains, ce que le moteur refuse. Avec un backend de
  code en lecture seule, c'est sans objet ; avec la mémoire, non. À trancher avec
  Lucie, et ma préférence est un serveur par base, pas par session.

## Ce qu'on ne fait pas

`resources` et `prompts` : plus tard, et déclarés absents dans les `capabilities`
plutôt que silencieux. Pas de `sampling`, pas de `roots`. Pas de transport HTTP.

## L'épreuve réelle, trois cas

1. **chercher dans ce dépôt** : `.mcp.json` sur le backend `code`, une session
   qui demande les `usages` d'une fonction de rag3db ;
2. **se souvenir d'une session à l'autre** : backend `memory`, une session qui
   pose une mémoire, une autre qui la retrouve. C'est le cas que Lucie veut
   voir — son « classeur » ;
3. **deux projets, une mémoire globale** : deux sessions, deux cellules, un
   `recall` qui rend le projet d'abord puis le commun.

Le troisième est le seul qui éprouve la portée, et le seul qui dira si la fusion
par rang entre cellules rend un ordre utile ou une soupe. Je ne le saurai pas en
le lisant.
