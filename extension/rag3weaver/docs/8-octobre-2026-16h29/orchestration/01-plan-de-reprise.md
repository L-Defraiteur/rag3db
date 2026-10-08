# Plan de reprise — six chantiers indépendants

8 octobre 2026, orchestration. Pour Lucie et pour les sessions qui reprennent.
Chaque chantier a sa session, ses fichiers, son premier geste et ce qu'il rend ;
les chantiers ne se touchent pas (la carte des fichiers au §3). L'état d'avant
est dans `../../3-octobre-2026-23h31/orchestration/01-rapport-de-session.md`.

Règles communes, inchangées : tout lourd sous `~/.cache/rag3weaver-build/poste`
(un rebâti exclusif à la fois) ; commit par chemins ; master en avance rapide,
jamais de force ; tests du changement pendant le travail, la batterie complète
une fois à la fusion ; rapport de session dans le dossier du jour.

## 1. Ce qui attend Lucie avant que certains chantiers partent

| Décision | Chantier bloqué |
|---|---|
| la branche `defauts-bascules` (`7b82c761f`) : oui / non ; coexistence des bases en blobs ; les 8-10 s avant la première chose cherchable | A |
| le modèle pour l'essai Claude en agent (`claude-opus-5` recommandé d'abord, `claude-fable-5-1` ensuite) et la clé d'API dans `.vault` | E |
| « envoie » pour les envois A, C, D, E vers tracel-ai (B est partie : tracel-ai/cubek#776) | aucun — se fait par l'orchestration |
| `cargo login` du compte personnel, pour réserver le nom `rag3weaver` sur crates.io | aucun |

## 2. Les six chantiers

### A — Arbre principal : les défauts basculés, puis la commande en fond

- **Premier geste** : rebaser `defauts-bascules` sur master, jouer la batterie
  complète une fois, fusionner en avance rapide. Si Lucie dit non à l'une des
  trois questions, ajuster la branche d'abord.
- **Ensuite** : la commande en fond (vision du produit code, §0) — un
  paramètre `background` sur `run` qui rend la poignée et le journal tout de
  suite ; un verbe « la fin du journal » (N dernières lignes, ou depuis mon
  dernier regard) à côté de `wait` ; la fin livrée comme message avec le code
  de sortie ; une mort propre du processus à la fin du run. Page courte,
  témoin, puis code.
- **Rend** : master fusionné ; les nouveaux défauts dits dans le README ; la
  commande en fond avec ses témoins.
- **Fichiers** : `catalog*.rs`, `code_sync.rs`, `fts_*.rs`,
  `dataflow/run_nodes.rs`, `commande.rs`, `tools.rs` (schéma de `run`).

### B — Cœur C++ : la fuite de pages, puis le basculement du COPY journalisé

- **Premier geste** : les deux mesures manquantes (COPY forcé tué avant son
  point de reprise ; COPY replié tué) — rouges d'abord ; puis le correctif de
  la page 06 (`../../3-octobre-2026-23h31/coeur-cpp/06-les-pages-sans-proprietaire.md`,
  acceptée : étendue du fichier dans l'en-tête, rendre sans tronquer, numéro
  de version de stockage), relecture du banc.
- **Ensuite** : `RelCopyBMExceptionRecoverySameConnection` (pourquoi le repli
  ne s'est pas déclenché sous un tampon minuscule) ; les huit tests qui
  supposaient le point de reprise d'un COPY ; le reste du banc sous le défaut
  basculé (la passe à blanc s'est arrêtée à 51 cas) ; puis le basculement,
  derrière la série tenue de F.
- **Rend** : ticket de la fuite fermé avec témoin ; `force_checkpoint_on_copy`
  à `false` par défaut, liste complète verte.
- **Fichiers** : `src/storage/`, `src/transaction/`, `src/main/`, `test/`.

### C — Session recherche : l'exécution asynchrone des graphes

- **Page** : `02-l-execution-asynchrone-des-graphes.md` (ce dossier).
- **Premier geste** : le trait (`execute` inchangé + `execute_async` avec son
  pont `block_in_place`), le runtime en `async` avec `execute_blocking` pour
  les appelants synchrones, le test « un seul des deux par nœud », un nœud
  témoin asynchrone. Puis la boucle d'agent et le réacteur en tâches, les six
  `block_on`.
- **Rend** : batterie du crate verte ; mesure avant/après (banc de recherche,
  une passe d'agent) ; aucun des 78 nœuds modifié.
- **Fichiers** : `dataflow/node.rs`, `dataflow/runtime.rs`,
  `dataflow/reactor.rs`, `agent.rs`, `postgres_connection.rs`, tests.
- **Ne touche pas** : `run_nodes.rs`, `tools.rs` (chantier A), `llm.rs`,
  `openai_llm.rs` (chantier E).

### D — Banc : l'élagage de l'index vectoriel, puis la mise à jour massive

- **Premier geste** : finir la colonne « master » des mesures, classer les deux
  colonnes (vecteurs distincts, quasi-doublons, exacts, ordinaires, trois
  passes fond et trois masse), rendre les chiffres. Si la règle classique +
  la borne ne passe pas le critère (aucune classe ne recule, rappel@10 ne
  baisse pas, bâti ≤ +20 %), essayer `keepPrunedConnections` (algorithme 4).
  Rien ne part sans ce critère.
- **Ensuite** : la mise à jour massive de vecteurs
  (`../../3-octobre-2026-23h31/banc-de-concurrence/04-la-mise-a-jour-de-vecteurs.md`),
  référence de coût prise sur le nouvel élagage ; plafond ×2 sur
  `TenThousandRowsInBatchesOf512`.
- **Rend** : les deux tickets de ligne injoignable fermés ou leur chiffre ; la
  mise à jour massive verte.
- **Fichiers** : `extension/vector/`, `test/` (banc). Relecture du cœur C++
  sur le rejeu et le chemin disque.

### E — Session nouvelle : Claude en agent, et la comparaison

- **Premier geste** : une implémentation du trait `Llm` pour l'API Messages
  d'Anthropic, en HTTP brut avec `ureq` comme le client OpenAI (pas de couche
  de compatibilité) : blocs `tool_use` / `tool_result`, flux SSE poussé dans
  le `TokenSink`, réflexion adaptative par défaut, gestion du `stop_reason`
  (`refusal` compris). Feature `anthropic-llm`. Test avec une transcription
  enregistrée, puis une passe réelle sur une tâche de ce dépôt.
- **Ensuite** : le même jeu de tâches que pour Gemini, joué avec Claude ; ce
  que chaque modèle a lu, édité, cassé, lu dans les rapports d'exécution ;
  l'avis du modèle sur nos outils gardé comme artefact.
- **Rend** : le client avec ses tests ; un tableau Gemini / Claude sur le
  même jeu ; l'artefact.
- **Fichiers** : nouveau `anthropic_llm.rs`, `Cargo.toml` (feature), un
  exemple ; lit `llm.rs` sans le modifier.
- **Attend** : le modèle et la clé (§1).

### F — Embarquements : la série d'avant basculement, puis le contrat du dialecte

- **Premier geste**, au signal du cœur C++ (lib ≥ `fb98852e1`, puis avec la
  fuite de pages corrigée) : la série tenue d'avant basculement — fichiers
  2 048 × 1 et blobs 2 048 × 1, défaut d'aujourd'hui et COPY journalisé, trois
  passes alternées ; fichiers à ±3 %, blobs à leur temps d'avant ; et ce que
  le saut du journal des transactions forcées a gagné sur le défaut
  d'aujourd'hui.
- **En attendant le poste** : le contrat du dialecte — ce qu'un
  `SchemaDialect` doit fournir, ce que chaque dialecte déclare savoir faire
  (transactions, chargement en masse, index de mots et de vecteurs), et la
  batterie qu'une implémentation nouvelle fait passer ; PostgreSQL comme
  première épreuve. Page d'abord.
- **Rend** : le tableau de la série ; la page du contrat et sa batterie.
- **Fichiers** : `tests/e2e_mesure_*.rs`, `dialect.rs`,
  `postgres_search_backend.rs`, `postgres_blob_store.rs`.

## 3. La carte des fichiers (qui ne doit pas se croiser)

| Fichier ou dossier | Chantier |
|---|---|
| `catalog*.rs`, `code_sync.rs`, `fts_*.rs`, `run_nodes.rs`, `commande.rs`, `tools.rs` | A |
| `src/storage/`, `src/transaction/`, `src/main/`, `test/` (moteur) | B |
| `dataflow/node.rs`, `dataflow/runtime.rs`, `dataflow/reactor.rs`, `agent.rs` | C |
| `extension/vector/`, `test/` (banc) | D |
| `anthropic_llm.rs` (nouveau), `Cargo.toml` features | E |
| `tests/e2e_mesure_*.rs`, `dialect.rs`, `postgres_*.rs` | F |

Deux croisements à savoir : C et F touchent tous deux `postgres_connection.rs`
(C pour ses cinq `block_on`, F pour le contrat) — C passe en premier, F lit
après ; B et D partagent `test/` du moteur et le rebâti exclusif — un seul à
la fois sous le verrou, comme avant la pause.

## 4. Ce que l'orchestration fait pendant ce temps

- Les envois tracel-ai A, C, D, E au mot de Lucie ; la réservation du nom sur
  crates.io quand le jeton est posé.
- La relecture par git de chaque chantier à son « prêt », la carte des
  fichiers tenue, le rapport d'orchestration à jour.
- La suite de C (ports-flux, puits d'affichage, run en fond) écrite en page
  quand C a fusionné, pas avant.
