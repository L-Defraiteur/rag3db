# Ce qui manque pour que tout logiciel se débrouille avec des DAG

23 novembre 2026, session everything declarative. Question de Lucie : « qu'est-ce qui nous manquerait d'autre ? des
webhooks ? des sécurités en gabarits pour recevoir des events, genre HMAC, ou pour appeler une API distante ? quels
autres nœuds seraient nécessaires pour que tout software se débrouille avec des DAG ? ». c7 a donné une liste par
familles ; cette page la confronte au registre des nœuds de `master` du 23 novembre (`register_builtins`), sans rien
coder.

La règle qui juge chaque ligne est celle de Lucie : **un nœud natif ne fait qu'une chose, et n'est jamais une
fonction composite**. Ce qui est composite se déclare en graphe à partir de primitives.

## Ce qui existe, ce qui manque, ce qu'il ne faut pas faire

### 1. Les entrées du dehors

| Besoin | Existe déjà | Manque | À ne pas faire |
|---|---|---|---|
| Recevoir un webhook | les routes de l'hôte (`ide.rs`), mais aucune n'est déclarable par un graphe | **un `listen` sur un canal alimenté par une route déclarée** (`/hooks/<nom>`), avec sa vérification en nœud scellé à part | un nœud « webhook » qui vérifie, analyse et aiguille à la fois |
| Vérifier une signature entrante (HMAC, horodatage, rejeu) | rien | **`VerifySignature`** (scellé) : l'algorithme, l'en-tête, la fenêtre d'horodatage, et le rejeu refusé par une clé d'idempotence (famille 4) | lire le secret dans le graphe, ou laisser passer un message non vérifié vers l'aval |
| Minuterie, calendrier | rien | `listen` à échéance (page 02), et un canal `clock` alimenté par l'hôte selon une expression de calendrier déclarée | une boucle d'attente dans un nœud |
| Surveiller des fichiers | la montre de l'index (`watch.rs`), interne | un canal `files` alimenté par la montre, écouté par `listen` | un nœud qui sonde le disque |
| Consommer une file | le bus de l'hôte, la boîte des équipiers | les canaux durables de la page 02 en sont la file ; un consommateur, c'est `listen` avec un accusé | une file à part, à côté du bus |

### 2. Les sorties

| Besoin | Existe déjà | Manque | À ne pas faire |
|---|---|---|---|
| Requête HTTP | aucun nœud ; des appels en dur (`web_search.rs`, les clients de modèle) | **`HttpRequest`** : méthode, adresse, corps, délai, et l'en-tête d'authentification **par référence de secret** (famille 3) ; il rend `{status, headers, body}` | y mettre la reprise, la pagination ou l'analyse du corps : ce sont d'autres nœuds |
| Reprise avec recul | les clients de modèle ont la leur, en dur | la reprise en graphe : l'erreur comme sortie (famille 5) et la ré-entrée de la page 02, avec un `listen` à échéance pour le recul | un paramètre « retries » sur chaque nœud |
| Pagination | rien | une ré-entrée tant que la réponse porte un curseur : une condition, puis un `emit` | un nœud qui suit les pages en interne |
| Envoyer un message | `SendMessageNode` (le bus et les boîtes), `emit` (page 02) | l'envoi vers le dehors est un `HttpRequest` | un nœud par fournisseur de messagerie |

### 3. Les secrets et l'identité, en primitives scellées

| Besoin | Existe déjà | Manque | À ne pas faire |
|---|---|---|---|
| Référencer un secret | `secret_from_env` (une variable nommée), `.vault` lu par le code | **`SecretRef(name)`** : une poignée opaque que seuls les nœuds scellés savent ouvrir ; elle ne se sérialise pas, ne se trace pas et ne s'affiche pas | **une valeur de secret dans un graphe, une trace ou un port lisible** |
| Signer, vérifier (HMAC, JWT) | rien | `Sign` et `VerifySignature`, scellés, qui prennent une `SecretRef` | un nœud scripté qui manipule la clé |
| Échanger un jeton (OAuth, compte de service) | `gcp_auth::TokenSource`, en dur pour Vertex | `TokenExchange`, scellé, qui rend une `SecretRef` fraîche | garder le jeton dans l'état du graphe |

