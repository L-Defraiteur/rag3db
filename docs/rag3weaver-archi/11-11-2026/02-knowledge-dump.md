# Ce qu'il faut savoir pour reprendre le périmètre « rag3weaver archi »

Cette page est écrite pour un agent sans mémoire des échanges. Le rapport de session est à côté
(`01-rapport-de-session.md`) : il dit l'état du jour, cette page dit comment les choses marchent.

## Reprise du 11 novembre : chemins et projection du chat

La référence est `cx-wt-flux` / `cx-archi-ide-par-le-flux`, validation sur luciepc,
target `flux`. Les chemins et commandes de l'ancienne session ci-dessous sont historiques ;
le rapport adjacent donne les lanceurs et dossiers de résultats courants. Sur luciepc,
les services d'embarquement sont `127.0.0.1:7878,127.0.0.1:7879,127.0.0.1:7880` ;
`7979,7980,7981` sont les ports du portable, pas des services locaux distants.

La fin du tour scelle `usage_du_chat` dans les ajouts du journal. La projection commune
lit d'abord les résumés en attente sous `ecritures` puis `etat`, libère les verrous, puis
liste et lit les fichiers ; elle déduplique par identité de run. Le total scellé remplace
les traces `Consumed`, qui restent la lecture de compatibilité des anciens journaux.
Les bornes début/fin suffisent aux rôles : ne pas cloner toutes les traces d'un run pour
faire une barre latérale. Les résultats absents ne se mettent pas en cache négatif.

## Le décor

- **rag3db** est le moteur de base de données de Lucie : un fork de Kuzu, en C++. Le dépôt est
  `~/git_workspaces/rag3db` sur le poste « portable », et son remote est
  `github.com:L-Defraiteur/rag3db`.
- **rag3weaver** est la couche Rust au-dessus, sous `extension/rag3weaver/`. Elle comprend :
  - des **backends**, chacun une base plus un manifeste JSON qui déclare ses outils ;
  - des **outils**, des graphes de dataflow écrits en `.mmd` (Mermaid), avec des nœuds Rust ;
  - un **hôte** (*daemon host*), un service HTTP qui tient les backends ouverts. Chez Lucie, il
    est servi par une socket systemd sur `127.0.0.1:7970`, et son adresse et son jeton sont dans
    `~/.local/share/rag3weaver/daemon.json` ;
  - un **IDE web**, servi par l'hôte sous `/ide/`.
- **codeparsers** est un sous-module (`extension/rag3weaver/codeparsers`). Dans un worktree
  neuf, il faut faire `git submodule update --init extension/rag3weaver/codeparsers`, sinon
  cargo échoue (« failed to get codeparsers »).
- **lucivy** est le moteur de plein texte, publié sur crates.io.

## L'IDE React

- **Les sources** sont sous `extension/rag3weaver/ui/ide/` (React, shadcn/ui, icônes lucide,
  Tailwind). Le **bundle** bâti est commité sous `extension/rag3weaver/templates/ide/app/`
  (`app.js`, `app.css`, `bundle.json`) et embarqué dans le binaire.
  - Pour le rebâtir : `bash extension/rag3weaver/scripts/ide_bundle.sh`, qui fait `npm ci` puis
    `node build.mjs`.
  - Pour vérifier les types : `cd ui/ide && npx tsc --noEmit -p .`.
  - Toute modification de `ui/ide` se commite **avec** le bundle rebâti.
- **Les vues** sont sous `ui/ide/src/views/` : `Home`, `Chat` (et `ChatSidebar`, la barre des
  conversations), `File`, `Editor`, `Tree`, `Search`, `Recherches`, `Symbol`, `Graph`, `Dag`,
  `Propositions`, `Reactions`, `Bilans`, `Tasks`, `TaskReview`. La route est
  `/ide/code/<id du backend>/<vue>` (`route()` dans `ui/ide/lib/host.ts`).
