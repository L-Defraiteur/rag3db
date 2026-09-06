# Issue 04 — rapport de la session « moteur burn » pour la session principale

**6 septembre 2026, soir.** Écrit par la session qui a passé la journée sur le
chemin burn/Vulkan (issues [02](02-le-chemin-burn-wgpu-a-optimiser.md) et
[03](03-le-chemin-vulkan-ce-qu-il-valait-et-les-lignes-qui-manquaient.md)),
à l'intention de la session principale, qui tient le chemin d'indexation. Ce
document dit ce qui est acquis côté moteur, ce qui est mesuré, et ce qui
revient à l'indexation pour que ça serve.

## 1. Le moteur, tel qu'il est ce soir

**Par défaut** : burn 0.22.0-pre.3 (par trois forks d'une ligne chacun, voir
`docs/forks-burn-cubecl-cubek.md`), Vulkan, **Flex32** (stockage f32, matmul
f16 sur les matrices coopératives, accumulation f32 ; `RAG3WEAVER_BURN_FLOAT=f32`
pour l'ancien comportement), autotune et fusion (features `burn-autotune`,
`burn-fusion`, dans `burn-embedder`), dépendances en -O3, graphe BGE-M3 avec
attention fusionnée sur 23 couches, `cubecl.toml`. Les six suites de modèles
passent. **Rien n'est commité** : l'arbre porte ces changements (16 fichiers
modifiés, 6 nouveaux). À commiter avant qu'une autre session touche à l'arbre.

BGE-M3 sur la carte TV (gfx1201), jetons ≈ mots :

| lot | ce matin | **ce soir** | llama.cpp (même modèle, même carte) |
|---|---|---|---|
| 8 × 100 mots | 5 351 j/s | **24 212** | 22 889 |
| 32 × 100 | 5 755 | **21 225** | 23 404 |
| 64 × 100 | 3 803 | **21 882** | 22 044 |
| 32 × 300 | 3 327 | 9 891 | 20 423 (une seule séquence) |
| 8 × 900 | 2 009 | 4 294 | 13 183 (une seule séquence) |

**On est à parité avec llama.cpp** sur BGE-M3 : le moteur n'est plus le
problème. Ingestion de `src/dataflow` (30 fichiers, 1,15 Mo) : **75 s** contre
149 s à la fin de l'issue 01 et 862 s au réveil.

Ce que ça veut dire pour un gros corpus : à 65 s/Mo, un noyau Linux (1,3 Go)
c'est 24 h sur une carte. Le reste du gain n'est pas dans le moteur.

## 2. Ce que Vertex et ragforge disent

**Vertex `text-embedding-005`** : un appel à la fois plafonne à 8 000 jetons/s
(latence 1,4 à 2,2 s) ; à huit appels en vol, 429 dès la deuxième seconde, quota
par minute. Vertex compte 1,5× plus de jetons que nous sur les mêmes textes.
Avec le quota tel qu'il est, **le local va trois à quatre fois plus vite**.

**Ragforge** (`/home/lucied/LR_CodeRag/community-docs/packages/ragforge-core`)
s'indexait en moins d'une heure parce qu'il embarquait *moins*, pas parce que
son moteur allait plus vite — à travail égal, notre chemin GPU est plus rapide
(0,33 s/fichier ramené à son modèle, contre 0,6 chez lui) :

| | ragforge | rag3weaver aujourd'hui |
|---|---|---|
| unité | scope AST (fonction, méthode) embarqué entier | scopes emboîtés de ~100 mots (~130 jetons) |
| taille | 30 lignes / 1 500 caractères (≈ 375 jetons), fenêtre glissante avec 5 lignes de recouvrement seulement si le scope dépasse | ~130 jetons |
| emboîtement | **aucun** : une classe = sa ligne de déclaration, la hiérarchie en relations | le parent contient le texte des enfants |
| volume embarqué | ≈ 1× la source | 1,5× la source |
| textes par scope | jusqu'à 3, séparés : signature, source brute, docstring | — |
| modèle | bge-base-en-v1.5, f16, TEI, 32 textes/appel, 5 appels en vol | BGE-M3 (5× le calcul), 8 192 caractères par appel |
| incrémental | hash SHA-256 à trois niveaux (fichier, nœud, embedding + modèle) | — |
| goulot documenté | Neo4j, pas TEI | — |

Sources : `src/runtime/embedding/text-chunker.ts:37-40`,
`codeparsers/src/scope-extraction/BaseScopeExtractionParser.ts:448-450`,
`src/runtime/embedding/tei-embedding-provider.ts:46-47`,
`src/brain/embedding-service.ts:1005`, `src/brain/change-detector.ts`.

## 3. Les modèles candidats (llama-bench, Vulkan, f16, une séquence)

| modèle | corps | dim | fenêtre | pp512 (j/s) |
|---|---|---|---|---|
| BGE-M3 (référence) | 24 × 1024 | 1024 | 8192 | 26 977 |
| multilingual-e5-base | 12 × 768 | 768 | 512 | 38 000–60 300 |
| **granite-embedding-278m-multilingual** | 12 × 768 | 768 | 512 | 38 200–59 200 |
| bge-base-en-v1.5 (ragforge, anglais) | 12 × 768 | 768 | 512 | 38 100–58 000 |
| EmbeddingGemma-300M | Gemma | 768 | 2048 | 46 000 |
| **granite-embedding-107m-multilingual** | 12 × 384 | 384 | 512 | 102 000–147 000 |
| multilingual-e5-small | 12 × 384 | 384 | 512 | 61 500 |
| all-MiniLM-L6-v2 (anglais) | 6 × 384 | 384 | 512 | 116 000 |

