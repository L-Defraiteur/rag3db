# Le service d'embarquement sur l'autre poste

3 octobre 2026 — session embarquements.

Lucie : « on pourrait, pour ne pas se bloquer de trop, utiliser mon GPU qui
sert pas à seat1 sur l'autre PC, faire servir un modèle d'embedding depuis
là-bas ». C'est fait : trois démons `rag3weaver-embeddings` tournent sur
`luciepc`, joignables d'ici par trois tunnels ssh, et une variable suffit pour
qu'une suite ou un backend s'en serve à la place de la carte de ce poste.

## 1. Ce qui tourne, et où

| | modèle | écoute sur luciepc | port local d'ici (tunnel) |
|---|---|---|---|
| service 1 | granite-278m (768 d) | `127.0.0.1:7878` | `127.0.0.1:7979` |
| service 2 | bge-m3 (1 024 d, dense + creux) | `127.0.0.1:7879` | `127.0.0.1:7980` |
| service 3 | granite-107m (384 d) | `127.0.0.1:7880` | `127.0.0.1:7981` |

Trois services parce qu'un démon sert un modèle : le banc et l'agent de code
demandent granite, les suites e2e ordinaires demandent bge-m3 dense et creux,
le banc de qualité compare aussi granite-107m.

Sur luciepc, rien n'écoute hors de la boucle locale. Le code vient d'un
worktree à part, `~/git_workspaces/rag3db-service`, détaché sur
`origin/master` (`6d6e5588d`) ; l'arbre `~/git_workspaces/rag3db` de là-bas
(branche `mtg-experiments`, fichiers modifiés) n'a reçu qu'un `git fetch`.
**On n'y commite rien et on n'y pousse rien** : l'identité git globale de ce
poste est l'adresse professionnelle. Les poids sont ceux de
`~/.cache/rag3weaver/` là-bas, que le démon trouve seul.

## 2. La carte

luciepc a deux Radeon AI PRO R9700 et un iGPU Intel, et deux sièges.

- `card0`, PCI `0000:07:00.0` : **la carte de `seat1`**, l'écran de jeu.
  udev la marque `ID_SEAT=seat1`, `loginctl seat-status seat1` la liste comme
  son seul périphérique DRM, son HDMI est actif. On n'y touche pas.
- `card2`, PCI `0000:04:00.0` : l'autre R9700, rattachée à `seat0`, aucun
  connecteur actif. **C'est elle que les démons prennent**, par
  `RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0` (wgpu énumère dans l'ordre PCI :
  bus 04 avant bus 07).

Vérifié à chaque lancement par la VRAM : au chargement de granite-278m,
`card2` est passée de 1 426 à 4 986 Mio et `card0` n'a pas bougé ; au
chargement de bge-m3, `card2` a pris 4 433 Mio et `card0` 17. Pour refaire la
vérification :

```bash
cat /sys/class/drm/card{0,2}/device/mem_info_vram_used
```

Le régime là-bas est `plein`, écrit : la carte n'affiche rien, il n'y a rien
à ménager.

## 3. Lancer, arrêter (sur luciepc)

Le shell de luciepc est fish : d'ici, passer les commandes par
`ssh lucied@luciepc bash -s < script`.

Bâtir (3 min 25 s à froid, `-j8` pour ménager le poste) :

```bash
cd ~/git_workspaces/rag3db-service/extension/rag3weaver
cargo build --release --bin rag3weaver-embeddings --features daemon,burn-embedder -j8
```

Lancer les trois, détachés :

```bash
BIN=~/git_workspaces/rag3db-service/extension/rag3weaver/target/release/rag3weaver-embeddings
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0 RAG3WEAVER_REGIME=plein RAG3WEAVER_EMBED_MODEL=granite-278m \
  setsid nohup $BIN --adresse 127.0.0.1:7878 > ~/.cache/rag3weaver/service-embeddings.log 2>&1 < /dev/null &
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0 RAG3WEAVER_REGIME=plein RAG3WEAVER_EMBED_MODEL=bge-m3 \
  setsid nohup $BIN --adresse 127.0.0.1:7879 > ~/.cache/rag3weaver/service-embeddings-bge-m3.log 2>&1 < /dev/null &
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0 RAG3WEAVER_REGIME=plein RAG3WEAVER_EMBED_MODEL=granite-107m \
  setsid nohup $BIN --adresse 127.0.0.1:7880 > ~/.cache/rag3weaver/service-embeddings-granite-107m.log 2>&1 < /dev/null &
```

