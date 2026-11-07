# Journal de la session hooks-essai (7 novembre 2026)

Session Claude Code seule avec les hooks de rag3weaver vers la mémoire longue
(worktree `rag3db-hooks-essai`, branche `hooks-essai`). Pour chaque tour qui
compte : ce que le bloc `[mémoire]` a montré, ce que j'en ai fait, ce qui a
aidé ou gêné. Matériau du juge et de Lucie.

## Tour 1 — SessionStart, puis le premier message de c7

**Bloc SessionStart, mot pour mot :**

> [mémoire]
> Aucune fiche d'identité ne cite cette session. Si tu as une clé (ton nom
> court, celui que les autres sessions emploient), écris ton L1 `handle` :
> `put_apport` {fiche: <clé>, source: "sessions", section: "handle", texte:
> "session:494dbbc6-f2ef-4f2e-91e7-6232d8a49424", niveau: "L1", via: "agent"}
> puis `relier_apport` et `citer_source` genre `handle` valeur
> `session:494dbbc6-f2ef-4f2e-91e7-6232d8a49424`. Si tu es nouvelle, crée
> d'abord ta fiche d'identité : `put_fiche` {name: <clé>, about: <une ligne>,
> state: "current"}. Rien n'est créé à ta place.

**Bloc UserPromptSubmit (message de c7), mot pour mot :**

> [mémoire]
> Fiches en rapport : « régime GPU » (Comment on ménage les cartes du poste.)
> — `get_fiche` pour lire.

**Ce que j'en ai fait :**

- `get_fiche hooks-essai` : absente ; `put_fiche` (révision 1).
- `put_apport` L1 `handle` (45:1) et L1 `identity` à sept rubriques (who,
  runtime, can, doing, holding, mandates, peers) en un seul texte (45:2) ;
  `relier_apport` des deux, 2/2 vérifiés.
- `citer_source` genre `handle` : refusé, le genre n'existe pas (seuls `doc`
  et `fichier`). Je n'ai pas créé le genre : il est global et durable, la
  décision revient à c7 ou Lucie. c7 : 96 crée `agent`, `alias`, `handle`,
  `same_as` ; je rejouerai `citer_source` à son signal.
- Fiche « régime GPU » non lue : sans rapport avec la demande (un lot d'IDE).

**Ce qui a aidé :** le bloc donne les appels exacts, avec l'uuid de session à
coller ; « rien n'est créé à ta place » dit clairement que c'est à moi.

**Ce qui a gêné (transmis à c7) :**

1. Le hook prescrit un genre de Ref (`handle`) qui n'existe pas.
2. Il fait passer `via: "agent"`, alors que le schéma dit `via` et `author`
   posés par l'hôte (lot M7) ; les deux sont revenus `null`.
3. `source: "sessions"` n'est pas un chemin `sources/<date>-<uuid>.md` comme
   le schéma le décrit, et il est pourtant accepté.
4. Le bloc ne dit pas la forme attendue de la fiche (`hooks-essai` ou
   `agent:hooks-essai`) ; j'ai pris la clé nue, comme il l'écrit.
5. La seule fiche « en rapport » au premier tour (« régime GPU ») ne l'était
   pas : le message parlait de hooks, d'IDE et de recherche.

## Tour 2 — le lot « chercher sans dossier »

Le bloc de ce tour n'a rien montré de neuf (pas de fiche en rapport, pas de
diff). Diagnostic, lu dans le code :

- L'accueil React (`ui/ide/src/views/Home.tsx`) n'a pas d'entrée de
  recherche ; la vue `Search.tsx` ne demande aucun dossier, mais on n'y entre
  que par l'URL.
- Le seul formulaire est celui de l'ancienne page (`src/ide/page.html`,
  `<form id="chercher" hidden>`), révélé par `filDAriane()` (`src/ide/page.js`)
  quand une vue d'arbre, de fichier ou de symbole pose son fil d'Ariane. Ce
  n'est pas une règle : le formulaire a besoin de l'id d'un backend, et le fil
  d'Ariane était le seul à le connaître. L'accueil liste plusieurs backends et
  ne le montre jamais.
- Le dossier ne filtrait rien, alors que `search_code` prend `scope`
  (`templates/tools/search_workspace.mmd`).
- Sur `ide-react-vue3b`, le `Tree.tsx` React n'a plus de formulaire du tout.

Question posée à c7 : baser le lot sur `ide-react-vue3b` (proposé) ou sur
master.

## Tours 3 à 6 — les réponses de c7, la forme de 96, le code

**Ce que les blocs ont montré :** rien de neuf (ni fiche en rapport, ni diff,
ni verdict). Le contexte utile est venu des messages de c7 et
d'everything declarative, pas du hook.

**Mémoire :**