Le multilingue ne coûte rien : e5-base et granite-278m ont le corps de
bge-base-en, seul le vocabulaire change, et un vocabulaire ne se calcule pas.
**Pas besoin d'un modèle anglais pour le code et d'un autre pour le reste.**

Recommandation : **granite-107m et granite-278m** (Apache-2.0, entraînés texte
et code, français inclus, même graphe XLM-R que BGE-M3 donc même pipeline
burn-onnx), et le choix entre les deux sur un banc de **qualité** sur nos
propres requêtes (questions en français sur le code de rag3db, BGE-M3 en
référence) — IBM publie quelques points d'écart en recherche de code, pas un
gouffre, et 384 dimensions divisent l'index par deux (un noyau : 3 Go au lieu
de 6). BGE-M3 reste pour la recherche fine ou ce qui est consulté.
EmbeddingGemma-300M (2 048 jetons, code) plus tard : son graphe n'est pas
BERT.

Réserve : notre MiniLM multilingue (même corps que granite-107m) n'a fait que
×1,2 sur BGE-M3 ce soir alors que llama.cpp lui donne ×4 — à cette taille,
notre chemin est borné par les lancements de noyaux et des lots trop petits,
pas par la carte. **Un petit modèle ne rend rien tant que les lots ne sont pas
faits pour lui.**

## 4. Cahier des charges pour la session principale (chemin d'indexation)

Par ordre de rendement. Chacun se mesure avec `e2e_mesure_ingestion_code`
(75 s aujourd'hui) et le banc `e2e_banc_bge_m3`.

1. **La découpe, façon ragforge.** Scopes **plats**, ~375 jetons (1 500
   caractères), fenêtre glissante avec recouvrement seulement quand un scope
   dépasse. Un scope non-feuille (classe, module, fichier) embarque **sa
   déclaration, pas le corps de ses enfants** ; la hiérarchie vit dans le
   graphe, et **le retrieval doit pouvoir relister les enfants d'un scope
   trouvé** (et remonter au parent) sans deuxième recherche vectorielle — une
   relation directe, ou un texte structurel court par non-feuille (signature +
   liste des enfants). Attendu : ÷3 sur le nombre de vecteurs, ÷1,5 sur le
   texte embarqué. C'est le plus gros levier.
2. **Les lots par modèle, en jetons.** Le budget de 8 192 caractères par appel
   est réglé pour la mémoire de BGE-M3 ; un modèle 12 × 384 veut 256 séquences
   et plus par appel. Budget en jetons, par modèle ; longueurs arrondies au
   multiple de 64 (`pad_to_multiple_of`) et tailles de lot fixes (8, 16, 32,
   64, 128, 256) après le tri par longueur — c'est la contrepartie de
   l'autotune (une classe de forme = un réglage, une fois par poste, cache
   dans `~/.cache/cubecl`). Voir issue 03 §3 et §10 A.
3. **Recoller le pipeline au banc.** L'ingestion tourne à 11 000 jetons/s là
   où le banc en fait 21 000 sur les mêmes lots : tokenisation, HTTP et JSON
   vers le démon, base entre deux appels, rien ne se recouvre. Trois étages
   (tokeniser, calculer, écrire) qui se chevauchent ; et le démon peut servir
   **les deux cartes** du poste.
4. **Incrémental par hash**, à trois niveaux comme ragforge (fichier, nœud,
   embedding + nom du modèle), pour qu'un noyau ne s'indexe entièrement
   qu'une fois.
5. **Embarquer moins** : déduplication des chunks identiques, exclusion des
   fichiers générés et vendorisés.
6. Optionnel : trois textes par scope (signature, source, doc) séparés comme
   ragforge, plutôt qu'un texte concaténé.

## 5. Ce que la session moteur garde

- Monter **granite-107m et 278m** sur burn (ONNX → burnpack, comme BGE-M3),
  avec leurs lots calibrés, et les mettre dans `e2e_banc_bge_m3` (un test par
  modèle, à lancer `--exact` un par un).
- Le **banc de qualité** : un jeu de requêtes en français sur le code de
  rag3db, BGE-M3 en référence, pour trancher 107m / 278m.
- Les formes stables côté moteur (issue 03 §10 A) avec le point 2 ci-dessus.
- Les trois PR amont (cubecl-wgpu, burn-cubecl, cubek-matmul) et ROCm comme
  option détectée (`/opt/rocm`, paquet `rocwmma`).

## 6. Où sont les choses

- Récit et chiffres : `docs/issues/6-septembre-2026/03-…md` (§8 bis pour les
  comparaisons).
- Forks : `docs/forks-burn-cubecl-cubek.md` (L-Defraiteur/cubecl, /burn,
  /cubek, branches `rag3weaver/pre.3`).
- Bancs : `tests/e2e_banc_bge_m3.rs` (BGE-M3, MiniLM, MiniLM multilingue),
  `tests/e2e_banc_vertex_embedding.rs` (feature `openai-llm`,
  `GOOGLE_APPLICATION_CREDENTIALS`, `GOOGLE_CLOUD_PROJECT`), GGUF des
  candidats dans `~/.cache/rag3weaver/gguf/`.
- Ce qui tourne à l'exécution se lit sur une ligne au chargement de chaque
  modèle : `[rag3weaver] burn : … · précision … · autotune … · fusion …`, et
  `RUST_LOG=cubecl_wgpu=debug,cubecl_runtime=info` dit le reste.
