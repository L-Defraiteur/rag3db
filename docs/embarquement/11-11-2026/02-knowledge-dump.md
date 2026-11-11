# L'embarquement de rag3weaver — ce qu'il faut savoir pour reprendre (11 novembre 2026)

*Pour un agent sans mémoire des échanges. L'état du jour (services, worktrees, tickets) est dans
`01-rapport-de-session.md`. Chemins de code relatifs à `extension/rag3weaver/` sauf mention ;
numéros de ligne relevés sur master le 11 novembre (ils bougent : chercher le nom).*

## 1. Les modèles (« embedders »)

Un **embedder** transforme un texte en vecteur. Tous tournent sur **burn** (bibliothèque Rust de
calcul), sur la carte par **Vulkan** (pilote RADV), en précision **Flex32** par défaut (stockage
f32, produits matriciels en f16, accumulation en f32 ; `RAG3WEAVER_BURN_FLOAT` pour changer).

| nom | dimension | fichier | lot conseillé (séquences × jetons) | rôle |
|---|---|---|---|---|
| `granite-278m` | 768 | `src/burn_granite_embedder.rs` | 128 × 512 | **modèle par défaut** du dense |
| `granite-107m` | 384 | idem | 128 × 512 | rapide ; choisi au premier index si plus de 50 000 fichiers ou carte faible |
| `bge-m3` | 1024 + creux | `src/burn_bge_m3_embedder.rs` | 32 × 512 | dense **et** creux en une passe ; 2,2 Go de poids |

- Le **creux** de BGE-M3 : une tête `Linear(1024→1)` + ReLU donne un poids par position du texte ;
  le vecteur creux garde, par id de jeton, le poids maximal (fonction `fondre`). Depuis `59f02b636`,
  `embed_sparse_spans` rend aussi la plage d'octets et le poids de chaque position (pour surligner).
  **Une plage peut commencer par l'espace qui précède le mot** (« ␣chat » = octets 2..7 dans
  « Le chat dort. ») : un surligneur retire l'espace de tête et fond les positions d'un même mot.
- Le choix du modèle : `src/embedding_choice.rs` (défaut `granite-278m`, seuils 50 000 fichiers et
  4 Gio de VRAM) ; le manifeste d'une base l'emporte sur `RAG3WEAVER_EMBED_MODEL`, qui l'emporte sur
  l'heuristique. `src/model_source.rs` relie un nom à un modèle local, à un service, ou à une API
  compatible OpenAI ; **un service injoignable est une erreur** (pas de repli local sauf
  `fallback: local` écrit).
- Les poids : `~/.cache/rag3weaver/<modèle>/model.bpk` et `tokenizer.json`, ou les variables
  `RAG3WEAVER_GRANITE_278M_BPK`, `RAG3WEAVER_BGE_M3_BPK`, etc.
- **OCR** (lecture de texte dans une image) : `ppocrv6-tiny`, feature `burn-ocr`,
  `src/burn_ppocr.rs`, poids dans `~/.cache/rag3weaver/ppocrv6-tiny/` (`det.bpk`, `rec.bpk`, `dict.txt`).
