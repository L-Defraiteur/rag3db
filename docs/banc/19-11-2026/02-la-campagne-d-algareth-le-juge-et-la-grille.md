# La campagne des correctifs passés d'Algareth : le format, le juge, la grille (19 novembre 2026)

Le ticket est `docs/banc/tickets/2026-11-19-juger-la-campagne-des-correctifs-passes-d-algareth.md`, et le chantier
vient de Lucie. Chaque session fournit des correctifs réels, Algareth corrige sans git, le banc juge et compare, et
archi tient le lanceur. Cette page fixe la forme, avant le premier cas joué. Elle part des deux premiers cas de tty
(`docs/tty/19-11-2026/08-` et `09-cas-de-campagne-*.md`, branche `cx-campagne-cas-tty`, e8729dd559).

## 1. Le format d'un cas

Un cas est une page de docs de la session qui le fournit : `docs/<session>/<date>/NN-cas-de-campagne-<nom>.md`.
Ses sections, dans cet ordre (celui de tty) :

1. **Le point de départ** : le commit parent (40 caractères dans le relevé du lanceur), la date, et l'arbre minimal
   à fournir à Algareth (par exemple « `extension/rag3weaver/tty` suffit »).
2. **Le signalement, tel qu'il était** : le texte qu'Algareth reçoit, cité. C'est la seule description du défaut
   qu'il a.
3. **Les témoins** : pour chacun, le filtre (le nom complet du test) et son fichier ; la suite (`cargo test` du crate,
   `lib`, ou une suite e2e nommée) ; **le rouge attendu au parent**, c'est-à-dire l'assertion qui échoue et pourquoi.
   On dit aussi si le témoin est identique sur master (vérifié, avec le hash).
4. **Les suites exigées par les chemins** (`tools/poste/required_suites.txt`).
5. **Les indices présents** dans l'arbre du parent : une souche posée avec le rouge (`tui::nom_garde -> None` dans le
   cas 09), un test rouge en place, un `todo!()`. On les liste tous. **Le cas ne les retire pas** : Algareth les voit,
   et la comparaison en tient compte (§ 3).
6. **Notre correctif** (caché à Algareth, pour le juge) : le commit, son titre, les fichiers avec leurs `+/−`, et
   ce qui est hors du cas (une page de docs, par exemple).

**Ce que le lanceur donne à Algareth** : l'arbre minimal au parent, avec un `.git` neuf sans histoire (§ 4), et le signalement. Il ne donne ni le
nom du témoin ni notre correctif. Le témoin est dans l'arbre, comme tout test : Algareth peut le trouver et le jouer.

## 2. Le juge

Le juge est **le témoin**. Il se joue dans l'ordre suivant, sous `poste lourd` à luciepc, sur un worktree à part par
cas.

1. **Le rouge, prouvé au parent avant la campagne** : le témoin joué tel quel sur le parent doit échouer **sur son
   assertion**, celle que dit le cas. Une erreur de compilation n'est pas un rouge : **on ne fabrique pas de rouge
   par compilation**. Si le témoin ne compile pas au parent sans aide, le cas l'a déjà prévu par une souche, qu'il
   dit. Sinon le cas est renvoyé à sa session.
2. **Après Algareth** :
   - le témoin passe ;
   - les suites exigées par les chemins passent, entières ;
   - **le témoin n'a pas été touché** : le corps du test et ses assertions sont identiques au parent (diff des
     fonctions de test du témoin).
3. **Le verdict**, un seul par cas :

| verdict | quand |
|---|---|
| **réussi** | le témoin passe, les suites exigées passent, et le témoin est intact |
| **échoué** | le témoin reste rouge |
| **dégâts** | le témoin passe, mais une suite exigée rougit ailleurs |
| **témoin touché** | le témoin passe parce qu'il a été modifié ou retiré |
| **non jugé** | ça ne compile pas, ou le témoin est sauté ; un test sauté n'est pas un test vert |

## 3. La grille de comparaison

