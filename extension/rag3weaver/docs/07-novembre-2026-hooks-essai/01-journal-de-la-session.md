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

## La passe complète sur la base (35aede9f3)

- `check_features.sh` : 5/5 verts.
- La lib entière : 1865 passed, 0 failed, 37 ignored, en 26,5 s. Elle est
  jouée sans `anthropic-llm`, une feature que la base 37c42b720 ne connaît pas
  encore. Le premier essai s'est arrêté net sur « does not contain this
  feature », avant toute compilation. Le jeu complet se rejouera après le
  rebase sur master.
- Le bloc `[mémoire]` est resté le même à chaque tour (« hooks-essai »,
  « régime GPU ») et n'a rien apporté à la passe.

## Tour de 15 h 46 — le binaire du lot 3 des hooks, et aucun bloc

c7 annonçait un binaire posé à 15 h 42 : le bloc devait me retrouver par mon
handle et donner le score des fiches rappelées.

**Ce que le bloc a montré :** rien. Aucun `[mémoire]` n'a été injecté sur ce
tour. Le journal du hook (`~/.local/state/rag3weaver/hooks.log`) :

> 2026-11-07 15:46:02 UserPromptSubmit 494dbbc6-… : hôte joint, backend
> « memory », prompt de 638 caractères [rag3weaver-backend@1794062140 ;
> gabarits 1794062534]
> 2026-11-07 15:46:02 recall_fiches : /call : l'hôte http://127.0.0.1:7970 ne
> répond pas : timeout: global

Aucune ligne « bloc de N caractères » ne suit.

**Ce que j'en tire :**

- Le hook a tenu sa promesse de ne jamais bloquer la session.
- Mais il s'est tu entièrement : même ce qui ne dépend pas du rappel (le
  handle, ma fiche) n'est pas passé.
- Vu de l'agent, un silence ne se distingue pas de « rien en rapport ».
- Proposé à c7 : un bloc partiel, qui dit « rappel des fiches : hôte trop lent
  (5 s) ».

**Vu aussi :** un « [rejeu] SessionStart » de ma session à 15:43:28 (bloc de
993 caractères), qui ne m'est jamais parvenu. Un rejeu hors de Claude Code
n'injecte rien dans la conversation : le juge ne doit pas le compter comme vu
par l'agent.

## Tour de 15 h 47 — toujours aucun bloc, et une base de deux fiches

**Ce que le bloc a montré :** rien, pour la deuxième fois.

- Le journal du hook : `15:47:12 recall_fiches : /call : l'hôte
  http://127.0.0.1:7970 ne répond pas : timeout: global`, puis aucune ligne
  « bloc ».
- À une minute de la précédente, on ne peut plus parler d'un hôte froid.
- `/sante` répond en 0,3 ms ; la charge du portable est à 11,4.

**Le rappel fait à la main** par le MCP memoire-longue, sur une phrase du
message de c7, a répondu en 0,36 s. Le moteur répond vite : c'est le chemin
`/call` du hook vers 7970 qui dépasse ses 5 s.

**Il a rendu 2 résultats en tout :**

- « régime GPU », ★ 0,0163 ;
- « hooks-essai », ★ 0,0162.

Le plein texte dit « rien de probant », avec 0,68 contre un seuil de 0,88.
La base `memory` du hook ne compte que ces deux fiches. Elles revenaient donc
à chaque tour faute d'autre chose, pas parce qu'elles étaient en rapport.

**Ce que j'en tire (proposé à c7) :**

- cacher les fiches sous un seuil de score plutôt que les dire « en rapport » ;
- mesurer l'essai sur une base peuplée, sinon le juge mesure une base vide.

## Ce que j'aurais voulu que la mémoire me rappelle (à ranger en premier)

Tour de 15 h 48 : toujours aucun bloc (`recall_fiches … timeout: global`).
c7 explique que 5 s est le budget total du hook pour des appels en série ; le
lot 3 bis les met en parallèle. La base `memory` ne compte que deux fiches
jusqu'au pas 3 de l'archiviste.