- **Relecteurs** (« rerankers », notent la pertinence d'un passage pour une requête) :
  `msmarco-minilm`, `mmarco-mminilm`, `bge-reranker-v2-m3` (`connect_reranker`, `src/model_source.rs`).
- `src/bge_m3_embedder.rs` (candle) n'est qu'un oracle de parité, pas une dépendance du produit.

## 2. Le service d'embarquement `rag3weaver-embeddings`

Un binaire (`src/bin/rag3weaver-embeddings.rs`, features `daemon,burn-embedder`, `burn-ocr` en plus
pour l'OCR) qui charge **un** modèle sur la carte et le sert en HTTP. Plusieurs clients partagent
ainsi un seul chargement (le 29 août, sept chargements de BGE-M3 par passe coûtaient 2 047 s).

- Lancement : `rag3weaver-embeddings --adresse 127.0.0.1:<port>` ; le modèle par
  `RAG3WEAVER_EMBED_MODEL` (défaut granite-278m) ; `RAG3WEAVER_RERANK_MODEL` et
  `RAG3WEAVER_OCR_MODEL` ajoutent un relecteur et un OCR au même processus. Une adresse non locale
  exige `--exposer`. Avant d'annoncer son adresse, il **chauffe** (lots de 1, 8, 32 et du lot
  conseillé ; ~3,5 s pour granite-278m ; `RAG3WEAVER_DEMON_CHAUFFE=0` pour s'en passer).
- Routes (`src/daemon/embeddings.rs`) : `GET /sante` (l'identité : modèle, dimension, dual, sparse,
  factice, précision, lot conseillé, `executable` = chemin@mtime, relecteur, OCR) ; `POST /embed`
  `{"texts":[…]}` → `{"vectors"}` ; `POST /embed_dual` → `{"dense","sparse"}` ;
  `POST /embed_sparse` → `{"sparse"}`, et avec `"spans": true` en plus `{"spans"}` ;
  `POST /rerank` `{query, passages}` → `{scores}` ; `POST /ocr` ; `POST /quitter`.
- Une seule passe GPU à la fois (verrou `passe`) ; les textes sont triés par longueur et découpés en
  lots (`par_lots`) ; `fils` = 4 sert à ce que `/sante` réponde pendant un calcul.
- Client : `DaemonEmbedder`. `from_service(modèle)` lit `RAG3WEAVER_SERVICE_EMBED` puis l'alias
  `RAG3WEAVER_EMBED_SERVICE` (liste d'adresses séparées par des virgules, **adresses complètes** :
  `127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981`, jamais `7979,7980,7981`) et s'attache à celle qui
  sert le modèle ; il ne lance ni n'arrête jamais un service. `assurer` s'attache ou lance ; `joindre`
  s'attache seulement.
- **Délai client** : `RAG3WEAVER_EMBED_TIMEOUT_SECS`, défaut **60 s** (`default_embed_timeout`) ; un
  lot qui le dépasse est rejoué texte par texte, les textes muets sont écartés du run.

### Les unités de luciepc

Depuis le 25 octobre, les trois services sont des unités `systemd --user` :
`~/.config/systemd/user/rag3weaver-embeddings@.service` (une instance par modèle,
`EnvironmentFile=%h/.config/rag3weaver/embeddings/%i.env`,
`ExecStart=%h/.local/bin/rag3weaver-embeddings --adresse ${RAG3WEAVER_EMBEDDINGS_ADDR}`,
`Restart=always`, au plus 3 démarrages en 5 min) ; plus la surcharge de journal par unité du 8 nov.
Ports : 7878 granite-278m, 7879 bge-m3 (+ relecteur, OCR), 7880 granite-107m. Sur le portable, les
tunnels 7979, 7980, 7981 (unité `rag3weaver-tunnels.service`). Une passe jouée **sur** luciepc pose
`127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880` (les ports 797x n'existent pas là-bas : 41 faux
rouges le 19 octobre).

## 3. Le régime GPU : brider la carte

Le **régime** décide combien de la carte une passe prend (`src/regime.rs`, `src/embedder.rs`,
`src/burst.rs`, `src/burn_device.rs`). Précédence partout : le code, puis la variable, puis le
régime, puis le défaut.

| variable | défaut | effet |
|---|---|---|
| `RAG3WEAVER_REGIME` | `plein` | `plein` : 100 %, 8 192 caractères par appel. `confort` : la deuxième carte si elle existe, sinon carte partagée à 60 % et rafales de 2 048 caractères. **Il n'y a pas de valeur `doux`** ; une valeur inconnue est traitée comme `plein`. `run_e2e.sh` pose `confort` par défaut. |
| `RAG3WEAVER_GPU_DUTY` | 100 (60 en confort partagé) | rapport cyclique en % (5 à 100) : après chaque lot, une pause de `travail × (100−d)/d` (`souffler`). Un client dont l'embarqueur est distant ne souffle pas (c'est le service qui le fait). |
| `RAG3WEAVER_EMBED_CHAR_BUDGET` | 8 192 (2 048 en confort partagé) | caractères par appel GPU ; une valeur posée désactive les rafales |
| `RAG3WEAVER_BURST_MS` / `RAG3WEAVER_BURST_PAUSE_MS` | 50 / 150 ms | durée visée d'une rafale et trou après elle |
| `RAG3WEAVER_BURN_DEVICE_EMBEDDER` / `_RERANKER` / `_OCR` | selon le régime | `cpu`, `gpu:N`, `igpu:N`, `rocm:N`. **À luciepc, en Vulkan, `gpu:0` = card2** (les services), card0 serait `gpu:1`. `HIP_VISIBLE_DEVICES` n'agit pas sur Vulkan. |

**Le « régime doux » des passes** (nom d'usage, pas une valeur de variable) : toute passe qui embarque
pose `RAG3WEAVER_EMBED_CHAR_BUDGET=4096 RAG3WEAVER_GPU_DUTY=70` (scripts `mesure_e2e.sh`,
`verdict-e2e-code.sh`) pour laisser la carte respirer pendant que Lucie travaille.

## 4. La dette d'embarquement, les réclamations et le rattrapage

- La **dette** est dans la base : un morceau de texte est dû quand son marqueur d'embarquement
  (`_embed_hash…__<modèle>` pour le dense, `_sparse_hash` pour le creux) est vide.
- Une **réclamation** : la colonne `_embed_claim` (`horodatage|writer_id`) qu'un processus pose
  atomiquement sur les morceaux qu'il va embarquer (`Catalog::reclamer_le_retard`, `src/catalog.rs`) ;
  libre si vide, périmée (60 s) ou la sienne. Deux écrivains ne calculent donc pas le même morceau.
- Le **rattrapage** : `embarquer_le_retard` réclame au plus 512 morceaux par table, les embarque, et
  n'écrit le vecteur que si le texte n'a pas changé depuis. Au-delà de 1 500 (dim ≤ 384) ou 2 000
  morceaux dus, l'index vectoriel HNSW est retiré puis reconstruit.
- Quand : l'ouverture d'un catalogue note seulement qu'une dette existe ; une recherche ou une
  ingestion qui exige des vecteurs la paie ; `run_index` (l'outil `index`) boucle jusqu'à dette nulle
  après le plein texte (`src/dataflow/index_nodes.rs`).
- **`catch_up_on_open`** : champ de manifeste `embeddings.catch_up_on_open` (`src/backend.rs`),
  défaut `false`, **`true` dans le gabarit de code livré** (`templates/backends/code/backend.json`).
  À l'ouverture d'un backend par l'hôte, si un premier index avait été coupé, ou si le champ est vrai
  et qu'il manque des vecteurs, l'hôte relance l'outil `index` en fond ; cet index-là ne bloque pas la
  fermeture (la dette reste en base).
- **Morceaux sans texte** : une passe de `run_index` les redécoupe depuis leur entité avant les
  vecteurs (`RAG3WEAVER_REECRIRE_LES_VIDES=0` la suspend).
- Ticket lié (pas à cette session) : des morceaux refusés par le service (erreur 500) pendant un
  index ne sont jamais repris par le rattrapage.

## 5. Mesures et chiffres

- **Latence de `POST /embed`** (18 octobre, luciepc seul, 50 appels après 5 de chauffe ; « doux » =
  3 morceaux de ≤ 1 200 caractères, « plein » = le lot conseillé ; à 50 appels p99 = max) :

  | modèle | doux médiane / max | plein médiane / max | débit plein |
  |---|---|---|---|
  | granite-278m | 36,2 / 89,2 ms | 1 229,5 / 1 484,7 ms (128 morceaux) | 104 morceaux/s |
  | bge-m3 | 69,9 / 80,3 ms | 829,8 / 1 860,5 ms (32 morceaux) | 39 morceaux/s |
  | granite-107m | 22,4 / 133,0 ms | 541,0 / 583,9 ms (128 morceaux) | 237 morceaux/s |

  Script : `~/.cache/rag3weaver-build/embarquements/latence-embed.py <adresse>… [--n 50]` (portable).
- **Mémoire de carte tenue par service** (25 octobre, `drm-total-vram` de `/proc/<pid>/fdinfo`) :
  granite-278m 6,1 Gio, bge-m3 + relecteur + OCR 8,7 Gio, granite-107m 4,1 Gio. card2 a 31,9 Gio.
- **Démarrage** : granite-278m prêt en ~6 s (chargé 2,1 s + chauffe 3,5 s), granite-107m ~2 s,
  bge-m3 avec relecteur et OCR ~14 s.
- **Parité d'un redéploiement** : le vecteur d'un texte témoin doit être identique **au bit près**
  (écart absolu max = 0). Les redéploiements des 25 oct., 7 et 8 nov. ont tous donné 0.
- `usages` sur l'index entier, d'octobre : 352 ms → 7-10 ms par les trois pas de la jointure par clé
  du moteur ; le « saut en deux temps » qui contournait le défaut a été retiré (pages d'octobre).

## 6. Incidents et pièges appris

- **Jamais un deuxième modèle d'essai sur card2 à côté des services** : le 25 oct., un second bge-m3
  a fait sept réinitialisations de l'anneau graphique (« ring gfx timeout ») ; le 9 nov., un
  cinquième service granite a refait un reset. Lire la VRAM libre d'abord
  (`/sys/class/drm/cardN/device/mem_info_vram_used`) ; une bascule se fait **en place** (arrêt de
  l'ancien par son pid, unité sur le vrai port, parité sur un vecteur noté).
- **Une unité `Restart=always` porte toujours `StartLimitBurst`** : sinon un plantage se répète sans
  fin. Revers : à la limite, l'unité reste arrêtée (`start-limit-hit`) ; la relancer par
  `systemctl --user reset-failed <unité> && systemctl --user start <unité>`, après avoir lu son journal.
- **Les quatre services partagent card2 sans partage équitable** : un e2e qui embarque en creux en
  continu (`e2e_charge_ingestion`, `e2e_sparse_dump`) prend bge-m3 à 100 % et affame les granite
  (5 à 19 s par texte le 8 nov.). Un tel e2e est un travail lourd GPU, pas pendant une file de pushes.
  Diagnostic : `rocm-smi --showuse`, le moteur tenu par chaque pid (`drm-engine-*` de fdinfo, delta
  sur 5 s), les clients par `ss -tnp`.
- **Les journaux tournent** : le journal `--user` de luciepc ne garde que quelques heures ; d'où la
  surcharge qui écrit chaque service dans un fichier. `dmesg` y demande sudo (refusé).
- **Le binaire est commun aux trois unités** : le remplacer ne change que les unités redémarrées
  ensuite ; redémarrer chaque unité exprès, avec sa parité.
- **`systemctl stop` d'une unité `systemd-run` qui lance `poste lourd …` n'arrête pas la passe** :
  `poste` la met dans sa propre portée ; arrêter par pid le `timeout`, le script et le `cargo`.
- **`cp -a --reflink` d'une copie scellée garde la lecture seule** : `chmod -R u+w` sur la copie.
- **Une lib de mesure se copie hors de l'arbre qui l'a bâtie** (un rebâti l'écraserait) et se prouve :
  identifiant de bâti par `strings librag3db.so | grep rag3db-build-`, `ldd` du binaire de test.
- **Shells** : l'outil Bash du portable est zsh (une variable non citée ne se découpe pas ; `$c:e`
  est un modificateur) ; luciepc est en fish (`$?` y est refusé : passer par `bash -s`).
- `rm -rf $VAR/…` est refusé par un contrôle de sécurité : écrire les chemins en clair.
- **Dernier pas de push** : la règle du 7 nov. veut la passe et tout rejeu **hors** du verrou
  `poste push` ; sous le verrou, seulement fetch, rebase, push et trois contrôles.

## 7. Procédures (racine du dépôt, `docs/procedures/`)

- `push-d-un-lot.md` : la passe à luciepc sous `poste lourd`, la lib entière sans
  `RAG3WEAVER_EMBED_SERVICE` (sinon elle peut se figer), les e2e avec, les témoins navigateur,
  `check_features.sh`, puis le dernier pas sous `poste push`.
- `bascule-de-lib.md` : vérifier les tunnels 7979-7981 avant de réveiller un hôte (sans eux, un hôte
  pose 0 vecteur et se replie en silence).
- Annonces : tout redémarrage d'un service de luciepc s'annonce 5 min avant au banc, à lucivy, à
  hooks-essai (les hooks de Lucie embarquent par 7979), à G et à c7.

## 8. Tests qui couvrent l'embarquement

- Unitaires : `src/daemon/embeddings.rs` (routes, spans, découpe, délais),
  `src/burn_bge_m3_embedder.rs` (`fondre`).
- e2e (un seul binaire `e2e`, lancé par `./run_e2e.sh --test <suite>`) : `e2e_service_spans` (par
  le service bge-m3, ne se saute pas sans service), `e2e_demon_embeddings`, `e2e_burn_granite`,
  `e2e_burn_embedder`, `e2e_banc_bge_m3`, `e2e_charge_ingestion`, `e2e_sparse_dump`,
  `e2e_embarqueur_absent`, `e2e_lectures`.
- `tests/common/mod.rs` : `SUITES_LOCALES` (suites qui doivent charger le modèle sur la carte
  locale) et `par_le_service`. **Soupçon non vérifié** : la liste compare le nom de l'exécutable,
  qui s'appelle maintenant `e2e-<hash>` ; elle ne reconnaîtrait donc plus aucune suite locale.
  Côté `run_e2e.sh`, `SUITES_CARTE_LOCALE` écarte ces suites par défaut.

## 9. Scripts

| script | où | usage |
|---|---|---|
| `latence-embed.py` | portable, `~/.cache/rag3weaver-build/embarquements/` | latence et débit de `/embed` par service |
| `temoin-embed.py` | luciepc, `~/.cache/rag3weaver-unites-embarquement/` | `noter <adresse> <fichier>` puis `verifier <adresse> <fichier> <dim>` : parité au bit près |
| `bascule-embarquement.sh` | idem | la bascule du 25 oct. des processus manuels vers les unités |
| `redeployer.sh`, `granites.sh` | luciepc, `~/.cache/rag3weaver-build/spans/` | redéploiement d'un binaire avec parité et retour arrière |
| `passe.sh` | idem | modèle d'une passe de lot à luciepc |
| `pousser-spans.sh` | portable, `~/.cache/rag3weaver-build/spans/` | dernier pas de push sous `poste push` |

## 10. La piste multimodale

Idée du 11 octobre, pas un chantier : un modèle local qui met images, audio et texte dans le même
espace (EmbeddingGemma-2 ou équivalent). Il s'emboîterait comme un modèle de plus derrière
`RAG3WEAVER_EMBED_SERVICE` (une unité de plus, un nom de modèle de plus dans `model_source.rs`). À
mesurer d'abord, sur une carte qui a la place (pas card2 en l'état).

## 11. Tickets

À cette session : `docs/embarquement/tickets/2026-11-08-granite-278m-a-luciepc-s-est-arrete-huit-fois-cause-illisible.md`
(l'unité 7878 sortie huit fois le 8 nov. à 10 h 47, cause illisible ; attend la prochaine sortie et
son journal). Les tickets voisins portés par d'autres sessions sont listés dans le rapport.
