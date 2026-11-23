# Un service sans écran, pilotable depuis Python ou TypeScript

Conception courte, avant tout code, à soumettre à Lucie (23 novembre 2026, « packager pour mieux régner »). Le but :
lancer rag3weaver comme un service, depuis un programme Python ou TypeScript, pour démarrer, lister, arrêter des
sessions d'agents, échanger leur flux, et déclarer les points d'accès des modèles (vLLM d'abord, chat et embeddings).

## Les exigences du premier terrain

1. **Un service à part, purement conteneurisable**, installé comme un outil et jamais mêlé aux dépôts qu'il sert : il
   reçoit un répertoire de travail et une branche, et ses propres données vivent dans son volume.
2. **Un plafond de sessions simultanées**, déclaré dans sa configuration (`max_active`). Une session inactive
   s'arrête et **reprend son contexte** quand on la rappelle : l'API a `start`, `stop` et `resume`, et l'état d'une
   session (sa conversation, ses plans, sa boîte) est durable hors du processus. C'est notre reprise au chargement,
   rendue explicite.
3. **Les garde-fous sont des permissions, pas des consignes.** Le service n'a besoin d'aucun secret au-delà du
   jeton des points d'accès des modèles. L'accès git se limite à pousser une branche et à ouvrir une demande de
   fusion, jamais à fusionner. En mode service, l'atterrissage pousse la branche de la session (`policy.landing:
   push_branch`) et ne commite jamais sur la branche principale.
4. **Le service dit où tourne chaque session** : `sessions.list()` rend, par session, l'hôte, le conteneur et le
   répertoire.

## Ce qui existe déjà

- **L'hôte** : `rag3weaver-backend daemon` (aujourd'hui sous systemd) et `rag3weaver-backend serve` ; un jeton local
  dans `daemon.token` ; les dossiers par `RAG3WEAVER_DATA_DIR` et `RAG3WEAVER_CONFIG_DIR`.
- **Les sessions d'agents**, sous des noms d'IDE : `POST /ide/teammate` (une demande, une instruction, un arrêt),
  `/ide/teammate/list`, `/ide/teammate/fork`, `/ide/agents`, `/ide/taches` (fusionner, rejeter), `/ide/runs/arreter`,
  `/ide/ouvrir` (un dossier).
- **Le flux** : `/ide/flux`, les sujets `agent`, `tokens` et la boîte, en événements (`RunStarted`, `LlmCall`, `Token`,
  `ended {stopped, by, landed}`, `turn_changes`…).
- **Les journaux** : le journal de chaque run, et la trace brute de chaque appel (`raw-call-trace`,
  `rag3weaver-backend trace <conversation>`).
- **Un premier mode sans écran** : `rag3weaver-tty --plain --request`.

Ce qui manque : un contrat stable et versionné (les routes `/ide/*` suivent l'IDE et changent avec lui), un flux par
session aux événements typés et à code de fin, un registre de modèles lu d'un fichier, un service lancé sans
systemd, et les clients.

## 1. Le contrat : `/v1`

Une façade mince et versionnée sur l'existant, décrite par un schéma (OpenAPI, dans le dépôt), dont les clients sont
tirés. Authentification : `Authorization: Bearer <jeton>`, le jeton lu d'un fichier.

| SDK | route | |
|---|---|---|
| `sessions.start({dir \| repo, branch?, task, model?, policy?})` | `POST /v1/sessions` | rend `{id}` ; `policy` : `workspace` (`in_place` / `copy`), la porte, les bornes |
| `sessions.list()` | `GET /v1/sessions` | id, état (`active`, `idle`, `ended`), modèle, début, fin, et **où** : hôte, conteneur, répertoire, branche |
| `sessions.resume(id)` | `POST /v1/sessions/{id}/resume` | reprend une session arrêtée avec son contexte, sous le plafond `max_active` |
| `sessions.send(id, message)` | `POST /v1/sessions/{id}/messages` | une instruction de suite |
| `sessions.stop(id)` | `POST /v1/sessions/{id}/stop` | l'arrêt d'aujourd'hui (coupe l'appel en vol, fait atterrir l'écrit) |
| `sessions.stream(id)` | `GET /v1/sessions/{id}/events` | SSE, reprise par `Last-Event-ID` |
| `endpoints.set(registre)` / `endpoints.list()` | `PUT` / `GET /v1/endpoints` | le registre (§ 5) |

**Le transport : HTTP et SSE**, pas de WebSocket : un flux d'un seul sens suffit (les commandes passent par les
routes), la reprise est native (`Last-Event-ID`), et un SSE traverse les mandataires sans réglage.

**Les événements du flux** (la forme proposée par tty, tirée de ce que fait le terminal aujourd'hui). **Un flux par
conversation, filtré et ordonné par l'hôte**, chaque événement portant `{seq, at_ms, conversation, turn, type, …}`,
`seq` étant le numéro d'émission commun posé par le bus (le ticket du même jour) :

- `turn.started {request_id, text, by}` · `turn.ended {code: done | failed | blocked | stopped | empty, by, landed, text}`
- `model.text {delta}` · `model.thinking {iteration, delta, summary}` · `model.thinking.end {iteration, chars}`
- `model.call {iteration, model, finish, prompt_tokens, cached_tokens, completion_tokens, context_window, since_reduction, empty}`
- `tool.started {call_id, tool, args, rank}` · `tool.finished {call_id, ok, ms, bytes, excerpt, error}`
- `card.opened {card_id, tool, class, command, scope}` · `card.closed {card_id, decision, by}`
- `facts.landed {files, into, commits, git}` · `facts.held {why, command}` · `facts.changed {files}` · `facts.uncommitted {paths}`
- `context.reduced {window, window_tokens, before, after, reduction, text}`
- `background.ended {handle, code, duration_ms, tail}` · `message {from, to, text}`
- `stream.gap {missed}`, quand le tampon de reprise ne couvre plus le curseur.

**Ce que l'hôte fait pour le client** (que le terminal fait seul aujourd'hui) : il filtre par conversation dès son
premier run, il fond le flux et la boîte en une seule source ordonnée, il n'envoie jamais les copies de fin de pas
(`origin: host_copy`), il donne le rang de chaque outil, et il sert les tours d'une conversation rouverte
(`GET /v1/sessions/{id}/turns`, texte, appels et `at_ms` de chaque tour). S'y ajoute la réponse à une carte :
`POST /v1/sessions/{id}/cards/{card_id}` (`card.answer`).

## 2. Le service

**Le nom** : la commande livrée s'appelle `rag3weaver`, et `rag3weaver serve` est un alias de `daemon`.

`rag3weaver serve --config <registre> --data <dossier> [--address 127.0.0.1:7970] [--token-file <f>]` : sans
systemd ni écran. `--data` pose les dossiers (`RAG3WEAVER_DATA_DIR`, `_CONFIG_DIR`, `_STATE_DIR`, `_CACHE_DIR`). Le
service n'a pas d'arrêt par inactivité, puisque personne ne le relancerait, et **il ferme proprement ses bases sur
SIGTERM et SIGINT** : un arrêt brutal casse le WAL, et un témoin le prouve. Les journaux vont en JSON par ligne sur la
sortie d'erreur, à côté du journal des runs. Sans humain, une carte d'accord n'attend pas : la politique déclarée
tranche (le juge, ou le refus), sinon la session finit en `blocked`.

**L'authentification** : `Authorization: Bearer <jeton>`, le jeton lu d'un fichier, sur toutes les routes de `/v1`.
Le jeton d'utilisatrice marque une décision humaine (une carte). **Une adresse autre que la boucle locale est
refusée sans `--token-file`** ; le chiffrement est l'affaire d'un mandataire devant le service (documenté), pas du
service.

## 3. Les SDK

Deux clients minces, **Python** (pip) et **TypeScript** (npm), tirés du même schéma, sans aucune FFI vers la lib :
ils parlent HTTP au service. `rag3weaver.serve()` lance le binaire livré en sous-processus, ou le client se branche sur
un service déjà lancé.

## 4. Le paquet (la part de G)

- **Le cœur livré : `bin/` + `lib/`**, le binaire lié avec `-Wl,-rpath,$ORIGIN/../lib`, et la lib rag3db épinglée
  copiée à côté. C'est la seule lib à nous ; lucivy et codeparsers sont liés dedans. Il n'y a qu'une chose à bâtir,
  pour toutes les formes.
- **L'image Docker d'abord** :
  - Debian slim, git, ca-certificates et tini ;
  - deux volumes : `/data` (local, pas de NFS) et `/config` ;
  - `ENTRYPOINT tini -- rag3weaver serve …`, avec un délai d'arrêt large ;
  - bâtie sans `burn-embedder` ni `burn-ocr`, inutiles devant un vLLM.
- **pip et npm par plateforme, sur le modèle de ruff et uv**, et non de lucivy, puisque nous livrons un serveur et
  non un addon :
  - une roue par plateforme qui porte `bin/` + `lib/` et le client Python pur ;
  - un paquet npm `rag3weaver` (client et lanceur) avec des `optionalDependencies` par plateforme (`os`, `cpu`, `libc`).
- **Linux d'abord** : macOS demande un bâti de la lib à part.
- **Le premier pas, qui décide de tout le reste** (mesuré par G, `objdump -T`) : l'hôte d'aujourd'hui demande
  `GLIBC_2.44`, et la lib `GLIBC_2.43` et `GLIBCXX_3.4.32` (libstdc++ de GCC 13), bâtis sur CachyOS. Ils ne
  tournent ni dans une image Debian (12 : glibc 2.36, 13 : 2.41), ni dans une roue manylinux. La lib C++ et l'hôte
  se bâtissent donc **dans l'image manylinux_2_28** (glibc 2.28), avec `-static-libstdc++`, ou avec libstdc++ copiée
  dans `lib/` et trouvée par `$ORIGIN`. La lib rag3db n'a jamais été bâtie dans une telle image : c'est l'étape la plus
  incertaine, estimée à une demi-journée à une journée d'agent, à prouver par un premier bâti.
- **La chaîne de publication, reprise de lucivy** (`.github/workflows/{release,build,ci}.yml` de son dépôt) :
  - un tag `v*` déclenche la release : la CI, les artefacts, la page de release, puis chaque registre dans son job ;
  - les publications passent derrière un environnement `release` et un interrupteur ;
  - chaque publication saute ce qui existe déjà, pour qu'un rejeu soit sans danger après une publication partielle ;
  - la publication de confiance (OIDC) pour PyPI et npm, sauf la toute première, qui demande un jeton ;
  - les runners se nomment `macos-latest`, jamais par un numéro que GitHub retire ;
  - chaque artefact natif est essayé avant d'être publié ;
  - pour un service, un simple binaire par plateforme sur la page de release vaut déjà paquet.

## 5. Le registre

Un fichier (TOML), une entrée par point d'accès compatible OpenAI ; ce qu'il y a derrière est l'affaire du client.

```toml
[[chat]]
name = "local"            # le nom par lequel une session le choisit
url = "http://vllm:8000/v1"
model = "qwen-coder"      # tel que le point d'accès le donne
key_file = "/run/secrets/vllm"
context = 131072
max_output = 16384
temperature = 0.666       # absent : rien n'est envoyé ; ramené dans la plage
temperature_range = [0, 2]
tools = true
thinking = false

[[embedding]]
name = "local"
url = "http://vllm:8000/v1"
model = "bge-m3"
key_file = "/run/secrets/vllm"
dimensions = 1024
max_input = 8192
batch = 64
```

Le registre remplace la température en dur (`model-temperature`) et la règle provisoire de Gemini 3 ; Vertex et
Anthropic y restent des entrées de leur protocole.

## L'ordre des lots, et ce qui part en parallèle

1. **archi** : le schéma `/v1` (OpenAPI) et le registre (le format ci-dessus, lu au démarrage et par
   `endpoints.set`) ; puis la façade `/v1/sessions` sur l'existant, avec un témoin par route et le flux typé.
2. **tty**, en parallèle : les événements typés et leur ordre (le relais, le numéro d'émission), le code de fin.
3. **G**, en parallèle, **en commençant par le bâti dans manylinux_2_28** (il décide de tout le reste) : `bin/` +
   `lib/` à `$ORIGIN`, `serve` sans systemd avec l'arrêt propre et son témoin,
   l'image Docker, les journaux sur la sortie d'erreur.
4. **lucivy**, dès le schéma posé : les deux SDK tirés du schéma, et les paquets pip et npm par plateforme.
5. **tty**, dès le schéma posé : le témoin de conformité, un faux hôte qui émet chaque type et un client minimal qui
   les lit tous.

## Les questions pour Lucie

- La propriété et la licence du paquet distribué.
- Sans humain, une décision attendue : refuser et finir en `blocked`, ou laisser le juge trancher ?
- Le démarrage à froid d'un pod : refaire l'index du dépôt à chaque session, ou le servir et le mettre en cache ?