La liste ci-dessous suit l'ordre des moments du lot. Pour chaque point : ce que
j'ai dû trouver seule (en lisant, en demandant ou en me trompant), et que la
mémoire aurait pu me dire.

1. **À l'arrivée (SessionStart).**
   - Qui orchestre (c7 = rag3db-74), sur quelle base et avec quelle lib
     épinglée (04063ee06 et son identifiant complet du magasin), et où passer
     le lourd (luciepc, `poste lourd`, deux places à la fois).
   - Où écrire : le journal de la session, et un worktree de docs à part
     quand la branche du lot n'est pas celle de la session.
   - La forme de mes propres apports (`author` = ma clé, `via` =
     `claude-code`, identity en sept lignes) : je ne l'ai apprise qu'au
     troisième message de c7.
2. **Au diagnostic (« la recherche exige un dossier »).**
   - La page 08 de l'IDE en React : l'ordre des vues, ce qui est atterri et ce
     qui ne l'est pas (vues 5, 6, 7, 3b sur des branches au 7 nov.), les
     repères DOM des témoins.
   - Le fait qu'everything declarative tient `Tree.tsx`, `main.tsx`,
     `File.tsx` et `Editor.tsx`, et qu'il faut se baser sur
     `ide-react-vue3b-2`.
   - Que `search_code` prend `scope` (préfixes séparés par `|`).
3. **À l'écriture du témoin.**
   - La forme d'appel de `search_code` par `/ide/call` : `options` est exigé
     (422 sans lui). C'est le seul rouge du lot.
   - Le gabarit d'un témoin navigateur (`hote_garde`, `manifeste_jouable`,
     `Navigateur::attendre`), la marque `navigateur` et
     `RAG3WEAVER_EXIGER_NAVIGATEUR=1`.
4. **À la passe.**
   - Les features de la lib entière dépendent de la base :
     `anthropic-llm` n'existe pas avant master. Le premier essai s'est arrêté
     net.
   - Les valeurs de luciepc : `RAG3WEAVER_PG` (port 5433), les services
     d'embarquement 7878-7880, `RAG3WEAVER_PAQUETS_REACT`.
5. **Au push (à venir).**
   - `rebase --onto master 37c42b720`.
   - Les fichiers bâtis `templates/ide/app/*` : se rebâtissent, jamais à la
     main.
   - La fenêtre de push du cœur, et le mot de c7.

Presque tout cela existe déjà, mais ailleurs : dans les fiches de la mémoire
de Claude Code (`~/.claude/projects/…/memory/`), dans les procédures
(`docs/procedures/`) ou dans les messages des sessions. Ce qu'il faut ranger
en premier dans `memory`, c'est le contenu des procédures et de la page 08 de
l'IDE, plus une fiche par session avec les fichiers qu'elle tient.

## Tour de la levée de la fenêtre du cœur : le bloc revient, avec les scores

**Bloc UserPromptSubmit, mot pour mot :**

> [mémoire]
> Fiches en rapport : « hooks-essai » (0.60 ; Session Claude Code d'essai des
> hooks de rag3weaver vers la mémoire longue (work …) ; « régime GPU » (0.43 ;
> Comment on ménage les cartes du poste.) — `get_fiche` pour lire.

**Ce qui a changé :** après trois tours muets, le bloc arrive de nouveau, et il
donne le score de chaque fiche. Ces scores ne sont pas ceux du rappel fait à
la main (★ 0,016) : le hook rend une autre échelle, normalisée ? Le bloc ne
dit pas laquelle.

**Ce qui n'a pas changé :** la base n'a que deux fiches, elles reviennent
donc toutes les deux. « Régime GPU » à 0,43 sur un message qui parle de file
de push reste du bruit, et aucun seuil ne l'a tue.

**Servi ou gêné :** rien qui serve au lot. Le score, lui, est utile : il me
permet de juger d'un coup d'œil qu'il n'y a rien à lire.
