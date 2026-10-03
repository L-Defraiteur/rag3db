# Un modèle, en service ou en local — proposition

3 octobre 2026, 21 h 22 — session embarquements. Pour Lucie, portée par
l'orchestration.

Lucie : « trouver une abstraction pour que les modèles puissent à chaque fois
être déclarés en service ou en local, abstraire ce qu'on a fait pour les
services d'embedding, que ce soit simple de refaire la même chose avec l'OCR,
ou TTS/STT si on ajoute ensuite ».

## 1. Où on en est : cinq capacités, cinq façons

Inventaire fait dans le code de master, le 3 octobre au soir.

| Capacité | Trait | En local | En service à nous | En service tiers | Au manifeste du backend | Par l'environnement |
|---|---|---|---|---|---|---|
| Embarquement dense | `Embedder` | burn, en processus | démon (`DaemonEmbedder`) | `HttpEmbedder` (contrat OpenAI) | `embeddings` (`provider: daemon \| compatible`) | `RAG3WEAVER_EMBED_SERVICE` |
| Creux, dual | `SparseEmbedder`, `DualEmbedder` | burn | le même démon | — | **rien** | — |
| Relecteur | `Reranker` | burn | — | — | **rien** | — |
| OCR | `Ocr` | burn (PP-OCR) | — | — | **rien** | le dossier des poids |
| LLM | `Llm` | — | — | `OpenAiLlm` (compatible, Vertex) | **rien** (le chat a sa propre section) | `RAG3WEAVER_LLM`, `RAG3WEAVER_LOCAL_LLM` |
| Décision | **n'existe pas** | — | — | llama-server, par un client Python | — | — |

Trois faits que l'inventaire a fait sortir, et qui ne sont pas de la
tuyauterie :

- **Un backend qui déclare un signal creux n'a pas d'embarqueur creux.** Le
  backend compte le signal pour décider qu'il lui faut un service, puis ne
  branche jamais ni le creux ni le dual : seuls les tests le font.
- **Un backend n'a jamais de relecteur ni d'OCR.** Les nœuds existent
  (`RerankNode`, `OcrNode`), le backend ne leur donne aucun service.
- **Rien ne dit, de façon commune, si un modèle calcule ici ou ailleurs.**
  Chaque trait d'embarquement a son `distant()` ; le relecteur, l'OCR et le
  LLM n'en ont pas. C'est pourtant ce que demandent le régulateur de rafale,
  la classe de carte et l'estimation.

## 2. Ce qu'on propose

**Une déclaration, la même pour toute capacité** — ce qui change d'une
capacité à l'autre est ce qu'on envoie et ce qu'on reçoit, pas la façon de
dire où vit le modèle.

```json
"models": {
  "embed":  { "provider": "service", "model": "granite-278m", "dimensions": 768 },
  "sparse": { "provider": "service", "model": "bge-m3" },
  "rerank": { "provider": "local",   "model": "bge-reranker-v2-m3" },
  "ocr":    { "provider": "local",   "model": "ppocrv6-tiny" },
  "decide": { "provider": "compatible", "protocol": "llama_server", "model": "jevk5-4b",
              "address": "http://127.0.0.1:7982", "fallback": "refuse" }
}
```

| Champ | Ce qu'il dit |
|---|---|
| `provider` | `local` (notre moteur, dans ce processus), `service` (un démon à nous), `compatible` (un service tiers) |
| `model` | Le modèle attendu. Un service à nous dit ce qu'il sert : s'il sert autre chose, c'est un refus. |
| `address` | Une adresse ou plusieurs. Absente : elle vient de l'environnement. |
| `protocol` | Pour `compatible` seulement : `openai`, `llama_server`, `vertex`. |
| `api_key_env` | Le nom de la variable qui porte la clé, jamais la clé. |
| `fallback` | Ce qu'on fait si le service ne répond pas : `refuse` (défaut) ou `local`. |

**Par l'environnement, la même chose** : une variable par capacité,
`RAG3WEAVER_SERVICE_<CAPACITÉ>` (`RAG3WEAVER_SERVICE_EMBED`,
`RAG3WEAVER_SERVICE_DECIDE`…), une adresse ou plusieurs séparées par des
virgules. `RAG3WEAVER_EMBED_SERVICE` reste, comme alias de
`RAG3WEAVER_SERVICE_EMBED` : aucune passe en cours ne casse. L'ordre de
précédence est celui du reste de la crate : le code, puis le manifeste, puis
la variable, puis le défaut.

