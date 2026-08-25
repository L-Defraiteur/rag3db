# 07 — Rapport de progression, 25 août 2026, minuit

Suite du [01](01-rapport-de-progression.md) (18h58). **Vingt et un commits**
entre 18h et minuit, tous sur `fts-lucivy-v3`, arbre propre au moment
d'écrire. Chaque chantier a son document dans ce dossier ; ceci est la vue
d'ensemble.

## 1. Ce qui est livré ce soir

| chantier | commits | doc | ce qui compte |
|---|---|---|---|
| **Recherche composable** | `3d4c51f83` | `../23-aout-2026-20h33/52` | signaux étiquetés, fusion N-aire, port `signals` en fan-in, `BM25SearchNode(fields=)`, **`RerankNode`** (remplace après la fusion, module en `boost`), vecteur via `SearchBackend`. Le « boost de titre » est une topologie. |
| **Gabarits typés** | `b8bf5f57f` | roadmap 06 | `$var` nu typé par inférence — `limit` et `gpu_batch_size` n'avaient jamais été respectés dans six gabarits sur sept. |
| **Environnement de test** | `7772db86b`, `d1b433c57` | 01 §7 | `in_memory()` réserve 1 TiB d'espace d'adressage (kuzu : 8) — 24 tests parallèles cassaient au hasard ; `run_e2e.sh` continue après un échec et dit « INCOMPLETE » ; `burn-llm`, `burn-ocr`, `code` dans le jeu de features. |
| **Vision fichiers / git** | `d1b433c57` | [02](02-fichiers-en-temps-reel-deux-modes-git-et-histoire.md) | `read`/`grep` lisent le réel et détectent la péremption ; deux modes = une `FileSource` ; résolution des relations contre la base ; `Commit`/`TOUCHES` avec horizon. |
| **`codeparsers` intégré** | `30c0fec67`, `19bea5934` | [03](03-codeparsers-integre-et-deux-bugs-de-fond.md) | crate réparé (offsets d'octets, hash, maps vides, UTF-8) ; `File`/`Scope`/`Library` avec `hashsafe` ; `ParseCodeNode → CodeIngestNode` ; FTS en upsert (la ré-ingestion doublait les documents). |
| **HNSW UPDATE segfault** | `aa86cc737` | `docs/25-aout-2026-20h30/01` (racine) | deux défauts dans l'extension (état de suppression partagé, vecteur en attente relu dans la table), un hors-bornes dans le cœur. UPDATE à 4 096, double ré-ingestion. **Passe complète 30 suites, 249 tests.** |
| **Attribution des références** | `cea9f5d3f` | [04](04-attribution-des-references-le-graphe-divise-par-sept.md) | 66 771 → 9 645 relations, mêmes 416 cibles ; références au scope le plus interne, fermetures fondues. |
| **`read` / `grep`** | `2e12851ef` | [05](05-read-et-grep-sur-une-source-de-fichiers.md) | sur une `FileSource` (`WorkingTree`, `Snapshot` — chemins virtuels), annotés par le scope le plus étroit, péremption par hash. Leçons de ragforge appliquées. |
| **`edit` / `list`** | `f0e72b6b3` | 05 §5 | remplacement unique, préfixes de `read` tolérés, ré-ingestion du fichier (disparus supprimés). |
| **L'agent sur notre code** | `53d9282aa`, `390ef76c5`, `0b62ee9b3` | [06](06-lacher-lagent-sur-notre-code.md) | 0,5 B : invente ; Gemini : juste en 8 s, **deux missions d'édition réussies**. Trouvé en chemin : fragmentation Vertex défectueuse (retours à la ligne bruts, `499` hors `data:`), arguments assainis sur le fil, « dernier pas » de la boucle d'agent. |

## 2. Mesuré

- Unitaires : **806** (toutes features), 720 (défaut).
- E2E : **30 suites, 249 tests, 0 échec** après le correctif HNSW (18h58 : 234). Depuis : `e2e_code` 5, `e2e_hnsw_scale` 13 (+4 gated), `e2e_burn_code_agent` 1, `e2e_cloud_code_agent` 2, `e2e_cloud_schema_probe` 1 — les trois derniers sautent sans identifiants Vertex.
- Notre `src/dataflow` : 25 fichiers, **1 229 scopes, 10 251 relations**, 8 par scope, 1,2 s d'analyse, 15 s d'ingestion.
- Gemini sur ce graphe : Q1 juste en 8,6 s / 4 appels ; M1 (ajouter une méthode) 5 appels ; M2 (renommer + appels) 11 appels, zéro reste. 14 000 à 40 000 jetons par question.
- Vertex : quelques dizaines de centimes sur la soirée.

## 3. Décidé ce soir (Lucie)

- **Les pondérations sont des topologies**, pas des réglages ; la KB garde l'index et perd la recette.
- `read`/`grep` **lisent le réel** ; la KB dit quand elle est périmée.
- **Deux modes, une `FileSource`** ; le mode recherche peut être « basé sur un git ».
- **L'histoire git avec horizon** (Commit/TOUCHES), à embarquer avec TTL.
- Relire ragforge avant d'écrire `read`/`grep` (fait — cinq leçons, trois pièges).
- Les chemins d'un dépôt distant sont **virtuels partout**.

## 4. Ce qui a été trouvé et qu'on n'a pas caché

- **HNSW UPDATE** : notre propre bug de février, révélé par le premier E2E à plus de 512 lignes vectorisées. La trace Release pointait trois appels trop bas ; le build Debug a été décisif.
- **`LocalRelTable::delete_`** (cœur) : référence hors bornes avant le test — UB en Release.
- **Ré-ingérer doublait les documents FTS** (`add_document` n'est pas un merge).
- **Vertex `stream_function_call_arguments`** : fragments non échappés + `499` hors `data:`. Désactivé par défaut.
- **`gcp_auth` avalait le corps du 400** — même piège que `openai_llm` le matin ; la clé de `secrets/` était l'ancienne, révoquée (celle du matin est dans `.vault/vertex-sa.json`).
- **Un test comptait 28 nœuds en dur** sous `openai-llm`, feature jamais retestée depuis `RerankNode` → `BUILTIN_NODE_COUNT` partout.
- **Le titre d'une entité simple n'est pas dans l'index BM25** (`bm25_fields = content_fields()`).
- **`.name()` résolu vers une méthode `name`** sans type de receveur — ambiguïté bornée, fausse une fois sur N.
- **Le modèle n'a pas pris `search_expand`** pour une question « qui utilise quoi » que le graphe résolvait en un appel.

## 5. En attente d'une décision humaine (inchangé)

`cargo publish` de lucivy 3.0.0 ; rapport de bugs à `tracel-ai` (16 entrées) ;
licence du lexique de prononciation ; et désormais : **renvoyer à Google** le
défaut de `stream_function_call_arguments` (dump SSE reproductible).