- **Les cartes sous l'appel.** Un résultat d'outil peut porter, hors du texte que lit le modèle,
  une liste `ui` de composants `{kind, sujets, …}`. La page les rend sous l'appel par le registre
  `REGISTRE` (« kind → composant ») de `ui/ide/src/components/UiDuTour.tsx` :
  - `index_progress` : l'avancement de l'index d'un dépôt ;
  - `accord` : la carte d'accord.
  
  Côté Rust, le tour garde ce `ui` dans `Turn.ui` (`src/llm.rs`), que la session relue rend
  aussi.
- **Le flux.** C'est un seul `EventSource` pour toute la page : `GET /ide/flux?sujets=…`, ouvert
  par `openFlux` dans `ui/ide/lib/host.ts`. Les vues s'abonnent par `onFlux`. Chaque message est
  `{sujet, backend, evenement: {kind, …}}`. Les sujets sont :
  - `files` (`FileChanged`), `dataflow`, `catalog`, `reactions`, `declarations` (propositions),
    `agent` (`RunStarted`, `LlmCall`, `ToolCallStarted`/`Finished`, `RunFinished`), `tokens`,
    `repos` (`RepoAttached`, `RepoIndexed`), `accords` (`AccordDecide`) ;
  - et, sur la branche `ide-par-le-flux`, `etat` (`{kind: "Etat", de, cle, etat}`). On s'y abonne
    par `onEtat(de, cle, f)`, et l'hôte publie par `Flux::publier_etat` (`src/ide.rs`).
- **La jauge du contexte** (le composant `Gauge`, dans `ui/ide/src/views/Chat.tsx`).
  - Elle fait `used` sur `max` : `used` est le nombre de jetons d'entrée du **dernier appel** au
    modèle, que le fournisseur rend (`LastCall`, dans `src/agent.rs`) ; `max` est la fenêtre du
    modèle déclaré.
  - Elle suit chaque `LlmCall` du flux pendant le tour.
  - `last_call` est gardé dans le fichier de session (`ChatFile`, `src/daemon_host/chat.rs`).
  - Chez Lucie, le modèle est `google/gemini-3.8-flash` : un million de jetons, donc une
    conversation ordinaire pèse moins de 1 %.

## L'atelier

L'**atelier** est le backend de chat généraliste de Lucie : il ouvre des dépôts, lit, écrit et
lance des commandes.
- **Son gabarit** : `extension/rag3weaver/templates/backends/atelier/backend.json`.
- **Chez Lucie** : le manifeste posé est `~/.config/rag3weaver/atelier/templates/backends/atelier/backend.json`,
  et sa racine est `/home/lucied`.
- **La pose et la racine.** `rag3weaver-backend atelier` pose un atelier neuf. Sa racine est le
  dossier d'où on le lance, ou `~/rw-code` si on le lance depuis `$HOME` ou `/`
  (`code_tree::racine_de_l_atelier`). Un atelier déjà posé garde sa racine.
- **Le suivi des gabarits.** Un manifeste posé d'après un gabarit livré est noté hors de l'arbre,
  sous `~/.local/state/rag3weaver/manifestes/<blake3 du chemin canonique>[..16]/livre.json`.
  À chaque ouverture, l'hôte le remet au gabarit du binaire (`builtin_templates::refresh`), s'il
  n'a pas été modifié à la main. Il n'y a donc rien à reposer à la main après une mise à jour.