Une ligne par cas, dans la page de la campagne (`docs/banc/<date>/NN-la-campagne-d-algareth.md`, ouverte au premier cas
joué) :

| colonne | ce qu'elle dit | comment elle se lit |
|---|---|---|
| cas | la session et le nom | le titre de la page du cas |
| verdict | § 2 | le juge |
| **endroit** | les fichiers et fonctions changés par Algareth, contre les nôtres : **même**, **proche** (même fichier, autre fonction) ou **autre** | `git diff --stat` et les noms de fonctions du diff, par le lanceur après le tour d'Algareth |
| **taille** | `+/−` d'Algareth, contre `+/−` de notre correctif, hors docs | le même diff |
| **coût** | les pas de l'agent, les appels au modèle, les jetons d'entrée et de sortie, la part servie par le cache, et la durée | la trace de l'agent (archi) |
| **dégâts à côté** | les fichiers touchés sans lien avec le défaut, et les tests ou suites hors témoin qui changent de couleur | le diff et les suites exigées |
| **indices présents** | les indices du § 1.5 que le cas déclare, et si Algareth les a suivis (il a rempli la souche, ou il a contourné) | la page du cas et le diff |

**Le poids des indices** : un cas avec une souche posée avec le rouge est plus facile qu'un cas sans indice. La page
de la campagne compte séparément les réussites avec indice et celles sans. Ce sont deux nombres, pas un seul taux.

## 4. La sortie du lanceur, convenue avec archi (19 novembre)

**Où** : sur le poste qui a joué, `~/.cache/rag3weaver-build/campagne/<cas>/<run>/`, où `<run>` vaut
`AAAAMMJJ-HHMMSS`. Le lanceur ne juge rien : aucun verdict dans ce dossier. On y trouve :
- `case.json` : le bloc « lanceur » du cas, et l'empreinte du texte donné ;
- `request.txt` : le signalement, mot pour mot, tel qu'Algareth l'a reçu ;
- `diff.patch` et `diffstat.txt` : `git diff base..fin` sur l'arbre de la tâche, nouveaux fichiers compris. C'est la
  source des colonnes « endroit », « taille » et « dégâts à côté » ;
- `trace.json` : `steps`, `model_calls`, `input_tokens`, `output_tokens`, `cached_tokens`, `reasoning_tokens`,
  `wall_ms`, `model_ms`, `tool_calls` par outil, `ended` (la ligne de fin de la boîte) et `stopped_by`. C'est la
  source de la colonne « coût » ;
  **`input_tokens` ne compte que la part de l'invite non servie par le cache** : l'invite totale est
  `input_tokens + cached_tokens` (vérifié par archi le 19 novembre ; le champ devient `uncached_input_tokens`, avec un
  `prompt_tokens` total, au lot suivant du lanceur) ;
- `mailbox.jsonl` et `runs/` : la boîte et les journaux bruts, pour relire sans relancer ;
- `budget.json` : le budget appliqué.

**Le budget d'un tour** : 12 pas, 2 M de jetons d'entrée cumulés, 30 min d'horloge, par défaut. Il se règle par cas
dans le bloc `budget`. Au premier dépassement, le lanceur écrit stop dans la boîte et `stopped_by` dans `trace.json`.
**Un « échoué » se lit en deux** : avec `stopped_by` non nul, il a manqué de budget ; sans, il a manqué de justesse.
La grille porte les deux.

**L'arbre a un `.git`, mais neuf** : un `git init` et un seul commit `base`, sans aucune histoire, parce que la tâche a
besoin d'un dépôt pour sa branche et son worktree. La porte de la campagne refuse toute commande git. Algareth ne peut
donc ni lire ni écrire l'histoire, et il n'y en a pas. C'est aussi ce qui rend `diff.patch` exact.

**Le juge rejoue** le témoin et les suites exigées sur un worktree du dépôt réel au parent, avec `diff.patch`
appliqué. Il ne les rejoue pas dans l'arbre de la tâche.

Le juge commence au premier cas que le lanceur aura joué.
