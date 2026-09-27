# Reprise sur la nouvelle machine, ingestion MTG, et le chemin de masse par lots

**27 septembre 2026, 03 h 57.** Session menée avec Claude Code, de la compilation
sur la nouvelle machine jusqu'à une première tentative d'accélération de
l'ingestion. Ce document fait le point, chiffres à l'appui, pour les sessions suivantes.

## 1. La machine

ROG Flow Z13 (GZ302EAC), **Ryzen AI MAX+ 395** (Strix Halo), iGPU **Radeon 8060S
(gfx1151)**, 128 Go de mémoire unifiée (112 Go accessibles au GPU via
`ttm.pages_limit=29360128`), CachyOS, noyau 7.2, BIOS 304. L'ancienne machine
avait deux Radeon R9700 (RDNA4, gfx1201).

## 2. Compilation (à refaire tel quel)

Prérequis ajoutés : `ninja`, `vulkan-headers` (le reste était déjà là).

```bash
cd ~/git_workspaces/rag3db
cmake -S . -B build/lecteurs-csv -G Ninja -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_SHELL=FALSE -DBUILD_TESTS=FALSE -DBUILD_EXTENSIONS="vector"
cmake --build build/lecteurs-csv --target rag3db_shared rag3db_vector_extension single_file_header -j32
export RAG3DB_SHARED=1 RAG3DB_LIBRARY_DIR=$PWD/build/lecteurs-csv/src RAG3DB_INCLUDE_DIR=$PWD/build/lecteurs-csv/src
cargo build --release --manifest-path extension/rag3weaver/Cargo.toml \
  --features daemon,rag3db-native,openai-llm,burn-embedder \
  --bin rag3weaver-backend --bin rag3weaver-chat --bin rag3weaver-embeddings
```

- **`single_file_header` manquait dans les docs** : sans `rag3db.hpp`, le pont
  C++ du crate `rag3db` (`tools/rust_api`) ne compile pas (`fatal error: rag3db.hpp`).
- Build C++ : 996 unités. Build Rust release : ~3 min sur 32 cœurs.
- Tests : `cargo test --release --features daemon,rag3db-native --lib` →
  **1 045 passés, 0 échec, 6 ignorés** (avec `LD_LIBRARY_PATH=build/lecteurs-csv/src`,
  `TMPDIR=/var/tmp`).

## 3. burn / cubecl / cubek sur gfx1151 : ça marche tel quel

Les forks (`github.com/L-Defraiteur/{burn,cubecl,cubek}`, branche
**`rag3weaver/pre.3`**, commits `630c546c` / `0565518f` / `e9821ceb`) n'ont
rien de spécifique à RDNA4. Le démon d'embeddings :

```bash
RAG3WEAVER_EMBED_MODEL=bge-m3 RAG3WEAVER_BURN_DEVICE_EMBEDDER=igpu:0 \
  extension/rag3weaver/target/release/rag3weaver-embeddings --adresse 127.0.0.1:7878
```

- `Device<Vulkan(IntegratedGpu(0))>`, Flex32, autotune et fusion actifs, chargé en **2 s**,
  chauffe de 16 classes de forme en 40 s.
- **~8 500 tokens/s** en lots de 64 textes de ~60 mots (contre ~15 000 mesurés sur les R9700).
- Sanité : même carte FR/EN 0,79 ; carte voisine 0,51 ; texte hors sujet 0,38.

## 4. Données MTG reconstituées

- `mtga-reader` (npm) sous Proton : collection lue, **5 412 impressions / 10 454 exemplaires**.
- Arena → *Detailed Logs* activé → `build.py` : **catalogue 27 071**, **185 decks**,
  434 mécaniques, aucun identifiant orphelin. Jokers : 72 C / 34 U / 5 R / 2 M.
- `fetch_source.py` (cards, decks, mechanics), puis `prepare_engine_collection.py`,
  puis `prepare_engine_catalog.py`, puis
  `engine_collection.py --ingest --ingest-relations --ingest-catalog --ingest-catalog-links`.
- Base `engine-search-lucivy43-recovered.rag3db` : 2,1 Go, ~15 min de travail
  réel (la nuit a compté 5 h de veille volontaire, sans effet sur les données).
- La requête d'exemple `backend/queries/catalog/cheap-graveyard-drain.json`
  répond, et les filtres sont respectés (non possédées, C/U, coût bas).

Changements dans `experiments/mtga/scripts/engine_collection.py` :
- `RAG3WEAVER_BACKEND_BIN` pour choisir le binaire (défaut inchangé : `target/debug`) ;
- une erreur du backend enregistre la réponse complète dans
  `data/engine-last-error.json` et n'affiche que le début et la fin. Avant,
  seule la fin était affichée, et c'était l'écho des 56 Mo de données.