Chacun annonce dans son journal la carte prise et « à l'écoute sur … » après
quelques secondes de chauffe.

**L'adresse d'écoute est l'argument `--adresse`**, pas une variable : le
binaire ne lit pas `RAG3WEAVER_EMBEDDINGS_ADDR` (le knowledge dump de
l'optimiseur dit le contraire, c'est une erreur ; cette variable n'est lue
que par `tests/common`).

Arrêter, proprement :

```bash
kill -TERM $(pidof rag3weaver-embeddings)      # les trois
ss -ltnp | grep -E ':78(78|79|80)'             # qui tient quel port, pour n'en arrêter qu'un
```

Pour mettre le service à jour : `git fetch origin` dans
`~/git_workspaces/rag3db`, `git checkout --detach origin/master` dans
`rag3db-service`, rebâtir, arrêter, relancer.

## 4. Les tunnels (sur ce poste)

```bash
ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -L 127.0.0.1:7979:127.0.0.1:7878 lucied@luciepc
ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -L 127.0.0.1:7980:127.0.0.1:7879 lucied@luciepc
ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -L 127.0.0.1:7981:127.0.0.1:7880 lucied@luciepc
curl -s http://127.0.0.1:7979/sante    # dit le modèle servi, la dimension, la précision
curl -s http://127.0.0.1:7980/sante
```

Les ports locaux ne sont pas 7878 exprès : c'est l'adresse du démon local que
les suites lancent et remplacent.

## 5. S'en servir d'ici

**Une variable : `RAG3WEAVER_EMBED_SERVICE`**, une adresse ou plusieurs
séparées par des virgules.

```bash
export RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981
```

Le client demande à chaque adresse son identité et prend celle qui sert le
modèle voulu. Si aucune ne le sert, c'est un refus qui dit ce que chacune
sert — pas des vecteurs d'un autre modèle dans un index, et pas un repli
silencieux sur la carte d'ici, que la variable existe pour épargner. Le
client **s'attache et rien d'autre** : il ne lance, n'arrête ni ne remplace
jamais un service désigné par cette variable.

> La variable vit dans le code depuis la branche `regime-carte-partagee` :
> une session bâtie d'avant sa fusion ne l'honore pas.

Qui l'honore :

- **Les suites e2e** (`tests/common`) : `BGE_M3`, `GRANITE_278M`,
  `GRANITE_107M` passent par le service quand la variable est posée. La
  batterie complète du 3 octobre : 374 tests verts, 0 rouge, en 20 minutes.
- **Le banc** : il prend ses modèles par ces mêmes statics, donc il suit sans
  rien de plus.
- **Les backends** (`rag3weaver-backend`) : dans `embeddings`, laisser
  `address` vide avec `provider: daemon` ; l'adresse vient alors de la
  variable, choisie par `model`. Une `address` écrite dans le manifeste
  garde le dernier mot. Les scripts Python (`test_backend_persistence`,
  `_snapshot`, `_lifecycle_batch`, `test_migration_v8`) vident l'adresse du
  gabarit quand la variable est posée.
- **Une ingestion par le code** : `DaemonEmbedder::from_service("granite-278m")`
  rend `None` si la variable n'est pas posée, sinon le client ou le refus.

