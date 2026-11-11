# Ce qu'il faut savoir pour reprendre le périmètre de la session « mémoire » (96)

Pour un agent sans aucun souvenir de nos échanges. L'état du jour (lots, branches, blocages) est
dans `docs/memoire/11-11-2026/01-rapport-de-session.md`. Tous les chemins de code sont relatifs à
`extension/rag3weaver/` sauf mention contraire.

## Actualisation du 11 novembre

Le refus 4xx se relit depuis `2d6a08a65` sur master ; les mentions de lot non poussé ci-dessous sont historiques. Le correctif de l’archiviste est livré sous `41526b29a` : les étiquettes courtes se résolvent vers les noms exacts, avec refus d’un alias inconnu ou ambigu. Les preuves, le rejeu après rebase et le nettoyage du lot sont dans [04-alias-candidats.md](04-alias-candidats.md). Le premier document réel passe sur copie ; les cinquante documents restent en attente d’une nouvelle consigne de l’orchestration. Aucune modification de la mémoire réelle.

## 1. Le vocabulaire

- **rag3db** : la base de données (un fork de Kuzu, en C++), à la racine du dépôt. **rag3weaver** :
  le runtime d'agents en Rust qui tourne dessus, dans `extension/rag3weaver/`.
- **Le portable** : le poste de Lucie, où tourne son **hôte** (`rag3weaver-backend daemon`, port
  7970, lancé par une socket systemd, binaire `~/.local/bin/rag3weaver-backend`). Ses bases en
  service s'y trouvent. **luciepc** : l'autre poste (`ssh lucied@luciepc`), où se jouent les tests
  lourds et les pushes ; ses services d'embarquement (bge-m3, granite) sont joints depuis le
  portable par des tunnels (127.0.0.1:7979 à 7981).
- **Un backend** : une base plus son **manifeste** (`backend.json`) qui déclare ses entités, ses
  outils (des graphes Mermaid de nœuds typés) et ses modèles. **Un gabarit** : un manifeste livré
  dans `templates/backends/<nom>/` ; le gabarit `memory` est la mémoire longue.
- **La mémoire longue de Lucie** : un backend `memory` à elle — manifeste
  `~/.config/rag3weaver/memoire-longue/backends/memory/backend.json`, base
  `~/.local/share/rag3weaver/memoire-longue.rag3db` (avec `.wal`, `.extensions`, `.fts/`), fiches
  en fichiers dans `~/memoire-longue/`. C'est **une copie du gabarit du 25 octobre** : elle n'a
  ni `archive`, ni `models.llm`, ni `via` dans les réponses de `recall_fiches`. Elle se lit par
  l'outil MCP `memoire-longue` (`recall_fiches`, `get_fiche`…). **On n'y écrit qu'au mot de
  Lucie, et toujours sur une copie d'abord.**
- **Fiche** : un sujet (son identité est son nom), avec `about`, `state`, `via` (qui l'a écrite :
  `juge`, `archiviste`…). **Source** (ou **L0**) : un document tel quel, immuable, à un chemin
  `sources/…md`. **Apport** (**L1**) : une section d'une Source rangée sous une fiche.
  **Ref** : un lien typé (« doc », « cite »…). **Bilan** : une fiche écrite par le juge.
- **Le juge** : un agent Gemini qui lit le journal d'un run et rend un bilan (session et outils)
  en fiches de mémoire (`src/daemon_host/juge.rs`). **L'équipier** : un agent Gemini qui travaille
  seul, pas à pas, sur une tâche, et parle par une **boîte** (mailbox)
  (`src/daemon_host/teammate.rs`).
- **Les crochets** (hooks) de Claude Code : `rag3weaver-backend hook`, qui glisse des fiches de
  la mémoire dans une session (`src/claude_code_hook.rs`) ; posés seulement dans la session
  d'essai « hooks-essai ». Leur journal : `~/.local/state/rag3weaver/hooks.log`.
