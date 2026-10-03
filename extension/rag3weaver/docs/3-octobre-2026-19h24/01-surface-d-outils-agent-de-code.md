# La surface d'outils de l'agent de code, servie par le backend déclaratif

3 octobre 2026, session recherche — proposition sans code, demandée par
l'orchestration pour les deux produits tranchés par Lucie (« ok pour avancer
en parallèle sur les deux produits code ») : un agent cloud qui télécharge
un dépôt git, un agent en ligne de commande sur le disque. **Un seul moteur,
deux politiques.** Sources : repérage sur `806757dc8`, réponses directes de
la session arbre principal (l'auteure du backend côté MTG) et de la session
embarquements (l'entrée « indexer ce dépôt »).

## 1. L'inventaire

Les **onze verbes de l'agent de code sont déjà des graphes-outils** (`.mmd`
sous `templates/tools/`, chargés par `builtin_graph_tools`, feature `code`) :
`search`, `read`, `grep`, `list`, `edit`, `run`, `run_bg`, `wait`, `schema`,
`place`, `adopt`. Sous les graphes, trois pièces de Rust en dur :

- **`FileSource`** (code_tools.rs) — l'accès disque : `WorkingTree` (lit et
  écrit le disque, écriture atomique) ou `Snapshot` (tout en mémoire).
  Chemins relatifs, `..` refusé, une source en lecture seule refuse d'écrire.
- **`commande::Garde`** — la porte de `run`/`run_bg` (modes Standard,
  Approbation, Auto) ; sans le service `garde`, `run` refuse tout.
- **`reingest_file`** — la réindexation, **automatique après chaque `edit`**
  (pas un verbe) : réanalyse le fichier, upsert, supprime les scopes
  disparus. Deux dettes connues : les relations inter-fichiers ne sont pas
  recalculées, et l'entité `SCOPE` est codée en dur — la session arbre
  principal la rebranche en ce moment sur la synchronisation déclarée
  (`code_sync.rs`) : ce point se règle de son côté.

Ce que les suites éprouvent déjà : `e2e_code` (24 — les onze verbes sur un
vrai dépôt, les seuils-canaris de la fusion), `e2e_agent_loop` (8 — la
boucle d'agent complète avec traces à deux étages), `e2e_graph_tool` (4 —
la mécanique des gabarits), `e2e_cloud_code_agent` (le montage
`WorkingTree` + modèle local par `RAG3WEAVER_LOCAL_LLM`). **Aucun binaire de
production ne monte ces verbes** : seul le chemin de test
(`mount_agent_services_on`) le fait.

Côté backend déclaratif (`rag3weaver-backend <backend.json>` + chat
`rag3weaver-chat`, un seul usage réel : le deck builder MTG) : un outil est
un `.mmd` **référencé par chemin** (`graph:`), avec `bindings` (paramètres
figés), `input_payloads` (schéma dérivé d'une entité), `harness`
(before/after/on_accept en Rhai, rapport complet, pas le premier échec). Le
chat est neutre : il reprend `describe` → outils, liste blanche
`allowed_tools`, n'importe quel endpoint compatible OpenAI.

## 2. Ce qui passe tel quel — et les quatre clés qui manquent

**Tel quel** : les onze `.mmd` s'attachent par chemin sans réécriture ; leurs
descriptions (`%% description:` des fiches) sont déjà soignées ; `search`
porte la pondération par genre déclarée dans `Scope` et prendra le couple de
fusion que Lucie choisit — par l'échelle du pas C, le manifeste n'a rien à
en savoir ; « suivre une relation » (qui appelle, qui est appelé) est déjà
le paramètre `relation` de `search`, pas un outil de plus.

**Les quatre manques, vérifiés des deux côtés** (repérage + auteure MTG) :

1. **La feature `code`** n'est pas dans les `required-features` du binaire —
   les fabriques des nœuds de code n'existent pas dans `rag3weaver-backend`.
2. **L'espace de travail n'est pas déclarable.** Une clé de manifeste :
   `workspace { source: snapshot | working_tree, root, read_only, commands:
   off | approbation | auto }` — `execute_plan` monte alors `file_source`,
   `file_access` et `garde`. C'est exactement la coupe des deux produits :
   cloud = un dépôt git cloné dans un espace géré (et, au plus simple,
   `snapshot`) ; poste = `working_tree` sur le dossier de l'utilisateur.
3. **La politique est une liste de nœuds unique, en dur** (backend.rs:882) —
   elle exclut tous les nœuds de code. Il faut une **politique par outil**
   dans `ToolAttachment` (lecture / écriture / exécution), le mode de
   `Garde` par backend ou par outil. L'auteure MTG le dit en propres
   termes : ces deux clés (workspace + politique par outil) « sont le cœur
   de ta proposition » — les deux produits ne diffèrent que par elles.
4. **La description d'entité dans le manifeste**, reprise par les
   descriptions d'outils générées — la ligne du journal, jamais faite, dont
   MTG a payé l'absence (tous les `search_*` disaient le même texte, l'agent
   choisissait au hasard du nom). Pour le code : `Scope` décrit (« un
   morceau nommé de code source : fonction, classe, module… ») injecté dans
   `search`/`read`/`edit` qui le visent ; et un champ `description`
   optionnel sur `ToolAttachment` pour surcharger une phrase sans dupliquer
   un gabarit. Règle apprise de MTG : la description dit **quand prendre
   l'outil plutôt qu'un voisin**, pas ce qu'il fait techniquement.