Qui reste sur la carte d'ici, variable posée ou non, parce que c'est
l'embarqueur, le démon ou la vitesse de ce poste qu'ils éprouvent :
`e2e_burn_*`, `e2e_demon_embeddings`, `e2e_mesure_ingestion_code`,
`e2e_banc_bge_m3`. La liste et sa raison sont dans `tests/common`
(`SUITES_LOCALES`) et en tête de chaque suite. Les modèles que le démon ne
sert pas (MiniLM, les relecteurs, l'OCR) restent locaux aussi.

**À ne pas faire** : pointer `RAG3WEAVER_EMBEDDINGS_ADDR` sur un tunnel.
Cette variable-là désigne *notre* démon local ; `tests/common` y remplace un
démon d'une autre construction en lui envoyant `/quitter` — il arrêterait le
service distant.

**Deux arbres qui jouent des e2e en même temps** se disputent le démon
local de `127.0.0.1:7878` : chacun trouve celui de l'autre « périmé » et le
remplace (arrivé le 3 octobre, entre cette batterie et celle de l'arbre
principal). Les suites restées locales en lancent un ; donner à chaque arbre
son port : `RAG3WEAVER_EMBEDDINGS_ADDR=127.0.0.1:7890`.

Il existe une autre voie, sans rien de tout cela : un service d'embarquement
compatible OpenAI, par `HttpEmbedder` (`provider: compatible` dans le
manifeste), quand le poste ne suffit pas et qu'on préfère un service tiers.

## 6. La preuve

- **Parité** : 32 phrases (trois courtes, vingt-neuf morceaux de 900
  caractères de code) embarquées par granite-278m ici, sur l'iGPU du Z13, et
  là-bas, sur la R9700, toutes deux en Flex32. Écart absolu maximal
  **4,1e-5**, médian 3,5e-5, cosinus minimal 0,99999993. Deux cartes
  différentes en demi-précision : l'écart attendu, pas zéro.
- **Débit là-bas, vu d'ici par le tunnel** : 2 048 morceaux de 1 200
  caractères du code de la crate, par requêtes de 256, en 16,4 s — 125
  morceaux par seconde, **148 000 caractères par seconde**, sérialisation
  JSON et tunnel compris. Non mesuré dans les mêmes conditions sur l'iGPU
  d'ici : la mesure aurait figé l'écran une à deux minutes.

## 7. Après un redémarrage

**De luciepc** : les démons ne se relancent pas seuls. Refaire le § 3
(lancer), vérifier la carte (§ 2), puis d'ici vérifier `/sante` — les
tunnels, eux, sont tombés avec la machine : refaire le § 4.

**De ce poste** : les démons tournent toujours là-bas ; refaire seulement
les tunnels (§ 4) et reposer la variable (§ 5).

**Un tunnel tombé** se voit à `curl` qui refuse la connexion, ou à une suite
qui échoue sur « aucun service ne sert … ne répond pas » : relancer la ligne
`ssh -f -N …` correspondante.

## 7 bis. Le modèle de décision, par llama-server

Ajouté le 3 octobre au soir. La session optimiseur a mesuré les modèles de
décision (`docs/optimiseur/3-octobre-2026-20h55/02-…`) : JevK5-4B est le seul
utilisable, à deux secondes la décision sur processeur. Il est servi de
luciepc, sur la même carte libre, **par llama-server et non par notre
moteur**.

| | modèle | écoute sur luciepc | port local d'ici (tunnel) |
|---|---|---|---|
| service 4 | JevK5-4B v0.3, Q8_0 (4,5 Go) | `127.0.0.1:7881` | `127.0.0.1:7982` |

- **La mémoire tient** : avant lui la carte `0000:04:00.0` portait 13 507 Mio
  (les trois modèles d'embarquement) sur 32 624 ; après, 18 323. Rien à
  arrêter. La carte de `seat1` n'a pas bougé (6 749 Mio).
- **Le binaire** est celui qui était déjà sur le poste,
  `~/git_workspaces/llama.cpp/build/bin/llama-server` (commit `d2462f8`, juin
  2026, bâti avec Vulkan). On ne bâtit ni ne modifie rien dans ce dossier.
  Il voit trois périphériques ; `Vulkan0` est la carte libre (reconnue à sa
  mémoire disponible, puis vérifiée par la mémoire prise au chargement).
- **Les poids** : `~/.cache/rag3weaver/decision/` là-bas, téléchargés du dépôt
  public `alibiserikbay/JevK5-GGUF` — `jevk5-4b-v0.3-Q8_0.gguf` (sha256
  `aea433883bc7…979d4a30`) et `jevk5-4b-v0.3-Q4_K_M.gguf` (`94ca0d7745c4…8938c882`).

Lancer (sur luciepc) :

```bash
setsid nohup ~/git_workspaces/llama.cpp/build/bin/llama-server \
  -m ~/.cache/rag3weaver/decision/jevk5-4b-v0.3-Q8_0.gguf -c 8192 -ngl 99 --device Vulkan0 \
  --host 127.0.0.1 --port 7881 > ~/.cache/rag3weaver/service-decision.log 2>&1 < /dev/null &
curl -s http://127.0.0.1:7881/health     # {"status":"ok"} en trois secondes
```

Arrêter : `kill -TERM $(pidof llama-server)`. Changer de modèle : arrêter,
relancer avec l'autre fichier — un seul modèle à la fois sur ce port.

Le tunnel (sur ce poste) :

```bash
ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -L 127.0.0.1:7982:127.0.0.1:7881 lucied@luciepc
```

Ce que le client appelle : `GET /health`, `POST /tokenize`, `POST /completion`
(un jeton, quarante log-probabilités). Vérifié par le tunnel : la réponse
porte `completion_probabilities[0].top_logprobs`, et un `/completion` prend
109 ms sur un prompt de seize jetons — contre deux secondes sur processeur.

**Ce service n'est pas désigné par `RAG3WEAVER_EMBED_SERVICE`** : cette
variable ne connaît que l'embarquement. Son adresse se donne à la main à qui
s'en sert, en attendant l'abstraction « un modèle, en service ou en local »
(proposition à venir).

Après un redémarrage de luciepc : relancer la commande ci-dessus avec les
trois démons d'embarquement (§ 3) ; après un redémarrage de ce poste :
refaire le tunnel avec les trois autres (§ 4).

## 7 ter. Un petit modèle de langage, pour éprouver les protocoles

Ajouté le 3 octobre à 22 h 30. La passe « agent faible » du backend de code
passe par le chat et un point d'accès compatible OpenAI ; ce poste n'a que
des modèles de 60 Go, qu'on ne lance pas pendant que Lucie travaille. Un
petit modèle généraliste **qui sait appeler des outils** est donc servi de
luciepc, sur la même carte libre.

| | modèle | écoute sur luciepc | port local d'ici (tunnel) |
|---|---|---|---|
| service 5 | Qwen2.5-7B-Instruct, Q4_K_M (4,7 Go) | `127.0.0.1:7882` | `127.0.0.1:7983` |

- **Le choix** : poids ouverts, licence Apache-2.0 (lue sur la fiche du
  modèle et dans son fichier `LICENSE`), sept milliards de paramètres en
  quatre bits, appels d'outils gérés par le gabarit du modèle (`--jinja`).
  « Faible » est voulu : c'est un modèle pour éprouver les protocoles, pas
  pour coder.
- **La mémoire** : 18 331 Mio avant, 24 459 après, sur 32 624. La carte de
  `seat1` n'a pas bougé. Il reste environ 8 Gio.
- **Les poids** : `~/.cache/rag3weaver/llm/Qwen2.5-7B-Instruct-Q4_K_M.gguf`
  là-bas, du dépôt public `bartowski/Qwen2.5-7B-Instruct-GGUF` (sha256
  `65b8fcd92af6…ceaaa1423`).

Lancer (sur luciepc) :

```bash
setsid nohup ~/git_workspaces/llama.cpp/build/bin/llama-server \
  -m ~/.cache/rag3weaver/llm/Qwen2.5-7B-Instruct-Q4_K_M.gguf -c 32768 -ngl 99 --device Vulkan0 --jinja \
  --alias qwen2.5-7b-instruct --host 127.0.0.1 --port 7882 > ~/.cache/rag3weaver/service-llm.log 2>&1 < /dev/null &
```

Arrêter celui-ci sans toucher au modèle de décision : `ss -ltnp | grep 7882`
donne son pid, puis `kill -TERM <pid>` (`pidof llama-server` en rend deux).

Le tunnel (sur ce poste) :

```bash
ssh -f -N -o ExitOnForwardFailure=yes -o ServerAliveInterval=30 -L 127.0.0.1:7983:127.0.0.1:7882 lucied@luciepc
```

S'en servir : `base_url: http://127.0.0.1:7983/v1`, `model:
qwen2.5-7b-instruct`, sans clé, fenêtre de 32 768 jetons. Vérifié par le
tunnel sur `/v1/chat/completions` : un appel d'outil (`finish_reason:
tool_calls`) en 0,6 s, puis la réponse en texte une fois le résultat de
l'outil rendu.

**Pour le déclarer par `models.llm`** (lot 6 de la proposition, pas codé) :
la déclaration existe déjà — `{"provider": "compatible", "protocol":
"openai", "model": "qwen2.5-7b-instruct", "address":
"http://127.0.0.1:7983/v1"}`, ou l'adresse par `RAG3WEAVER_SERVICE_LLM`. Il
manque trois choses : un constructeur qui rende un `OpenAiLlm` depuis cette
déclaration (comme `connect_embedder`) ; la fenêtre de contexte, que la
déclaration commune ne porte pas et que le chat déclare aujourd'hui
(`context_tokens`) ; et que la section `llm` du chat devienne un alias de
`models.llm`, comme `embeddings` l'est devenu de `models.embed`.

## 7 quater. Le relecteur et l'OCR, par le démon bge-m3

Depuis le 3 octobre à 23 h 45, le démon du port 7879 porte aussi un relecteur
et un OCR : un seul démon à relancer. Il se lance comme au § 3, avec deux
variables de plus (et la carte des deux rôles) :

```bash
RAG3WEAVER_BURN_DEVICE_EMBEDDER=gpu:0 RAG3WEAVER_BURN_DEVICE_RERANKER=gpu:0 RAG3WEAVER_BURN_DEVICE_OCR=gpu:0 \
RAG3WEAVER_REGIME=plein RAG3WEAVER_EMBED_MODEL=bge-m3 \
RAG3WEAVER_RERANK_MODEL=bge-reranker-v2-m3 RAG3WEAVER_OCR_MODEL=ppocrv6-tiny \
  setsid nohup $BIN --adresse 127.0.0.1:7879 > ~/.cache/rag3weaver/service-embeddings-bge-m3.log 2>&1 < /dev/null &
```

Son identité (`curl -s http://127.0.0.1:7980/sante`) déclare `"reranker":
"bge-reranker-v2-m3"` et `"ocr": "ppocrv6-tiny"`. Un backend les demande par
`"models": {"rerank": {"provider": "service", "model": "bge-reranker-v2-m3"},
"ocr": {"provider": "service", "model": "ppocrv6-tiny"}}` ; sans adresse
écrite, le client les cherche parmi les adresses de l'embarquement. La carte
libre passe de 17,5 à 22,3 Gio. Le binaire doit être bâti avec la feature
`burn-ocr` en plus (`--features daemon,burn-embedder,burn-ocr`).

## 8. Ce qui reste ouvert

- Les démons ne sont pas des services systemd : un redémarrage de luciepc
  demande la relance à la main. À faire si l'usage dure.
- Cinq modèles chargés (trois d'embarquement, un de décision, un de
  langage) tiennent environ 24 Gio sur les 32 de la carte.
- Une enveloppe d'embarqueur doit relayer `distant()` : celle des tests ne
  le faisait pas, et le client cadençait ses rafales pour une carte qui
  n'était pas la sienne (`e2e_charge_ingestion` : 1 019 s, puis 123 s).
- Le service distant tourne le code de master, sans le régulateur de rafale
  de la branche : sans effet là-bas, la carte n'affiche rien.
