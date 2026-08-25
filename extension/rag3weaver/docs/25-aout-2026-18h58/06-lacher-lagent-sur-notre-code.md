# 06 — Lâcher l'agent sur notre propre code : le 0,5 B et Gemini

25 août 2026, 23h. Lucie : *« essayons, ça peut être rigolo et nous
redonner du baume au cœur »*. Le graphe de `src/dataflow/` (25 fichiers,
1 194 scopes, 9 934 relations), les quatre outils — `grep`, `read`, `search`,
`search_expand` — et trois questions :

1. *Where is the function `take_results` defined, and which method calls it?*
2. *What does `FuseResultsNode` do with its `signals` input port? Read the code before answering.*
3. *Which node types are registered by `register_builtins`? List a few with the file they live in.*

Deux tests, `e2e_burn_code_agent` (Qwen2.5-0.5B hors ligne) et
`e2e_cloud_code_agent` (Gemini 3.5 Flash via Vertex, quelques centimes). Ils
n'affirment que la forme de l'historique ; le reste est un rapport, imprimé.

## 1. Le 0,5 B : la plomberie tient, le modèle non

| | appels | réponse |
|---|---|---|
| Q1 | 0 | *« `take_results` est défini dans la fonction `search` »* — inventé |
| Q2 | 0 | *« … passe les signaux à la méthode `onSignal` »* — inventée |
| Q3 | 1, en erreur | appelle un outil **`register_builtins`** avec une liste d'arguments en boucle (`NodeTypeNameNameName…`) ; reçoit `{"error":"unknown_tool", "connus : grep, read, search, search_expand"}` ; s'excuse |
| Q1 forcé + exemple | 0 | *« défini dans `src/func.rs` »* — inventé ; `ToolChoice::Required` n'est pas contraignant pour un modèle local |
| Q2 forcé + exemple | 1, en erreur | `search_expand(target="FuseResultsNode", relation="HAS_SIGNALS")` — une cible qui est un nom de scope, une relation qui n'existe pas |

Ce qu'on garde : **les erreurs sont lues.** Chaque `{"error": …}` a été
compris et repris dans la réponse. Et le 0,5 B n'est pas un agent de code —
on le savait, c'est mesuré.

## 2. Gemini : juste, en huit secondes

| | itér. | appels | jetons | temps | réponse |
|---|---|---|---|---|---|
| Q1 | 5 | 4 (`grep`, `grep`, `read`, `read`) | 8 500 | 8,6 s | **juste** : défini `generic_search_nodes.rs` 1016–1020, signature exacte ; appelé par `FuseResultsNode::execute` (653–717), lignes 664 et 668 |
| Q2 | 6 | 5 | 32 687 | 27 s | **juste et complète** : « fan-in », définition du port (639–644), regroupement par étiquette, ordre de première apparition, poids… |
| Q3 | 8 | 8, 1 erreur | 30 379 | 70 s | **pas de réponse** — `MaxIterations` |

Le modèle s'est servi **directement des scopes annotés** : `read` dit
*« Scopes: function `take_results` (1016-1020), … »*, et c'est ce qu'il cite.
`grep` puis `read` à l'offset rendu — le motif attendu, sans qu'on l'ait
montré.

## 3. Ce que la Q3 a révélé, et ce qu'on a corrigé dans la foulée

Première passe : le modèle a demandé `read("src/dataflow/node_factories.rs")`
— un préfixe de répertoire deviné, alors que la source est enracinée dans
`src/dataflow`. *« no such file »*, et il a erré. Trois corrections :

- **« Vouliez-vous dire »** sur chemin inconnu : même nom de fichier ailleurs,
  ou un chemin dont le demandé est un suffixe. Seconde passe : l'erreur dit
  *« did you mean: node_factories.rs »* et **le modèle se corrige au tour
  suivant**.
- **`EntityConfig.return_fields`** : ce qu'une recherche rend en plus du
  titre et des contenus. Un `Scope` trouvé par `search` dit désormais
  `file_path`, `start_line`, `end_line`, `scope_type`, `parent_name` — sans
  quoi il n'est pas lisible.
- **La fiche de `search_expand`** nomme les relations d'un graphe de code et
  ce que rend l'expansion.

Et `max_tokens` 700 → 1 500 : la Q2 était coupée en plein « fan- ».

Ce qui reste, et c'est la vraie leçon : à la Q3, le modèle **énumère les
fabriques une à une** — `grep "struct ComposeNodeFactory"` (0 résultat :
elles sont déclarées par la macro `named_factory!`), `grep`, `read`, `search`
par fichier… huit tours, rien. **Le graphe avait la réponse en un appel** :
`register_builtins CONSUMES` chaque `*Factory`, chacune `DEFINED_IN` son
fichier. Le modèle n'a pas pris `search_expand`, même décrit. Deux pistes,
non faites : un système qui dit *« pour les questions « qui utilise quoi »,
`search_expand` d'abord »*, et surtout des **`enum` dans le schéma** des
outils (cibles et relations réelles, tirées du catalogue) — un modèle ne peut
pas inventer `HAS_SIGNALS` si le schéma ne le permet pas.

## 4. Deux défauts de plomberie trouvés en passant

- **La clé de service dans `secrets/` était l'ancienne**, révoquée en
  octobre ; celle du matin est dans `.vault/vertex-sa.json`. Et
  `gcp_auth` **avalait le corps du 400** — `ureq` lève une erreur sur les
  statuts non-2xx par défaut, le même piège que celui corrigé le matin dans
  `openai_llm` ; corrigé, le message de Google (`invalid_grant`) arrive
  désormais.
- Un test d'`openai_llm` comptait 28 nœuds en dur — la feature n'avait pas
  été retestée depuis `RerankNode`. `BUILTIN_NODE_COUNT` partout.

## 5. Chiffres à garder en tête

Une question = 8 000 à 33 000 jetons (le contexte est renvoyé à chaque
tour). C'est le prix du « lire avant de répondre » ; c'est aussi l'argument
pour que les outils rendent **peu et juste** — le tableau markdown de `grep`
plutôt que du JSON, les scopes de la fenêtre plutôt que le fichier.