## 5. Ce que l'ingestion a montré

**Profil observé avec l'ancien chemin** (MERGE ligne à ligne) :
- `rag3weaver-backend` saturé à ~90 % d'un cœur ;
- GPU en dents de scie (30 à 88 %) : il attend pendant que le backend écrit ;
- ~63 cartes/s sur `CatalogCard`, en ralentissant avec la taille de la table.

**Pourquoi le chemin de masse ne servait qu'une fois** :
- `Catalog::premiere_ingestion_possible` exige une **table vide** ;
- or les outils `ingest_*` et `link_*` sont plafonnés à **512 lignes par appel**
  (`backend_nodes.rs:243-259`, `:412-415`, `maxItems: 512` plus une double
  vérification). Ce plafond est voulu : ces outils sont exposés aux agents ;
- seul le **premier** lot de chaque entité prenait donc `COPY`, et les 52 suivants
  (sur 27 071 cartes) passaient par `MERGE` et `split_unchanged`.

## 6. Tentative : le « lot de naissances » (NON concluante, à trancher)

**Changement** (`src/catalog.rs`, non commité) :
- `split_unchanged` rend aussi `Option<usize>` : le nombre de lignes du lot déjà
  en base (`None` si la relecture a échoué ou si l'entité est inconnue) ;
- si ce nombre vaut `Some(0)`, on est face à un **lot de naissances** : aucune
  clé ne peut heurter. Avec `naissances_par_copy_possibles()` (mêmes
  interrupteurs que la première ingestion, `RAG3WEAVER_INGESTION_LIGNE_A_LIGNE`
  et `supports_copy_from`), le lot prend le **chemin de masse** : `COPY`,
  embarquement `Enrich` avant l'insertion, `_chunked_hash` dans la ligne, sans
  `MarquerDecoupeNode` ;
- l'avertissement « 0 relue » de `split_unchanged` ne sort plus que s'il manque
  des uuids : un lot de naissances est normal ;
- le filet de sécurité existant reste en place : un `COPY` refusé repasse par `MERGE`.

**Mesure** (catalogue dans une base neuve, `RAG3WEAVER_INGEST_PROFILE=1`) :
- le chemin est bien pris : « lot de naissances, 512 lignes par le chemin de masse » ;
- le backend tombe à ~43 % de CPU, le GPU monte à 94-97 % ;
- **mais le débit baisse avec la taille de la table** : 74, puis 69, 60, 51, 47,
  et 45 cartes/s. À 17 920 cartes : **356 s**, contre **286 s** pour l'ancien
  chemin (ce matin, même machine, rien d'autre en cours). Même en retirant un
  blocage ponctuel de 30 s, le nouveau chemin reste **~14 % plus lent**.

**Cause localisée par le profil** : `chunk_insert` (le `COPY` des chunks avec
leurs vecteurs) **croît linéairement avec la table** :

| lot | 1 | 6 | 11 | 16 | 21 | 26 | 31 | 36 | 41 |
|---|---|---|---|---|---|---|---|---|---|
| `chunk_insert` (ms) | 781 | 1 263 | 1 624 | 2 112 | 2 718 | 3 303 | 3 736 | 4 205 | 4 474 |

Par comparaison, `embed` reste stable (~6,4 s par lot de 512), `insert` des
parents est à ~0,3 s et `chunk_link` à ~0,6 s. Un `COPY` sur une table **non vide**
semble donc coûter **O(taille de la table)**, soit un coût quadratique sur
l'ingestion complète. Hypothèses à vérifier côté rag3db :
- la maintenance de l'index vectoriel à chaque `COPY` : reconstruction ou
  parcours global au lieu d'insertions incrémentales ;
- un parcours complet de la table par le `COPY` sur une table non vide (index
  de clé primaire, vérification des doublons) ;
- la relecture des identifiants (`select_node_ids`) après le `COPY`, si elle
  balaye la table entière.

**Décision prise** : le chemin reste dans le code mais est **désactivé par
défaut**. Il faut `RAG3WEAVER_COPY_NAISSANCES=1` pour l'essayer. Sans cette
variable, le comportement est celui d'avant, et les tests donnent toujours
1 045 passés et 6 ignorés. Pour aller plus loin :
- ne l'appliquer qu'aux **parents** et aux entités **sans vecteurs**, où le
  `COPY` ne touche pas l'index vectoriel ;
- ou corriger le coût du `COPY` sur une table non vide dans rag3db.

## 7. Pistes suivantes

1. **Profiler l'ancien chemin** avec `RAG3WEAVER_INGEST_PROFILE=1`, pour avoir sa
   décomposition par nœud et comparer `chunk_insert` en `COPY` à `embed` suivi de `SET`.
2. **Trouver le coût linéaire du `COPY` sur table non vide** dans rag3db
   (extension vector, index de clé, `select_node_ids`).
3. **Recouvrir embarquement et écriture** : encoder le lot N+1 pendant l'écriture
   du lot N. C'est compatible avec l'unique écrivain de rag3db (voir
   `docs/6-septembre-2026-13h08/01-relecture-du-mvcc-de-vela.md`), et c'est ce qui
   supprimerait les creux de GPU de l'ancien chemin.
4. **Lots d'embarquement plus gros** : le démon annonce `lot_conseille (32, 512)`,
   alors que `EmbedNode` part à 32.
5. Une **voie d'ingestion « de confiance »** pour les scripts, sans le plafond de
   512, par exemple une opération `ingest_bulk` du protocole du backend, non
   exposée aux agents.
6. Côté produit MTG : brancher le backend en **MCP sur `llama-server`**
   (`--mcp-servers-config`), rebrancher la **validation Rhai des decks**
   (`templates/backends/validated-result/examples/deck`), et ne pas exposer les
   outils `ingest_*` / `link_*` aux agents.

## 8. Environnement LLM local (pour mémoire)

`llm-serve` (`~/.local/bin`) : gpt-oss-120b MXFP4 (50,5 t/s en génération,
621 t/s en lecture de prompt) et Qwen3.5-122B-A10B UD-Q4_K_XL (21,3 / 320 t/s),
les deux en **Vulkan** (plus rapide que ROCm sur gfx1151). Qwen3.5 : 1 couche sur
4 en attention complète, soit ~24 Ko/token de cache KV ; 262k natif, 1M possible
avec YaRN ×4.

## 9. Deck builder MCP sur `llama-server` (fait)

**Refus du harnais = erreur MCP.** `scripts/serve_backend_mcp.py` renvoie maintenant
`isError: true` quand `validation.accepted` vaut `false` (étapes `before`/`after`) ou
quand `delivery.ok` vaut `false`. Le texte liste chaque diagnostic (code, chemin,
message) et demande de corriger puis de resoumettre. Avant, un refus arrivait comme
un succès, qu'un agent pouvait ignorer. Autres ajouts au pont :
- `--hide PREFIX`, répétable : l'outil n'est ni listé ni appelable ;
- `--fixed-format` : le client ne choisit plus `response_format`. gpt-oss
  demandait `json` à chaque recherche, soit ~9 000 tokens par étape ;
- les textes rendus par `on_accept` sont affichés tels quels, donc copiables ;
- les reçus vides (validation acceptée sans diagnostic, livraison sans résultat)
  ne sont plus ajoutés aux vues texte.

**Outil `submit_deck`** (`experiments/mtga/backend/harness/`), déclaré dans `backend.json` :
- entrée : `{name, plan, mainboard:[{arena_id, quantity}], sideboard?}` ;
- `before` (`validate.mmd`) : 9 règles Rhai indépendantes. Erreurs : taille ≥ 60,
  réserve ≤ 15, carte connue, 4 exemplaires par nom toutes impressions confondues
  (exceptions « any number » / « up to N »), carte fabricable, budget de jokers par
  rareté. Avertissements : nombre de terrains hors de 21 à 27, peu de sources d'une
  couleur. Les sources de couleur manquantes sont une erreur ;
- `on_accept` : `export.rhai` (texte d'import Arena, une impression possédée si
  possible) et `craft_plan.rhai` (jokers à dépenser, rien n'est fabriqué) ;
- faits figés : `scripts/prepare_deck_harness.py` écrit `cards.json` (25 501
  impressions, 2,3 Mo) et `wildcards.json`. **À relancer après chaque
  rafraîchissement** de la collection.

Limite Rhai rencontrée : la limite de 131 072 éléments s'applique à la **somme** des
structures imbriquées. Chaque impression est donc une chaîne à tabulations, décodée
par `card()` dans les scripts, uniquement pour les cartes du deck.

**Branchement** : `~/.config/llm-serve/mcp.json` (serveur `mtg`, avec
`--hide close_backend --fixed-format`), pris en compte par `llm-serve`.
`serve_engine_mcp.sh` masque `ingest_*` et `link_*`, utilise le binaire release et
transmet ses arguments. Il faut que le démon d'embeddings tourne sur 7878.

**Essai réel** (gpt-oss-120b, sur batterie) : 10 étapes, 12 min ; un refus
(`copy_limit`), une correction, puis un deck accepté avec son texte d'import. La qualité
du deck reste celle du modèle : il a mis des terrains bicolores dans un mono-rouge.

**Ensuite** :
- alléger le rendu texte des recherches (~17 000 caractères pour 10 cartes) ;
- avertir des terrains hors couleurs ;
- légalité par format (Standard, Historic) : absente du catalogue ;
- rappel mémoire obligatoire : impossible en MCP. Il faut un proxy OpenAI devant
  `llama-server` ou la boucle `rag3weaver-chat`.

## 10. Extraits : `contentKind` et lignes du fichier (fait)

**Constat** : le bloc `chunk` des résultats affichait `startChar` / `endChar` /
`startLine` / `endLine` / `index` / `uuid` / `score`. Ces positions comptent
**depuis 0 et depuis le début du champ découpé**, pas depuis le début du fichier.
De plus, `_start_char` est un décalage en **octets** (`record_nodes.rs`). Pour un
scope, le `content` est son « texte propre » (`own_texts`) : chaque enfant y est
**replié** en une ligne de signature suivie de ` …`. Après le premier enfant, la
ligne k du contenu n'est donc plus la ligne `start_line + k` du fichier. Un agent
qui ferait un `replace` avec ces numéros viserait la mauvaise ligne.

**Config d'entité** (`config.rs`) :
- `contentKind` : `document` (défaut) ou `record` (une fiche) ;
- `sourceLines: {field, startLine?, folds?}` : le champ dont les lignes sont celles
  d'un fichier. Déclaré pour `Scope` (`content`, `start_line`, `folds`).

**Ingestion** (`code.rs`) : `own_texts` rend aussi les replis. Le nouveau champ
`folds` du scope vaut `ligne:début-fin,…` (ligne du contenu, depuis 0 ; empan dans le
fichier, depuis 1). Il vaut `?` quand le texte propre n'a pas pu être calculé (repli
de l'analyseur, qui peut n'être que le corps). **Le code doit être ré-ingéré** pour
remplir `folds` ; la colonne s'ajoute seule (`ALTER TABLE` de `migrate_entity`).

**Rendu** (`render_nodes.rs`, vue seulement ; le JSON des programmes ne change pas) :
le `chunk` de chaque résultat devient `excerpt`.
- **record** : pas d'extrait s'il répète un champ rendu, sinon le texte seul.
- **document avec lignes prouvées** : `sourceLines` déclaré, découpe `Lines`, et texte
  du chunk **identique** aux lignes annoncées du champ. On affiche alors les lignes du
  fichier, à partir de 1 : `… 125` en haut s'il y a du texte avant, `125` seul si
  l'extrait commence au début ; de même en bas. Une ligne repliée porte son empan
  `⟨142-168⟩`. Pas de colonnes.
- **sinon** : aucun numéro, le texte, et `…` là où il est coupé.
- l'arbre (`tree`) rend `excerpt` en bloc, au lieu de l'aplatir en `↵`.

Tests : 5 nouveaux, dont une propriété sur `own_texts` (chaque ligne non repliée
correspond à la ligne calculée du fichier). `--lib` avec `code` : 1 082 passés, 0 échec.

MTG : `contentKind: "record"` sur les 8 entités de `backend.json`. Recherche
« Lightning Bolt » : 17 097 → 14 095 caractères.

À faire plus tard : un **symbole déterministe lisible** (`src/catalog.rs#Catalog::ouvrir`),
utilisable à la place d'un uuid pour suivre les relations.

## 11. Ce que l'agent voit des outils : filtres, limites, schémas générés (fait)

Constaté dans l'interface de chat (`rag3weaver-chat` + `scripts/chat_app.py`, config
`experiments/mtga/chat/chat.json`) :
- l'agent a écrit `{"filter": {}}`, refusé par « expected map with a single key » ;
- il a mis « instant cost {1} » dans le texte de la requête ;
- avec `"filter"` au lieu de `"filter_condition"` dans les `options`, le filtre aurait
  été **ignoré sans erreur**.

**Moteur (générique)** :
- `FilterCondition` : `{}` = aucun filtre ; une forme invalide renvoie une erreur qui
  liste les clés attendues et donne un exemple (`filter.rs`).
- `SelectRecordsNode` : `limit` (0 = sans limite, négatif = non fourni) et
  `unfiltered_limit`, le plafond déclaré **par l'outil exposé** quand il n'y a ni
  filtre ni limite. Les graphes internes restent exhaustifs. Quand la limite coupe,
  un avertissement donne le total (« 20 lignes affichées sur 5412 ») sur le nouveau
  port `meta`.
- `json_schema.rs` garde le vocabulaire (`enum`, y compris celui des `items`) et le
  premier `examples` de chaque champ.
- Au chargement (`backend.rs`), le graphe de chaque outil dit quel paramètre alimente
  un `SelectRecordsNode.filter` ou un `SearchSourceNode.options`, et sur quelle
  entité (littérale ou liée). Le paramètre reçoit alors :
  - un **schéma** (forme d'un filtre à une clé ; pour `options`, seulement
    `filter_condition`, `limit`, `offset`… avec `additionalProperties: false`) ;
  - une **description générée** : champs groupés par type, avec opérateurs,
    vocabulaire et un exemple (les `examples` déclarés d'abord, sinon une heuristique
    qui évite `key`, `…_id`, `unknown`).

  Premier essai, avec schéma récursif et listes recopiées : +16 000 jetons par tour. Le
  compromis retenu (forme seulement, le moteur valide le reste) coûte ~6 000 jetons,
  mis en cache après le premier tour.

**MTG** :
- `vocabulary.py` déclare les couleurs (`White`…), les symboles de mana (`W`…), les
  types de carte et les exemples (`card_types ["Instant"]`, `printed_mana_value 2`).
  `subtypes` et `land_types` restent ouverts.
- `ability_facts.py` dérive du texte Oracle, pour chaque capacité : `kind`
  (keyword / activated / triggered / static), `costs` (tap, sacrifice…), `triggers`
  (cast, enters, dies…) et `effects` (untap, draw, token…). Il en fait l'union sur
  chaque carte : `ability_kinds`, `ability_costs`, `triggers`, `effects`. Couverture
  sur les 20 328 capacités du catalogue :
  - activées : 3 261 / 3 278 avec leur coût ;
  - déclenchées : 7 402 / 7 906 avec leur déclencheur ;
  - `{T}` de coût : 28 non reconnus sur ~1 700, les capacités accordées étant comptées.

  Paradox Engine → `triggers [cast]`, `effects [untap]` ; Llanowar Elves / Sol Ring →
  `ability_costs [tap]`, `effects [add_mana]`.
- `printed_mana_value` ajouté à la collection (`OwnedCard`).
- La chaîne de préparation **régénère `backend.json` et les schémas** : `contentKind`,
  le vocabulaire et `submit_deck` (`prepare_deck_harness.register`) y sont désormais
  écrits par les scripts, et la sortie `meta` des sélections est branchée par
  `prepare_engine_render.py`. Un essai à blanc confirme que rien ne s'est perdu.
- Prompt de `chat.json` :
  - collection très large ;
  - méthode de construction (carte moteur, synergies dans les deux sens, deck
    équilibré sauf demande) ;
  - critères structurés → filtre, jamais dans la requête ;
  - usage des champs de capacité pour les combos.

**Incident** : le WAL de la base MTG est devenu illisible
(`wal_record.cpp:79 KU_UNREACHABLE`) après qu'un second backend a ouvert la base tenue
par celui du chat. Récupération : copie reflink de la base et du WAL
(`data/backup-20260927-wal/`), WAL mis de côté (`….wal.illisible`), réouverture à
l'état du dernier checkpoint, sans perte puisque la session n'avait fait que lire.
**Correction du diagnostic** : à 14:15, l'ingestion a été tuée par la fin de la session
Claude Code, **sans aucun second processus**, et le WAL est à nouveau devenu illisible
(même assertion). Un arrêt brutal suffit donc, et l'ouverture concurrente de midi n'est
au mieux qu'un facteur. À instruire côté rag3db (rejeu du WAL après une coupure en pleine
écriture de lots avec vecteurs et lucivy). En attendant :
- arrêter les backends proprement (EOF ou Ctrl-C) ;
- prendre une copie reflink avant une longue écriture ;
- lancer les ingestions détachées (`setsid nohup`) ;
- un seul processus par base.

Fichiers mis de côté dans `data/wal-illisible-20260927-1415/` ; base restaurée depuis
`data/backup-20260927-avant-faits/`, puis ré-ingestion.

**Encore à faire** :
- afficher la réflexion (`reasoning`) dans la page de chat : aujourd'hui l'agent
  paraît figé ;
- `llama-server -np 2` si deux interfaces servent le même modèle ;
- symboles déterministes lisibles à la place des uuid ;
- réduire encore la taille du rendu des résultats (tableaux vides, `text_fr`, booléens
  par défaut…) : liste en attente de décision.
