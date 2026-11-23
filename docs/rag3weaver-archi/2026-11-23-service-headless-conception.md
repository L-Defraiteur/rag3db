# Un service sans écran, pilotable depuis Python ou TypeScript

Conception courte, avant tout code, à soumettre à Lucie (23 novembre 2026, « packager pour mieux régner »). Le but :
lancer rag3weaver comme un service, depuis un programme Python ou TypeScript, pour démarrer, lister, arrêter des
sessions d'agents, échanger leur flux, et déclarer les points d'accès des modèles (vLLM d'abord, chat et embeddings).

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
| `sessions.list()` | `GET /v1/sessions` | id, dossier, état, modèle, début, fin |
| `sessions.send(id, message)` | `POST /v1/sessions/{id}/messages` | une instruction de suite |
| `sessions.stop(id)` | `POST /v1/sessions/{id}/stop` | l'arrêt d'aujourd'hui (coupe l'appel en vol, fait atterrir l'écrit) |
| `sessions.stream(id)` | `GET /v1/sessions/{id}/events` | SSE, reprise par `Last-Event-ID` |
| `endpoints.set(registre)` / `endpoints.list()` | `PUT` / `GET /v1/endpoints` | le registre (§ 5) |

**Les événements du flux**, typés, numérotés (le numéro d'émission commun du ticket du même jour) : `text` (morceaux
de la réponse), `thinking`, `tool_call`, `tool_result`, `facts` (fichiers changés, commits), `needs` (une décision
humaine attendue), `ended {code, landed}` avec **un code par issue** : `done`, `failed`, `blocked`, `stopped`.

## 2. Le service

`rag3weaver serve --config registre.toml --data <dir> --listen 127.0.0.1:7970 --token-file <f>` : sans systemd ni
écran, tout par chemins. Les journaux en JSON par ligne sur la sortie d'erreur, plus le journal des runs. Sans
humain, une carte d'accord n'attend pas : la politique déclarée tranche (le juge, ou le refus), sinon la session
finit en `blocked`.

## 3. Les SDK

Deux clients minces, **Python** (pip) et **TypeScript** (npm), tirés du même schéma. Ils lancent le binaire
(`Service.start()`) ou se branchent sur un service déjà lancé ; le binaire est embarqué par plateforme ou téléchargé
au premier usage, sur le modèle de lucivy.

## 4. Le paquet

Une image Docker d'abord, puis des roues pip et des paquets npm par plateforme ; la lib rag3db embarquée et trouvée
par `$ORIGIN` ; aucun modèle d'embarquement embarqué : l'embarquement passe par un point d'accès déclaré.

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
3. **G**, en parallèle : `serve` sans systemd, l'image Docker, les journaux sur la sortie d'erreur.
4. **lucivy**, dès le schéma posé : les deux SDK tirés du schéma, et la façon d'embarquer ou de télécharger le
   binaire.

## Les questions pour Lucie

- La propriété et la licence du paquet distribué.
- Sans humain, une décision attendue : refuser et finir en `blocked`, ou laisser le juge trancher ?
- Le démarrage à froid d'un pod : refaire l'index du dépôt à chaque session, ou le servir et le mettre en cache ?
