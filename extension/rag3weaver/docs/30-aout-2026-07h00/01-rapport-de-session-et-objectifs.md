# Rapport de session — 29 au 30 août 2026

Une session longue, en deux temps : une soirée à deux, puis une nuit en
autonomie. ~45 commits, de `0ba1d0dc2` à `69cde71e5`.

## 1. Ce qui a été livré

### Les démons — savoir lancer quelque chose

Le moteur savait charger un modèle et exécuter un graphe ; il ne savait **rien
lancer**.

- **`src/serveur.rs`** — démarrer un serveur et savoir s'il est déjà là. Trois
  états, pas deux : `Repond`, `Absent`, `Occupe` (quelqu'un répond, ce n'est
  pas lui — on ne le tue pas, on le dit).
- **`src/daemon/embeddings.rs`** — un modèle chargé une fois, servi à
  plusieurs. **4,22 s de chargement contre 1,16 ms d'attachement.**
- **`src/daemon/db.rs`** (rag3daemon) — la base qu'un seul processus peut
  ouvrir, mise derrière une adresse. Mesuré : deux processus, quarante travaux,
  **20/20**, aucun doublon.
- **`--exposer`** — un argument de ligne de commande ne suffit plus à ouvrir un
  port sur le monde.

### Le régime — rendre la machine utilisable

`RAG3WEAVER_REGIME=confort` : carte la moins chargée, rythme 60 %, rafales de
2 048 caractères. Mesuré : carte d'affichage à **0 %** pendant toute une passe.

Et deux leviers qui existaient depuis le 27 août n'étaient branchés que sur
l'ingestion — hissés dans `embedder.rs`, ils valent maintenant partout.

### La recherche — deux défauts silencieux

- **`BM25Mode::Auto`** : une phrase se pèse (BM25 terme à terme), un
  identifiant se contient. Trois questions françaises : **0/3 → 3/3**.
- **Les avertissements remontent** jusqu'à la fiche, y compris à zéro résultat.
  `BM25SearchNode` n'avait aucun port de sortie pour eux.

### Le catalogue de gabarits — les quatre verbes

`place` (poser), `adopt` (le catalogue apprend du projet, sous
`.rafg3weaver/templates/`), et `schema` (la carte du graphe, en Mermaid).

### L'agent de code — la boucle fermée

- **`src/commande.rs`** — la porte : trois modes (`auto` par défaut), un
  verdict à quatre morceaux (décision, portée, fondement, faits), une
  `Autorisee` dont le champ privé rend l'exécution sans verdict **impossible
  par construction**.
- **`codeparsers/src/shell.rs`** — réduire une ligne en argv, ou refuser en le
  nommant. `git status && rm -rf ~` rend **deux** commandes, toutes deux jugées.
- **`run`, `run_bg`, `wait`** — le verbe qui manquait. `run_bg` est le premier
  usager du mécanisme asynchrone, complet depuis le 26 août et que pas une
  fiche ne déclarait.
- **Le journal** : la sortie entière va dans un fichier, l'aperçu dit ce qu'il
  ne montre pas.

### Trouvé par les modèles eux-mêmes

- **Gemini** a trouvé un défaut bloquant : `Agent::new` calculait les fiches
  **une fois**, donc un agent qui posait une entité ne pouvait plus la
  chercher — et la liste contraignant le décodage, il ne pouvait pas même
  prononcer son nom.
- **Les deux modèles** ont montré que notre dialecte Mermaid était plus étroit
  que Mermaid : `a -- p --> b` refusé, nœuds déclarés dans la ligne d'arête
  ignorés.

## 2. Les chiffres

| | |
|---|---|
| tests unitaires | **952**, 0 échec, trois jeux de features |
| `codeparsers` | ~90, 0 échec |
| outils offerts | **11** (était 5) |
| lecture/écriture Mermaid | 5/5 et ✓ sur Gemini **et** Qwen3-Coder-30B local |

## 3. Les objectifs immédiats, dans l'ordre

### 1. Prouver le chemin Postgres — **bloquant pour tout le reste**