Ces nœuds sont **scellés** au sens de la page 02 : aucune surcouche ne les sélectionne, et la politique fermée des types
de nœuds exige qu'ils soient ceux du crate.

### 4. L'état durable

| Besoin | Existe déjà | Manque | À ne pas faire |
|---|---|---|---|
| Lire, écrire une clé | les enregistrements du catalogue (`InsertRecord`, `UpdateRecord`, `DeleteRecord`, `Cypher`), le checkpoint d'un graphe | **`KvGet` et `KvPut`**, sur un magasin clé-valeur porté par une tâche, un run ou le poste | se servir d'un fichier écrit par un script |
| Clé d'idempotence | rien | **`Once(key, ttl)`** : rend « nouveau » ou « déjà vu » | dédoublonner dans chaque nœud |
| Verrou, sémaphore | des verrous en dur (le registre, les cartes) | **`Lock(key)`** et **`Semaphore(key, n)`**, avec leur relâche | un verrou caché dans un nœud composite |
| Compteur, limiteur de débit | rien de déclaré (le plafond des bilans est en dur) | **`Counter`** et **`RateLimit(key, n, per)`** | confondre le limiteur et le plafond des dépenses, qui reste une garde scellée |
| Cache | rien de déclaré | `KvGet` et `KvPut` avec un `ttl` suffisent | un nœud « cache » qui enveloppe un autre nœud |

### 5. Le contrôle

| Besoin | Existe déjà | Manque | À ne pas faire |
|---|---|---|---|
| Condition, aiguillage | `Route`, `Decide` ; `Not`, `All`, `Any` et `FirstMatch` (branche `ed-guard-chain`) | les conditions par un modèle complètes (contexte en tableau, `unsure`, page 02) | — |
| Éventail et jointure | `ForEachNode` (un sous-graphe par élément), `Collect` (rassemble des entrées nommées dans un ordre déclaré), `SpawnNode` (lance en fond) | un **`Join`** qui attend des lancements en fond par leur référence | — |
| Attente, délai maximal | rien de déclaré | `listen` à échéance (page 02) | une attente active dans un nœud |
| **L'erreur comme sortie typée** | aujourd'hui, une erreur de nœud arrête le graphe | **un port `error` sur tout nœud**, de type `{kind, message, retryable}`, qu'on branche comme une autre sortie (le try/catch par port). Non branché, l'erreur arrête le graphe comme aujourd'hui | avaler l'erreur par défaut |
| Reprise | rien de déclaré | le port `error`, une condition sur `retryable`, et la ré-entrée de la page 02 | — |
| Compensation | rien | une convention plutôt qu'un nœud : chaque effet de bord déclare son inverse (`undo`), et une branche d'erreur appelle les inverses dans l'ordre contraire | un moteur de saga caché |
| Décision humaine | les cartes d'accord (`card.rs`), en dur | **`AskHuman(question, choices, timeout)`** : ouvre une carte et rend la réponse signée `by.kind == "human"`. La signature reste une garde scellée | accepter une réponse qui ne porte pas la signature humaine |

### 6. Les données

| Besoin | Existe déjà | Manque | À ne pas faire |
|---|---|---|---|
| Transformer | `RhaiNode`, les nœuds scriptés en TypeScript et en WebAssembly | — | — |
| Valider contre un schéma aux frontières | `ValidateNode` (sur le catalogue), la sortie structurée de `LlmNode` | **`ValidateSchema(schema)`** sur n'importe quel élément JSON, avec un port `error` | valider au milieu de chaque nœud |
| Analyser, formater | les nœuds scriptés | un nœud scripté livré par format suffit (JSON, CSV, XML) | un analyseur natif par format sans besoin de vitesse |
| Rendre un gabarit | `PlaceTemplateNode` (les gabarits de fichiers), les invites de `Decide` | **`RenderTemplate`** (un texte à trous sur un élément) | — |
| Diff et patch | `EditFileNode` (sur un fichier) | `Diff` et `Patch` sur du texte | — |

