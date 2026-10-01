# Recherche — rapport de session (nuit du 1er au 2 octobre 2026)

## Ce qui a été fait

1. **La proposition des pondérations dans les graphes** (pas C), par lecture
   seule : `docs/2-octobre-2026-00h43/01-ponderations-dans-les-graphes.md`
   (`5d31e4844`, sur master). Le nœud `FieldWeightNode`, l'échelle de
   préséance, la déclaration d'entité, les gabarits de dérivées, le sort du
   sparse, les preuves sans banc.
2. **Lucie a retenu la préséance C** (« le plus générique et personnalisable
   sans être chiant ») : distinguer le **choix** (`weights`, prime sur
   l'entité) du **défaut** (`default_weights`, ne vaut que si personne ne
   déclare). L'échelle, du plus fort au plus faible :
   `appelant > weights du graphe > entité > default_weights du gabarit > moteur`.
   C'est un choix de Lucie : il ne se retouche pas en passant — si le codage
   révèle un cas où « choix » et « défaut » ne suffisent pas, on s'arrête et
   on le dit.
3. **Le code est commencé** — étape 1a commitée sur `pas-c-ponderations`
   (`130984f61`, poussée) : les **quatre tests de l'échelle** sur
   `FuseResultsNode` (le défaut pèse sans déclaration ; l'entité bat le
   défaut ; le choix bat l'entité ; l'appelant bat tout) et la **structure**
   (champ `default_weights`, `with_default_weight`, `node_config`, parsing
   de fabrique aux deux formes). **La logique de préséance n'est pas
   écrite** : tests d'abord — ils doivent rendre leur rouge avant.

## Où exactement je me suis arrêtée

- **Branche** : `pas-c-ponderations` (`130984f61`), sur `origin`, basée sur
  `747b3ab70`.
- **Worktree** : `/home/lucied/git_workspaces/rag3db-pas-c`, submodule
  `codeparsers` initialisé, `user.email` vérifiée (la personnelle).
- **Target à part** : `/home/lucied/git_workspaces/rag3db-pas-c/target`
  (le partagé est à la session de l'arbre principal). `-j6`, jamais plus.
- **Deux compilations laissées finir en arrière-plan à l'arrêt** (détachées,
  elles remplissent ce target pour la reprise) : la précompilation
  `cargo test --no-run` du jeu complet, puis le run ciblé ci-dessous.
- **La commande à relancer en premier** (elle doit rendre les quatre
  nouveaux tests **rouges**, puisque la logique manque) :

```sh
cd /home/lucied/git_workspaces/rag3db-pas-c/extension/rag3weaver
CARGO_TARGET_DIR=/home/lucied/git_workspaces/rag3db-pas-c/target \
CARGO_BUILD_JOBS=6 RAG3DB_ROOT=/home/lucied/git_workspaces/rag3db \
cargo test --lib --features rag3db-native,burn-embedder,burn-ocr,code,daemon -j6 fuse_
```

## Le premier geste de la reprise : la logique de l'étape 1

Une fois le rouge lu, la logique tient en quatre retouches de
`generic_search_nodes.rs` plus une ligne de gabarit :

1. Un étage nommé à la place du booléen :

```rust
/// L'étage qui a fourni le bloc de fusion (voir `base_de_fusion`).
#[derive(Clone, Copy, PartialEq)]
enum SourceDesPoids { Appelant, Entite, Moteur }
```

2. `base_de_fusion` rend `(FusionConfig, SourceDesPoids)` — mêmes trois bras
   qu'aujourd'hui, l'étage au lieu du booléen `gabarit_decide`.
3. `signal_config(label, base, source)` :
   - `Appelant` → rien ne retouche le bloc ;
   - `Entite` → seul `self.weights` (le **choix**) retouche le poids ;
   - `Moteur` → `self.weights`, sinon `self.default_weights` ;
   - le rôle `Boost` s'applique **quel que soit l'étage** : c'est le câblage
     du graphe (un reranker branché en `signals` module au lieu de
     fusionner) — l'éteindre quand une entité déclarait cassait le graphe,
     pas les poids (latent, personne ne combinait les deux).
4. Dans `execute` : `strategy`/`rrf_k` viennent du bloc si
   `source != Moteur`, du nœud sinon (inchangé en substance).
5. `templates/tools/search_base.mmd` :
   `FuseResultsNode(weights='bm25:0.6,vector:0.4')` →
   `FuseResultsNode(default_weights='bm25:0.6,vector:0.4')` — ce que ces
   poids ont toujours été en intention.

Preuve de l'étape 1 : les quatre tests verts, le test du 18 septembre sur la
fusion aplatie toujours vert, et la non-régression des suites (le défaut du
gabarit s'applique exactement là où ses `weights` s'appliquaient).

## Ce qui attend ensuite

- **Étape 2** : `FieldWeightNode` dans un **module neuf** (corpus vivant :
  ni `port.rs` ni un gros fichier du banc), `EntityConfig.field_weights`
  (`Vec<FieldWeight>`, transporté par `SearchTarget` sans aplatir),
  `options.field_weights`, la même échelle, les trois conduites d'absence
  avec avertissements, posé d'office dans `search_base` entre `fuse` et
  `rerank` — attention : les tests qui comptent les nœuds internes du
  composite (`e2e_agent_loop`, 9 nœuds nommés) passeront à 10.
- **Arrêt convenu après les étapes 1 et 2** : branche poussée (pas de
  fusion), tableau des suites jouées une à une — lib, search,
  generic_search, result_mode, search_queue, code, graph_tool, agent_loop,
  simple_entity, entites_derivees — et `e2e_banc_etage` doit **compiler et
  tourner** (il n'est dans aucune liste de livraison et a déjà cassé en
  silence le 18 septembre).
- **Étape 3 ensuite** (après relecture de l'orchestrateur) : les gabarits
  de dérivées au catalogue.
- **Pas encore** : aucune valeur dans `Scope` — elles se mesurent au banc
  contre la référence du 2 octobre
  (`docs/2-octobre-2026-01h01/01-reference-du-banc-de-l-etage.md`,
  granite-278m, 43 questions : tel quel MRR 0,333 / R@1 9 / R@5 24 ;
  **G 0,412 / 13 / 27** — la cible que la pondération doit approcher sans
  filtrer). Ne jamais comparer aux chiffres de septembre : corpus, poids et
  poste ont changé. Le banc ne sort pas le rang par question ; si la
  pondération en a besoin, l'ajouter **dans le banc**, pas dans `src/`.