- **Les sessions** : **c7** (l'orchestration), **ED** (« everything declarative » : équipier,
  juge, atelier, gabarit vivant), **archi** (l'IDE et les crochets), **G** (« optimiseur » : pose
  les binaires et les libs sur les postes), **hotfixs**, **recherche**, **codeparsers**, **tty**,
  **hooks-essai**, **banc**. Les adresses changent ; c7 les donne.
- **Un lot, une passe, un tour, un rejeu** : un lot est ce qui part sur master ; la passe le teste
  à luciepc ; mon script `tour.sh` joue la passe (**rejeu**) puis, à part, le push ; un **rejeu
  court** ne rejoue que le nécessaire après un rebase qui apporte du code.
- **La lib** : la bibliothèque partagée de rag3db, bâtie et rangée au **magasin** :
  `~/.cache/rag3db-libs/rag3db-build-<commit>-Release-…/` (les deux postes). **Épinglée** : une passe
  nomme une lib précise par son identifiant complet (aujourd'hui 29a7bc52a), jamais `courant`.
- **`poste lourd` / `poste push` / `poste mesure`** : le verrou du poste (script
  `~/.cache/rag3weaver-build/poste` sur les deux postes) ; `lourd` pour une passe, `push` pour le
  seul dernier pas d'un push (≈ 30 s), `mesure` pour une mesure exclusive (jamais sans c7).
- **Témoin** : un test qui prouve une promesse ; **rouge d'abord** : il échoue sur le code d'avant,
  puis passe avec le correctif.

## 2. Les parties que je tiens, et où elles vivent

### 2.1 La mémoire longue (le gabarit `memory`)

- `templates/backends/memory/backend.json` : entités Fiche, Source, Apport, Memory, Subject, Ref,
  RefType ; outils (`put_*`, `get_*`, `recall*`, `select_*`, `cited_by`, `whois`, `archive`…).
- `templates/backends/memory/graphs/`, `scripts/` (Rhai), `schemas/`.
- La conception : `docs/25-octobre-2026-memoire-longue/` (01 rapport, 03 roadmaps et visions,
  04 identité des agents, 05 la migration de la fiche de Lucie, 06 M4 l'archiviste).
- Les témoins : `tests/e2e_memoire_modele.rs`, `e2e_memoire_fichiers.rs`, `e2e_cited_by.rs`,
  `e2e_reconstruction.rs`, `e2e_whois.rs`, `e2e_archiviste.rs`.

### 2.2 L'archiviste (M4, l'outil `archive`)

Un Markdown entre ; ses sections sont rangées une par une sur une fiche par une décision du
modèle.
- `templates/backends/memory/graphs/archive.mmd` : le genre `doc` assuré ; `SplitSections` ; pour
  chaque section le graphe cité `archive_decide.mmd` (candidates par `search_structured` sur
  Fiche, puis `Decide via='llm'` aux étiquettes « les candidates, ou none, ou create ») ; un
  `Route` sur `kind` vers `archive_write.mmd` (l'apport), `archive_create.mmd` (une fiche
  `proposed` et son apport), `archive_link.mmd` (lien `A_POUR_APPORT`, Ref `doc`) ; la Source
  écrite **après** les décisions. Paramètres : `titre`, `contenu`, `jour`, `chemin`, `origine`
  (« envoi », « filet » ou « migration »), `candidates` (5), `search_options` (`{"limit": 8}`),
  `allow_create` (**faux par défaut** depuis 3b119beb6), `instruction`.
- Les nœuds : `src/dataflow/decide_nodes.rs` (Decide, schéma JSON à `enum`, `max_tokens` 2048),
  `src/dataflow/split_sections.rs`, `src/dataflow/rhai_node.rs`, `ForEachNode` (graphe cité par
  élément, écrit `graph='@builtin:…mmd'`).
- Le modèle d'un backend : `models.llm` du manifeste (`src/model_source.rs`). Pour Gemini :
  `{"provider":"compatible","protocol":"vertex","model":"google/gemini-3.8-flash","params":{"project":"lr-hub-472010","location":"global"}}`,
  les identifiants par `GOOGLE_APPLICATION_CREDENTIALS` qui pointe dans `.vault/` (jamais lu ni
  affiché, seulement nommé par son chemin). Un outil qui appelle un modèle déclare
  `"policy": {"call_models": true}`.

### 2.3 Le banc des fiches classées

- `tests/e2e_banc_fiches_classees.rs`, le jeu `tests/data/banc-fiches-classees/v1.json` (39 fiches,
  10 sujets), les tentatives datées `templates/experiments/<date>-fiches-classees-<n>/`
  (`tentative.json` : la forme, les étiquettes `aucun`/`creer` ; `decide_subject_llm.json` ; `NOTE.md` ;
  `resultats/`).
- C'est un **banc à services** : jamais dans l'e2e entier ; il se joue seul, avec sa voie :
  `RAG3WEAVER_BANC_VERTEX=1 RAG3WEAVER_LLM=vertex RAG3WEAVER_BANC_TENTATIVE=templates/experiments/<dossier> RAG3WEAVER_BANC_SORTIE=<…>/resultats/llm-gemini.json ./run_e2e.sh --test e2e_banc_fiches_classees`
  (à luciepc, `GOOGLE_APPLICATION_CREDENTIALS` et `GOOGLE_CLOUD_PROJECT` posés ; le script
  `~/.cache/rag3weaver-build/banc5.sh` le fait). Sans voie, `run_e2e.sh` le refuse avant le bâti.
- Écrire sa sortie **hors** de l'arbre du tour (sinon la pose du diff suivant refuse).

### 2.4 M8, la montée des gabarits (conception seulement)

- Page `docs/09-novembre-2026-memoire-longue/02-m8-la-montee-des-gabarits.md`, ticket
  `docs/tickets/2026-11-07-une-memoire-ne-sait-pas-que-ses-gabarits-sont-en-retard.md` (à la racine).
- Elle s'appuie sur le **gabarit vivant** d'ED, `src/builtin_templates.rs` : `livre.json` (le
  gabarit d'origine et son contenu à la pose) sous `~/.local/state/rag3weaver/manifestes/<empreinte
  blake3 du chemin canonique, 16 caractères>/`, `refresh` (fusion à trois sur les feuilles du
  manifeste), `state_root`, `state_dir_of`. M8 ajoute : `templates adopt` pour une copie d'avant
  `livre.json`, une base pour les fichiers copiés, la fusion au-delà de l'intouché avec refus
  nommé, le diff des outils et de leurs appelants, la migration de schéma.

### 2.5 Le classeur local (conception seulement)

Page `docs/09-novembre-2026-memoire-longue/03-le-classeur-local-dans-le-rappel.md` : Qwen3-4B par
llama-server à luciepc, dans le fils du **rappel différé** des crochets (`rappel_differe` dans
`src/claude_code_hook.rs`), à la place du juge Gemini quand il est déclaré. Le client existe :
`LlamaServerDecider` (`src/decider.rs`, `cache_prompt: false` aujourd'hui). La page 04 dit qu'il
ne vaut rien tant que la mémoire n'a pas de contenu.

### 2.6 L'équipier, côté mesure

- Ses essais : `docs/08-novembre-2026-orchestration/01-essais-de-l-equipier.md` (c7) et ma page
  `docs/09-novembre-2026-memoire-longue/01-l-equipier-sur-les-readme.md` (avec la référence ligne
  par ligne pour la prochaine tentative).
- Une tâche se pose par `POST /ide/teammate/mailbox {backend, task, kind:"request", text}` à
  l'hôte ; la boîte se lit par `GET /ide/teammate?backend=&task=` ; l'authentification est le
  cookie `rag3weaver_ide=<token>`, le jeton lu dans `~/.local/share/rag3weaver/daemon.json` (jamais
  affiché). Un client en Python : `…/scratchpad/essai-readme/boite.py`.
- Les runs : `~/.local/share/rag3weaver/runs/<backend>/agent-<instant hex>-<n>.json.zst`
  (événements `LlmCall` avec `prompt_tokens`, `retries`) ; journal de l'hôte
  `~/.local/state/rag3weaver/daemon.log`.

### 2.7 Le client du modèle

`src/openai_llm.rs` (Vertex par l'interface OpenAI, `RetryPolicy` : une minute puis le double,
dix minutes au plus, cinq reprises, trente minutes en tout ; `retry_budget` de l'appelant ; la
garde des refus 4xx dans le lot non poussé), `src/llm.rs` (`GenOptions`, les budgets
`BUDGET_DES_RECULS_DU_CHAT` 300 s, `_DE_L_EQUIPIER` 120 s, `_DU_JUGE` 600 s), `src/chat.rs`
(statut « recul n° k du modèle »), témoins `tests/openai_llm_retry.rs` (faux serveur
`tests/common/fake_sse.rs`, horloge virtuelle `FakeClock`).

## 3. Les scripts, et comment on les lance

Sur luciepc, dans `~/.cache/rag3weaver-build/` :
- `tour.sh <lot> rejeu|push` : `rejeu` pose `<lot>-tout.diff` sur le master du moment dans
  `~/git_workspaces/rag3db-p1r-96`, joue `scripts/check_features.sh`, la lib entière (jeu large),
  l'étape navigateur et les suites de `<lot>-suites.txt`, et note `<lot>-base-jouee.txt` (master et
  lib) ; `push` vérifie les planchers, rebase et rejoue court si le crate a bougé, puis
  `pousser.sh` sous `poste push` (fetch, refus si master a bougé, commit avec `<lot>-commit.txt`,
  push, les trois contrôles). On le lance par
  `systemd-run --user --unit=<lot>-<mode>-96 --collect bash ~/.cache/rag3weaver-build/tour.sh <lot> <mode>`,
  **sans** `poste push` autour ; son journal est `<lot>-<mode>.log`. Il refuse de pousser sans un
  rejeu vert sous la même lib. `instables.txt` liste les témoins rejoués seuls s'ils sont les
  seuls rouges.
- `essai.sh <lot> <suite>`, `un-test.sh <filtre> <journal>`, `pose-et-teste.sh <lot> <filtre> <journal>`,
  `nav-seul.sh` : un essai d'une suite ou d'un test seul.
- `recul-temoin.sh`, `refus-temoin.sh` : posent un diff et jouent `tests/openai_llm_retry.rs`.
- `bin-blanc.sh` : bâtit `rag3weaver-backend` dans l'arbre du tour.

Sur le portable : `~/.cache/rag3weaver-build/mesurer_fil_par_pas.py runs/<backend> <tâche>` (la
table des jetons d'entrée par pas d'un équipier).

## 4. Les procédures

- **La procédure du push** : `docs/procedures/push-d-un-lot.md` (à la racine). L'essentiel : la
  lib épinglée par son identifiant complet ; `check_features` ; la lib entière sous le jeu large
  `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm` ; l'étape
  navigateur (`RAG3WEAVER_PAQUETS_REACT`) ; les e2e du changement ; `check_tickets.py --lot` ; un
  témoin qui se saute est rouge ; seul le dernier pas sous `poste push` ; le push dans un
  `if grep -qx VERT` ; **les trois contrôles** (HEAD = origin/master, status vide, aucun rebase en
  cours) avant de dire « poussé ».
- **Le protocole avec c7** : chaque push de code attend son « à toi » ; un rouge se dit dans la
  minute, même sans analyse ; un lot de docs seules se pousse sans attendre (rebase, push, trois
  contrôles, hash à c7) — mais jamais pendant mon propre tour de push.
- **Les commits** : identité `luciedefraiteur@gmail.com` (jamais l'adresse pro), **aucune ligne
  d'attribution** à un assistant (vérifier `grep -ciE 'co-authored|anthropic.com'` = 0), commit par
  chemins (`git commit -- <chemins>`), jamais de push en force (après un rebase : une branche au
  nom neuf `-2`, `-3`).
- **Les tickets** : un défaut connu est un fichier de `docs/tickets/` avec un en-tête à champs
  (État, Famille, Gravité, Atteignable en service, Touche rag3weaver) ; `python3
  extension/rag3weaver/scripts/check_tickets.py` les contrôle ; un ticket corrigé passe dans
  `closed/` dans le lot qui le corrige.
- **Une ligne de nouveautés** (`docs/nouveautes.md`) pour toute capacité visible, et les deux
  README la suivent (ils sont **en anglais**).

## 5. Les pièges appris (et la leçon)

- **Un veilleur `until systemctl is-active <u> | grep -qv active`** ne sort jamais : « inactive »
  contient « active ». Comparer la chaîne exacte.
- **L'outil Bash est zsh** : une variable de plusieurs chemins ne se découpe pas (`${=F}`) ; le
  shell distant de luciepc est **fish** : passer les scripts par `ssh … bash -s <<'EOF'`.
- **Arrêter une unité** ne tue pas ce qu'elle a lancé sous `poste lourd` : lister les processus
  dont `/proc/<pid>/cwd` est sous mon arbre et les arrêter par pid, jamais par nom.
- **Un appel refusé a pu s'exécuter** ; un `reset --hard` refusé par la permission : une avance
  rapide (`merge --ff-only`) fait la même chose sans rien pouvoir perdre.
- **Copier une base pour mesure** : base, `.wal`, `.extensions` et `.fts` au même instant, sans
  écrivain (hôte fermé ou instantané de G, dans `~/.local/share/rag3weaver/instantanes/`) ;
  sinon le plein texte sort d'une autre génération.
- **Une sortie de banc dans l'arbre du tour** fait refuser la pose du diff suivant (fichier non
  suivi).
- **Un banc qui exige un service** ne va jamais dans l'e2e entier.
- **Les témoins écrivaient dans le dossier d'état réel de Lucie** : désormais une racine
  temporaire sous `cfg(test)`, et `.init_array` dans les binaires de test qui n'ont pas
  `cfg(test)` (`tests/e2e/generer.py`, `tests/openai_llm_retry.rs`).
- **Le dossier de travail `/tmp` est de la mémoire vive** (tmpfs de 61 Gio) : rien de gros, et il
  disparaît au redémarrage.
- **Les noms de fiches du juge** (des bilans de 2 000 caractères) cassent les décisions de
  l'archiviste chez Vertex (400) ; les écarter des candidates (`via = juge`) les fait passer.
- **Un binaire installé peut être plus vieux que master** : vérifier par
  `git merge-base --is-ancestor <commit> <commit du binaire>` (le commit est dans le journal des
  crochets).

## 6. Les mesures faites

- **Banc des fiches classées** (Gemini 3.8 Flash, 39 fiches) : tentative 4 rejouée 35/39 justes,
  0 faux ancrage ; tentative 5 (avec `create`) : création sur les 8 fiches sans sujet, sur les 2
  hors rappel, et 2 créations fautives ; ancrages inchangés (27 justes, 0 faux). D'où
  `allow_create` éteint par défaut. Note : `templates/experiments/2026-11-09-fiches-classees-5/NOTE.md`.
- **L'équipier sur les README** : 1 pas, 12 appels, 20 min 50 s, dont l'appel 8 à 1 004 s en
  quatre reprises muettes ; 0 proposition ; fin « fait sans plan ».
- **Le fil par pas** (essai 3 de c7) : jetons d'entrée du premier appel de 5 275 (pas 1) à
  122 242 (pas 10) — la mesure d'ED ; mon script la redonne à la ligne près.
- **La précision du rappel des crochets** (171 rappels, 104 jugés par Gemini) : 92 % des rappels
  ne glissent rien ; 1 fiche glissée pertinente sur 12 ; hors la fiche propre, 4 candidates
  pertinentes en 104 rappels ; durée p50 161 ms, p95 397 ms ; mon contrôle du juge : 20 accords
  sur 20.
- **Le dossier d'état** : 477 entrées de témoins, une seule réelle.
- **L'archivage à blanc** : le gabarit d'aujourd'hui s'ouvre sur la base de Lucie ; 400 de Vertex
  sur les étiquettes des fiches du juge ; 50 s par décision avec le binaire installé.

## 7. Mes tickets ouverts

La liste, une ligne chacun, est au § 5 du rapport de session. Ils restent dans `docs/tickets/`
jusqu'à ce que c7 dise « déplace ».