**L'édition sur le disque est un outil du backend, pas de l'hôte** : `edit`
écrit dans la `FileSource` que le manifeste déclare — la politique choisit
la cible (mémoire ou disque), le mécanisme ne bouge pas. C'est la même
coupe que l'entrée d'indexation de la session embarquements (« la
différence est dans ce que la politique autorise comme source, pas dans le
mécanisme »).

## 3. La plus petite surface utile

Pour un premier agent : **`search`** (pondération et fusion comprises),
**`read`**, **`grep`**, **`list`**, **`schema`**, la relation par le
paramètre de `search` ; **`edit`** (réindexation automatique incluse, ses
deux dettes nommées) ; **`estimate`** et **`index`** — les verbes
d'amorçage de la session embarquements (l'estimation en lecture ;
l'indexation rend un **reçu** et avance en fond), le reçu attendable par le
**`wait`** existant (sa page : `docs/3-octobre-2026-14h26/03-indexer-ce-depot.md`),
qui rend le niveau de disponibilité (« plein texte prêt, vecteurs à 40 % »)
en lignes de journal — `timeout_s=0` fait la lecture d'état sans attendre,
le paramètre existe déjà ; un type de plus ne se justifie pas. `run`/
`run_bg` : politique poste seulement d'abord, garde en mode approbation.

**Pour après** : `place`/`adopt` (le catalogue de gabarits n'est pas un
verbe de codage), le rerank par défaut, les familles de dérivées, `run` en
politique cloud.

## 4. La preuve — deux cases, la règle de Lucie

« Local quand c'est juste pour valider un flux ou un protocole, Gemini
quand c'est vraiment une vraie expérience de codage. »

- **La tuyauterie, en local** (gratuit, dans les batteries) : un scénario
  scripté sur une base jetable — l'outil est appelé avec les bons
  arguments, le harnais refuse ce qu'il doit (argument hors schéma, outil
  hors liste blanche), la garde refuse une commande non approuvée, `edit`
  déclenche la réindexation, la boucle se termine. Montage existant :
  `llm-serve` (llama-server :8080, `LLM_SERVE_NO_MCP=1` contre le verrou
  « un seul hôte par base »), ou `--demo`/`MockLlm` pour le plus
  déterministe ; les `scripts/test_backend_*.py` montrent la forme. Leçon
  MTG à appliquer ici : un refus de harnais doit dire **quoi faire**
  (l'outil ou le paramètre à changer), pas seulement la règle violée —
  c'est sur l'agent faible que ça se voit.
- **La vraie expérience de codage, avec Gemini par Vertex** (le modèle de
  référence — crédits startup, le frein est le réseau, pas le coût) : sur
  ce dépôt indexé (`estimate` → `index` → `wait`), une tâche réelle —
  « trouve pourquoi X se comporte ainsi et corrige-le » — jugée sur
  trouver/comprendre/modifier juste. Le chemin existe : `llm.rs` gère
  finement Gemini 3.x (le `thought_signature`, la reprise local→Gemini, un
  budget mesuré sur Vertex avec `gemini-3.5-flash`), le chat parle à tout
  endpoint compatible OpenAI, les identifiants sont dans `.vault`
  (`lr-hub-472010`). Hors batteries par défaut, annoncée, **le compte
  d'appels rendu à chaque passe**, rejouée autant qu'il faut — et un
  « 0 passed » est un saut à corriger, jamais une ligne verte. En garder
  l'artefact : faire dire au modèle ce que nos outils rendent possible et
  ce qui lui manque (la mémoire du projet le demande).

Il en faut **un de chaque** : la tuyauterie prouve que le protocole tient
avec un agent faible ; Gemini juge ce que l'agent sait faire de nos outils.

## 5. Ce que je coderais en premier

1. **La description d'entité au manifeste** (+ `description` sur
   `ToolAttachment`) — la journée recommandée depuis MTG, dont tout hérite.
2. **La clé `workspace`** et le montage `file_source`/`file_access`/`garde`
   dans `execute_plan` ; la feature `code` au binaire.
3. **La politique par outil** (lecture/écriture/exécution + mode de garde) —
   le cœur : c'est elle qui fait deux produits d'un moteur.
4. Un **`templates/backends/code/backend.json`** d'exemple : `Scope`/`File`
   décrits, la surface du §3 attachée par chemins, les deux politiques en
   deux manifestes ; le `chat.json` qui va avec ; le scénario tuyauterie en
   script.
5. **`estimate`/`index`** branchés quand l'API en deux temps de la session
   embarquements existe ; puis le scénario Gemini.