### 7. Les modèles

| Besoin | Existe déjà | Manque |
|---|---|---|
| Appel | `LlmNode` (compté au registre des dépenses) | — |
| Décision | `Decide` | le contexte en tableau et la sortie `unsure` (page 02) |
| Embarquement | `EmbedNode` | — |
| Recherche | `SearchSource`, `VectorSearch`, `BM25`, `SparseSearch`, `FuseResults`, `Rerank`, `ExactName`… | — |
| OCR | `OcrNode` | — |

### 8. L'exécution

| Besoin | Existe déjà | Manque | À ne pas faire |
|---|---|---|---|
| Commande en bac | `RunCommandNode`, `WaitOutputNode`, `TailNode`, sous la porte des commandes | — | contourner la porte |
| Sous-graphe | `GraphNode`, le sous-graphe cité (`ForEachNode`), `ComposeNode` | la surcouche (page 02) | — |
| Lancer un agent ou une session | `SpawnNode`, `InvokeToolNode` ; l'équipier lancé par l'hôte | `StartAgent(template, request)`, qui rend la référence d'un run d'agent | — |

### 9. L'observabilité

| Besoin | Existe déjà | Manque |
|---|---|---|
| Trace de chaque nœud | la trace du moteur, `TraceSinkNode`, `EventSourceNode` | la trace écrite d'office pour tout graphe déclaré, sans nœud à poser |
| Mesure | `ctx.metric` dans chaque nœud | l'agrégation par type de nœud, lisible par l'IDE |
| Audit | les lignes de boîte, `decisions.jsonl` des cartes | un journal d'audit commun aux nœuds scellés (qui a ouvert quel secret, quelle commande a été accordée), en lecture seule |

### 10. Le test d'un graphe

| Besoin | Existe déjà | Manque |
|---|---|---|
| Un témoin déclaratif | les témoins sont en Rust | **un fichier de cas à côté du mmd** (`<graphe>.cases.json`) : des entrées, les réponses simulées des nœuds de bord (modèle, HTTP, commande, horloge), les sorties attendues. Un lanceur générique les joue en bac. Un graphe se prouve alors sans Rust, et le pousseur peut le rejouer comme une suite |

## Les vrais trous

Ce qui manque et que rien ne remplace aujourd'hui :
1. **l'erreur comme sortie typée** (famille 5) : sans elle, ni reprise, ni pagination robuste, ni compensation ne se
   déclarent ;
2. **`emit` et `listen` sur des canaux durables** (page 02), dont dépendent les entrées du dehors, la minuterie, l'attente
   et les boucles ;
3. **les secrets en poignée opaque** (`SecretRef`), avec `Sign`, `VerifySignature` et `TokenExchange` scellés : sans
   eux, aucun webhook ni aucune API distante ne se déclare sans fuite ;
4. **`HttpRequest`** ;
5. **l'état durable en primitives** (`KvGet`, `KvPut`, `Once`, `Lock`, `RateLimit`) ;
6. **le témoin déclaratif d'un graphe** : sans lui, chaque graphe nouveau demande encore un témoin en Rust.

Le reste existe déjà, ou se compose de ces primitives.

## L'ordre proposé

1. **L'erreur comme sortie typée**, sur le moteur : elle touche tous les nœuds sans en ajouter un.
2. **Les canaux durables, `emit` et `listen`** (déjà en tête de la phase 1).
3. **Le témoin déclaratif d'un graphe**, pour que tout ce qui suit se prouve sans Rust.
4. **`SecretRef`, puis `HttpRequest`**, puis `Sign`, `VerifySignature` et `TokenExchange` : l'API distante, puis le
   webhook entrant (une route déclarée vers un canal, puis `VerifySignature`, puis `Once`).
5. **L'état durable** : `KvGet` et `KvPut`, `Once`, `Lock`, `RateLimit`.
6. **`AskHuman`, `Join`, `StartAgent`, `ValidateSchema`, `RenderTemplate`, `Diff` et `Patch`**, dans l'ordre où un graphe
   réel les demande.