- **Les outils :**
  - `attach_worktree`, `list_worktrees`, `open_branch` : attacher un dépôt, ou ouvrir une branche
    dans un worktree frère ;
  - `run_command` : une commande, par la porte des commandes (la `Garde`, `src/commande.rs`) ;
  - `show_progress` ;
  - `read_file` (200 lignes au plus) et `read_scope` (une fonction par son nom, par l'index) ;
  - `replace_text` (un texte unique), `replace_lines` et `insert_lines`. Ces deux derniers
    exigent d'avoir lu ces lignes au tour même ou au précédent : c'est l'**empreinte de lecture**,
    `src/daemon_host/empreintes.rs` ;
  - `ask_permission`.
  
  Les outils de fichier passent par l'hôte (`src/daemon_host/atelier_fichiers.rs`). Un chemin
  dans un worktree attaché va au backend de code de ce worktree (son index, sa réindexation) ;
  sinon, il va à la racine de l'atelier ; hors de la racine, c'est refusé, sauf accord.
- **La marque « né de l'atelier ».** Un worktree ouvert par `open_branch` reçoit le fichier
  `<config>/<nom>/ouvert-par-l-atelier` (`code_tree::MARQUE_DE_L_ATELIER`), écrit après
  l'attachement. L'atelier écrit ce worktree sans demander ; tout autre arbre inscrit passe par
  `ask_permission`.
  - Ce n'est pas un champ du manifeste : écrire le manifeste recharge le backend et coupe
    l'index.
  - `task_writes` (dans le workspace d'un dépôt) garde son sens : l'agent d'une tâche y écrit,
    l'atelier non.
- **La carte d'accord** (`kind: "accord"`). Elle sert à deux choses :
  - un chemin hors de la racine (`ask_permission`, mode `read` ou `write`) ;
  - une commande que la porte met en attente (`genre: "commande"`).
  
  La décision passe par `POST /ide/accords {id, decision: accorder|refuser}` ; l'état se relit par
  `GET /ide/accords?id=`.
  - Un oui ouvre le chemin, ou accorde à la `Garde` la **commande exacte**, jamais sa famille
    (le ticket fermé `docs/tickets/closed/2026-11-09-un-oui-pour-une-commande-accordait-toute-sa-famille.md`).
  - La décision **relance le tour d'elle-même** : un message « (carte d'accord) … » entre dans la
    conversation, et un tour repart dans un fil.
  - La demande porte le backend de sa conversation, que le chat rattache (`rattacher_accord`).
- **Le journal des décisions** : `<racine des runs>/decisions.jsonl`, une ligne par décision
  (id, genre, chemin ou commande, mode, raison, décision, qui, quand). Il est à part du journal
  des runs.
- **Dans une tâche** (un équipier), les commandes en attente passent par la boîte du superviseur,
  pas par la carte.

## Les hooks de Claude Code

- **Le verbe** est `rag3weaver-backend hook`, dans `src/claude_code_hook.rs`, avec la lecture des
  transcripts dans `src/claude_code_transcript.rs`.
- **L'installation, par session :** `extension/rag3weaver/scripts/hooks_claude_code.sh`, avec son
  témoin `scripts/test_hooks_claude_code.sh`.
  - **Jamais `--global` sans le mot de Lucie.**
  - Le réglage est dans `~/.config/rag3weaver/hooks.json`, le journal dans
    `~/.local/state/rag3weaver/hooks.log`.
- **Les pages** sont sous `extension/rag3weaver/docs/07-novembre-2026-hooks-claude-code/`.
- **Ce qu'ils font :**
  - au début d'une session, ils disent qui l'on est et ce qu'on tenait ;
  - à chaque prompt, ils glissent au plus une fiche de mémoire liée, et ce que d'autres ont
    changé dans les fichiers qu'on a touchés.
- **La règle tranchée :** le hook n'écrit que le mécanique (`via: hook`). L'apport `identity`
  reste la parole de l'agent ; il est redemandé après chaque compaction.
- **Le contexte** se lit sur `message.usage` de la dernière ligne `assistant` du transcript.
- **Le ticket ouvert** : le juge du rappel coupe sa réponse sous 400 jetons.

## Les témoins navigateur

- **Ce que c'est.** Ce sont des tests Rust qui lancent un vrai Chrome sans tête, par le protocole
  DevTools (`src/daemon_host/navigateur.rs`, type `Navigateur` : `aller`, `attendre`,
  `evaluer`).
- **La marque.** Ils portent
  `#[cfg_attr(not(feature = "navigateur"), ignore = "témoin navigateur : …")]`. Ils sont donc
  ignorés par `cargo test --lib` et joués par `scripts/temoins_navigateur.sh`, qui les trouve à
  cette marque.
- **Les variables :**
  - `RAG3WEAVER_NAVIGATEUR` : un binaire précis ;
  - `RAG3WEAVER_EXIGER_NAVIGATEUR=1` : l'absence de navigateur est un rouge, jamais un saut ;
  - `RAG3WEAVER_PAQUETS_REACT=<dossier>` : les paquets npm de React pour l'aperçu à chaud,
    préparés par `scripts/paquets_react.sh <dossier>`. Le dossier est
    `~/.cache/rag3weaver-build/paquets-react`.
- **L'hôte de test.** Un témoin monte son hôte par
  `crate::daemon_host::tests::hote_garde_avec("j", |h| h.with_…)`, entre par
  `/ide/entrer?jeton=j`, et parle à l'hôte par `crate::daemon::agent_de_temoin()`, qui a un délai.
- **Les modèles de test :** `CallbackLlm` et `MockLlm` (`src/llm.rs`). Attention, `MockLlm::new`
  a une fenêtre de 4 096 jetons ; pour un plus grand plafond, écrire
  `MockLlm { context_len: N, ..MockLlm::new("") }`.
- **Le rouge sur l'ancien bundle** : `git stash push -- templates/ide/app`, jouer, puis
  `git stash pop`.

## Les scripts et les procédures

Les scripts de lot sont sous `~/.cache/rag3weaver-build/`, sur le portable.
- **`poste`** sert de verrou du poste :
  - `poste lourd <commande>` pour tout bâti ou toute passe ;
  - `poste push archi -- <commande>` pour le dernier pas d'un push ;
  - `poste mesure` pour une mesure.
  
  Au plus deux passes lourdes à la fois sur le portable. Ne jamais lancer `-j$(nproc)` : laisser
  deux cœurs libres.
- **Une passe** : `wt-<lot>-passe.sh`, sur le modèle de `wt-jauge-passe.sh`. Ses étapes (`ETAPE`) :
  - `module` : les tests filtrés par `FILTRE=<motif>` ;
  - `navseul` : un seul témoin navigateur, `FILTRE=<nom>` ;
  - `navigateur` : tous les témoins navigateur ;
  - `complete` : `check_features`, le témoin du script d'installation des hooks, la lib entière
    sous le jeu de features large, `e2e_hooks_claude_code`, puis tous les témoins navigateur.
  
  Le verdict est écrit dans `~/.cache/rag3weaver-build/<lot>-passe/verdict` (`VERT` ou
  `ROUGE <étape>`). « Vert » veut dire **aucun rouge** : le script cherche les `failed`, les
  `SAUTÉ` et les `error`.
- **La lib du moteur épinglée.** Une passe exporte `RAG3DB_LIB=<id du magasin de libs>`
  (aujourd'hui `rag3db-build-29a7bc52aaf11e6204f9beafaffb154105417e13-Release-…`), puis
  `exporter_la_lib`, de `scripts/lib_du_magasin.sh`. `RAG3DB_LIB` seul ne lie rien.
- **Un target cargo par lot** : `CARGO_TARGET_DIR=~/.cache/rag3weaver-build/<lot>`. Un target
  bâti par un autre worktree est refusé. Pour le reprendre : `RAG3WEAVER_TARGET_REPRENDRE=1`.
- **Les services d'embarquement**, par `RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,127.0.0.1:7980,127.0.0.1:7981`.
  7980 est le service bge-m3 de luciepc, par tunnel ; d'autres sessions le prennent parfois.
- **Le procédé d'un lot** (`docs/procedures/push-d-un-lot.md`) :
  1. un worktree propre sous `~/.cache/rag3weaver-build/wt-<lot>`, avec sa propre branche ;
  2. le témoin d'abord, **vu rouge** pour la bonne raison ;
  3. le commit, poussé sur sa branche dans la minute : rien ne reste en local ;
  4. le correctif, puis la passe complète sous `poste lourd` ;
  5. le feu de c7 (« à toi ») ;
  6. le push par `wt-<lot>-push.sh` (sur le modèle de `wt-jauge-push.sh`), sous `poste push`.
- **Ce que fait le script de push :**
  - il rebase sur le master du moment ;
  - si du code non `.md` est arrivé sous `extension/rag3weaver`, il s'arrête en exit 3, et il faut
    rejouer court hors verrou ;
  - sinon, il pousse `HEAD:master` ;
  - puis il fait les **trois contrôles** : status propre, aucun rebase en cours,
    `origin/master` = `HEAD` ;
  - enfin, il efface le worktree et le target du lot.
  
  Ensuite, le hash part à c7, G et ED.
- **Les règles de fond :**
  - jamais de push forcé, jamais de branche distante supprimée ;
  - une branche rebasée se pousse sous un nom neuf (suffixe `-2`) ;
  - pas de mention de Claude dans les commits, et l'identité git est l'adresse gmail de Lucie ;
  - un rebase sans rejeu ne vaut que si **seules** des docs ou du C++ du moteur sont arrivés ;
    du Rust ou un gabarit sous `extension/rag3weaver` imposent le rejeu court ;
  - l'arbre principal de Lucie (`~/git_workspaces/rag3db`) est intouchable : on n'y commite
    rien et on n'y change pas de branche.
- **Les tickets** vivent dans `docs/tickets/`, et ceux qui sont fermés dans `docs/tickets/closed/`.
  `python3 extension/rag3weaver/scripts/check_tickets.py --lot origin/master` liste les tickets
  d'un lot. Un défaut connu et non corrigé devient un ticket, pas une ligne de message.

## Les pièges appris

- **Un client HTTP de test sans délai fige la lib entière** sous charge (25 min, le 9 novembre).
  Les témoins de l'hôte passent par `agent_de_temoin()` (connexion, envoi et réponse bornés à
  120 s, corps libre pour les flux) ou par `demander` (10 s).
- **Un témoin qui finit pendant un index** laisse le moteur couper sous lui : SIGSEGV à la sortie.
  Il faut attendre la fin de l'index (`attendre_l_index` dans `atelier_fichiers.rs`).
- **`WorkspaceConfig` refuse les champs inconnus** (`deny_unknown_fields`) : un champ de gabarit
  neuf s'ajoute aussi à la struct.
- **Une boîte `'a` dans `OutilsDuChat`** lui donne un destructeur ; les temporaires passés à
  `OutilsDuChat::de` doivent alors vivre dans des variables (`teammate_tools.rs`).
- **Un rouge sous charge se départage** en jouant le témoin seul plusieurs fois, et en comparant
  la durée de la lib avec master (environ 33 s pour la lib entière sur le portable). Une lib trois
  fois plus lente est un défaut du lot, pas du bruit.
- **Sous zsh, l'outil Bash** n'a pas de `/dev/tcp`, et une variable non citée ne se découpe pas.
  `pgrep -f <mot>` se trouve lui-même : ne jamais tuer sur sa sortie.
- **Le journal de l'hôte** (`~/.local/state/rag3weaver/daemon.log`) ne trace pas les requêtes
  HTTP.
- **Les tests écrivent leurs `livre.json`** dans le vrai `~/.local/state/rag3weaver/manifestes`.
  96 a le ticket et son correctif.

## Les tickets ouverts

- `docs/rag3weaver-archi/tickets/2026-11-09-cinq-clients-de-production-attendent-sans-delai.md` : il attend la
  décision de Lucie sur les délais.
- `docs/rag3weaver-archi/tickets/2026-11-08-le-juge-du-rappel-coupe-sa-reponse-sous-400-jetons.md` : il faut
  relever le plafond du juge, ou couper la réflexion de Gemini.
- `docs/rag3weaver-archi/tickets/2026-11-07-search-structured-le-score-d-une-ligne-n-est-pas-comparable-d-un-signal-a-l-autre.md` :
  reste le seuil mesuré de `recall_min_score`.
- `docs/rag3weaver-archi/tickets/2026-11-07-un-test-de-la-lib-qui-parle-au-service-d-embarquement-attend-sans-delai.md` :
  relevé par archi, sans porteur.