**Le repli se déclare, il ne s'improvise pas.** Le défaut est `refuse` : un
service injoignable donne une erreur qui dit ce que chaque adresse sert et
quoi poser. `local` ne s'obtient qu'en l'écrivant. Jamais de repli silencieux
sur la carte qui porte l'écran — c'est la règle déjà tenue par
`RAG3WEAVER_EMBED_SERVICE`, étendue à tout.

## 3. Ce que toute capacité reçoit sans rien écrire

Un seul module résout une déclaration en un client, et rend avec lui **d'où
vient le calcul**. Tout ce qui a été fait pour l'embarquement lit ce fait, et
vaut donc pour la capacité suivante :

- **plusieurs adresses**, la bonne choisie par le modèle servi ;
- **le refus qui dit quoi faire**, et le repli seulement s'il est déclaré ;
- **le régulateur de rafale** : il ne cadence que ce qui calcule sur la carte
  d'ici ;
- **la classe de carte** de l'heuristique : un service attaché fait taire
  les déclencheurs de la carte d'ici ;
- **la sonde de débit et l'estimation** : un débit noté par modèle et par
  origine, dans l'unité de la capacité ;
- **les tests** : la même variable, honorée par `tests/common` ; les suites
  qui éprouvent le moteur lui-même restent locales, par la liste qui existe.

## 4. Ce qui reste propre à chaque capacité, et la frontière

La frontière passe **au trait**. `Embedder`, `Reranker`, `Ocr`, `Llm` gardent
leur forme : des textes contre des vecteurs, une requête et des passages
contre des scores, une image contre du texte. L'abstraction ne les fond pas
en un seul — ce serait un type qui ne veut plus rien dire. Elle demande à
chaque capacité trois choses, et rien d'autre :

1. construire son client local ;
2. construire son client de service (pour un démon à nous : une route) ;
3. dire l'unité de son débit (caractères, paires, pages, jetons).

Ajouter la voix (`stt`, `tts`) demain, c'est écrire un trait et ces trois
points ; la déclaration, les adresses, le repli, le régulateur et les tests
sont déjà là.

## 5. Ce qui migre, et dans quel ordre

| Lot | Contenu | Dépend de |
|---|---|---|
| 1 | La déclaration, sa résolution, les variables et l'alias. **Embarquement dense** migré : `embeddings` au manifeste devient un alias de `models.embed`. | rien |
| 2 | **Décision**, premier client neuf : un trait (`Decider` : un état, un critère, des options → une probabilité par option) et le fournisseur `llama_server`. Si le nœud de la session mémoire se déclare « service llama-server à telle adresse » ou « local » sans code à lui, l'abstraction est la bonne. | lot 1 ; le format de prompt du client Python de la session optimiseur |
| 3 | **Creux et dual** branchés par le backend (le défaut trouvé au § 1). | lot 1 |
| 4 | **Relecteur et OCR** déclarés `local` au manifeste : le backend leur donne enfin un service. | lot 1 |
| 5 | Relecteur et OCR **en service** : deux routes de plus au démon. | lot 4 |
| 6 | **LLM** : `models.llm` remplace la section propre au chat ; `RAG3WEAVER_LLM` et `RAG3WEAVER_LOCAL_LLM` restent comme alias. | lot 1 ; à faire avec la session qui tient le chat |

Les lots 1 à 4 ne changent le comportement de personne tant qu'on ne déclare
rien de neuf. Le lot 6 touche au régime (`confort` choisit aujourd'hui
l'origine du LLM) : il attend.

## 6. Ce qui pourrait demander un choix

Rien ne bloque les lots 1 à 4. Trois points sont tranchés ici par défaut ;
dire si l'un d'eux ne convient pas :

- **Le nom de la clé** : `models`, au manifeste du backend.
- **Le nom des variables** : `RAG3WEAVER_SERVICE_<CAPACITÉ>`.
- **Le repli par défaut** : `refuse`.