`PostgresDialect` fait 944 lignes, compile, et n'a **aucun test E2E**. Or la
boucle étrange qui ne sert que kuzu ne sert pas les projets réels : il faudra
proposer kuzu, Postgres, peut-être Neo4j. Docker est disponible ; il faut une
image `pgvector`, un `setup_statements` (`CREATE EXTENSION vector`,
`CREATE SCHEMA rag3weaver`) et le parcours qu'on connaît : créer, ingérer,
chercher, comparer.

**C'est le préalable des étages 2 et 3 du [doc vision 13](../vision_roadmap_09_2026/13-avaler-une-base-existante.md).**

### 2. L'agent qui écrit son propre outil

Bloqué sur un point d'architecture identifié mais non résolu : `GraphToolBox`
tient `&'a GraphToolRegistry` — une référence **immuable** — alors que
`attach` demande `&mut self`. Il faut une **seconde couche**, mutable, pour les
outils écrits en session :

- `Arc<RwLock<GraphToolRegistry>>` à côté du registre statique ;
- `tool_defs()` fusionne les deux ;
- **les fournis l'emportent** : un agent ne doit pas pouvoir masquer `run` ou
  `edit` par sa propre version.

Le reste est prêt : les deux modèles écrivent du Mermaid que notre parseur
relit, et `tool_defs()` se relit à chaque tour depuis le correctif de Gemini —
donc un outil écrit apparaît **au tour suivant**. C'est la boucle qui se ferme.

### 3. Les douze affichages faits à la main

`rendre<T: Serialize>` existe et `schema` passe par là. Restent : `list`,
`read`, `grep`, `edit` (4 dans `code_nodes`), `place`, `adopt` (3 dans
`template_nodes`), `run`, `wait` (5 dans `run_nodes`).

**La standardisation porte sur le mécanisme, pas sur le format** : un gabarit
nommé plutôt qu'un `format!`, et chacun choisit sa forme. Mermaid pour un
graphe, un tableau pour une liste, du texte pour une sortie de commande.

### 4. La couverture de `codeparsers`

Cahier des charges écrit en trois documents
(`docs/30-aout-2026-06h00/01, 02, 03`), destiné à une autre session.
L'invariant : **l'union des scopes couvre le fichier entier**, et ce qu'on n'a
pas compris se dit au lieu d'être jeté.

## 4. Les objectifs futurs, et leur ordre

1. **La normalisation xlsx** avant l'ingesteur de base étrangère — Lucie :
   *« ça nous inspirera sûrement »*. La question « quelles colonnes portent du
   texte cherchable ? » est la même des deux côtés, et le pipeline tableur la
   rencontrera en premier, sur un terrain plus simple.
2. **L'ingesteur de base étrangère** ([doc 13](../vision_roadmap_09_2026/13-avaler-une-base-existante.md)) —
   lire un schéma, proposer un graphe, en séparant `Declaree` / `Deduite` /
   `Devinee`.
3. **Les quatre rôles** ([docs 09 et 10](../vision_roadmap_09_2026/09-trois-roles-et-une-seule-main.md)) —
   vision, design, contexte, code. La plomberie existe depuis le 26 août ; ce
   qui manque, ce sont les rôles et ce que chacun possède.
4. **La file de travaux** (issue 03 §8) — délibérément repoussée : quatre
   champs de bookkeeping dans un processus unique, et aucun consommateur
   aujourd'hui.
5. **La recherche web** — Gemini grounded via Vertex, plus un repli sans
   Gemini, plus `fetch`. Première fois qu'un agent sortirait de la machine :
   surface à revoir avant d'écrire.
6. **LadybugDB** — la continuation MIT de Kuzu, v0.19.1, poussée
   quotidiennement. Nous sommes sur Kuzu v0.11.2.2 ; nos modifications du cœur
   font **26 fichiers, 520 insertions**. Le repérage de fusion n'est pas fait.

## 5. Ce qui reste ouvert et non décidé

- Où vit la configuration des familles de commandes `Toujours`.
- Comment demander à l'humain quand il n'y a pas de terminal.
- `Absorb::DernieresManches` — la coupe de mémoire à la manche de design.
- L'arbitre GPU **entre processus** : le verrou du démon ne sérialise que les
  embarquements entre eux ; rien n'empêche llama.cpp de prendre la même carte.
- L'agentique vers Gemini — la moitié manquante du régime `confort`.