- 96 a créé les genres `agent`, `alias`, `handle` et `same_as` ;
  `citer_source` genre `handle` rejoué sur le L1 `handle` : exécuté, « genre
  sans harnais : citée, non vérifiée ».
- La forme demandée par 96 : `author: "hooks-essai"` et `via: "claude-code"`.
  Le L1 `identity` est réécrit en sept lignes, une rubrique chacune (révision
  2, `expected_revision: 1`). **Écart à dire :** le schéma décrit un L1 comme
  une archive qui « ne change plus une fois posé », et la révision a pourtant
  été acceptée. Faut-il réécrire, ou poser un L1 neuf et consolider ? Le hook
  ne le dit pas.

**Le lot :** basé sur `ide-react-vue3b-2` (37c42b720), branche
`chercher-sans-dossier`. Fichiers partagés avec everything declarative : ses
lots suivants ne touchent ni Home, ni Tree, ni Search ; aucun de ses témoins
ne lit `#chercher`.

- `aed52e422` : `SearchBox` sur l'accueil, l'arbre et la recherche ; le
  paramètre `dossier` part dans `scope` de `search_code` ; la pastille
  `#chercher-dossier` s'efface d'un clic ; bundle rebâti, `--check` vert,
  `tsc` vert.
- `de77cf72c` : le témoin navigateur
  `la_recherche_se_tape_depuis_l_accueil_et_le_dossier_n_est_qu_un_filtre`.
  Le même mot dans `src/` et dans `outils/`. Depuis l'accueil, la requête
  tapée rend les deux fichiers ; depuis l'arbre de `src/`, un seul ; après le
  clic sur la pastille, la même question rend de nouveau les deux.

**Ce qui a aidé :** les repères DOM écrits en tête de chaque vue
(`Search.tsx`, `Home.tsx`) et les commentaires « pourquoi » de l'ancienne page
ont donné le diagnostic en trois lectures.

**Ce qui a gêné :** le journal vit sur `hooks-essai` et le lot sur
`chercher-sans-dossier` dans le même worktree. Il a fallu un worktree de docs
à part (`~/.cache/rag3weaver-build/wt-docs-hooks-essai`). La consigne « un
worktree par session » ne dit pas où vont les docs de session quand la
branche de la session change.

## Tours 7 et 8 — la passe à luciepc

**Ce que les blocs ont montré :** « Fiches en rapport : « hooks-essai » … ;
« régime GPU » … ». Ma propre fiche apparaît maintenant à chaque tour : utile
pour me rappeler ce que j'ai déclaré, mais elle n'apporte rien que je n'aie déjà.
« Régime GPU » revient à chaque tour sans rapport avec le lot (un témoin sans
embarquement) : du bruit.

- Passe 1 (de77cf72c) : le bundle et les vues de l'IDE sont verts ; mon témoin
  est rouge par sa propre faute (`search_code` sans `options`, 422 pendant
  l'attente de l'index).
- Passe 2 (35aede9f3) : mon témoin est vert seul, 1 passed, 0 ignored, code de
  sortie 0.
- Restent `check_features.sh` et la lib entière, après l'atterrissage de la 3b
  et mon rebase ; master au mot de c7.

## Bilan du bloc [mémoire] sur le lot « chercher sans dossier »

Ce que le bloc a montré pendant le lot, tour par tour :

- **Au premier tour :** la consigne d'identité et « régime GPU » en rapport.
- **Aux tours suivants :** rien, ou bien « hooks-essai » (ma propre fiche)
  et « régime GPU ».
- **Jamais :** une fiche sur l'IDE, la recherche, `search_code` ou `scope`,
  un diff d'une autre session sur les fichiers touchés, un plan en pause, un
  verdict du juge.

**Ce qui m'a servi :** la consigne d'identité seule, parce qu'elle donnait les
appels exacts. Rien du bloc n'a servi au code. Le diagnostic est venu de la
lecture des fichiers, et l'accord sur les fichiers partagés d'un message
d'everything declarative. Le bloc aurait pu le dire de lui-même : « Tree.tsx
est né dans la vue 7 d'everything declarative, pas encore sur master ».

**Ce qui aurait servi :** au tour du diagnostic, une fiche sur l'IDE en React
(page 08, l'ordre des vues, les repères DOM des témoins) ; au tour du témoin,
la forme des arguments de `search_code` (`options` est exigé). Ce sont deux
choses que la mémoire de Lucie connaît : la fiche « L'IDE en React, vues 3 à
7 » existe dans la mémoire de Claude Code, et le hook ne l'a pas trouvée.

**Bruit :** « régime GPU » à chaque tour, sans rapport avec le lot ; ma propre
fiche à chaque tour, qui ne m'apprend rien.

En cours : `check_features.sh` puis la lib entière sur 35aede9f3, à luciepc
sous `poste lourd`.
