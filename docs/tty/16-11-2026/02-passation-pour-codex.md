# Passation tty pour Codex — 16 novembre 2026

Écrite pour une session Codex neuve, qui n'a que les docs et les tickets. Le périmètre est **le terminal de l'agent**, le crate `extension/rag3weaver/tty` (binaire `rag3weaver-tty`). Le détail technique (fichiers, règles de la vue, pièges, commandes) est dans `docs/tty/16-11-2026/03-knowledge-dump.md` : lis-le avant d'écrire une ligne.

## 1. État

**État final (16 nov. 2026, 10 h 5x, arrêt demandé par Lucie)** : rien en branche, aucun pas en cours, aucune passe ni aucun processus de tty en vol ; le binaire posé est `db755e5d` (master, code de tty-20). Codex repart de cette page.

### Sur master (tête 9f0cbe7ce4 pour le dernier lot tty)

Le terminal fait aujourd'hui, dans l'ordre des lots du 15 et du 16 novembre :

| Lot | Ce qu'il apporte | Page |
|---|---|---|
| tty-14 | `/model` : les modèles de la tâche, en choisir un | `docs/tty/15-11-2026/05-model.md` |
| tty-15 | Maj-Entrée sous un tmux réglé en CSI u | `docs/tty/15-11-2026/06-maj-entree-sous-tmux.md` |
| tty-16 | l'historique ne garde que demande et réponse ; accords vivants à boutons `[ Oui ] Non` ; o/n sur une proposition de déclarations ; jamais un mot d'état nu (animation de travail, animation de repos par thème, cause nommée à côté) ; la réponse affichée une fois | `docs/tty/15-11-2026/07` à `09` |
| tty-17 | le témoin `/model` de l'hôte (`src/daemon_host/modele_de_tache.rs`) exige `openai-llm` | pas de page : le commit a8907ac57d le dit |
| tty-18 | `/nom <x>` nomme l'agent de la tâche (`POST /ide/teammate/name`, route d'ED) ; les lignes de l'agent portent son nom | `docs/tty/15-11-2026/10-nom.md` |
| tty-19 | une demande s'écrit une fois ; `/nom` seul dit le nom gardé | même page, « L'essai réel » |
| tty-20 | `--plain --demande` reste attaché jusqu'à la fin de sa demande, journal avec les outils (défaut T1) | `docs/tty/16-11-2026/01-plain-reste-attache.md` |

Le binaire posé pour Lucie est `~/.cache/rag3weaver-build/rag3weaver-tty`, md5 `db755e5d` (bâti depuis master 9de69102c, le code de tty-20), et le même en `~/.cache/rag3weaver-build/rag3weaver-tty-essai`.

### En branche

Rien. Toutes les branches `cx-tty-*` sont sur master ou abandonnées ; aucun travail en vol.

### Ce qu'il faut savoir pour ne rien casser

- **Le binaire de Lucie** : on le remplace par `cp` vers un `.neuf` puis `mv` (son terminal ouvert garde l'ancien) ; jamais en écrivant dessus. Seulement après qu'un lot est sur master, bâti depuis master.
- **Ses tâches** : `atelier` et `essai-tty` sur le backend `12389e156742` (le dépôt rag3db) sont à elle ; `c7-essai` est à l'orchestration. Pour un essai réel, créer une tâche à soi (`POST /ide/taches {backend, nom}`), puis la rejeter (`POST /ide/taches/rejeter {backend, nom, why}`).
- **L'hôte** (port 7970, `~/.local/share/rag3weaver/daemon.json`) est posé par la session G (« optimiseur ») ; on ne le relance pas soi-même. Les journaux de runs et les boîtes des tâches (`~/.local/share/rag3weaver/taches/…`) ne s'effacent pas.
- **Les thèmes** (`extension/rag3weaver/templates/themes/*.json`) : couleurs dérivées par `scripts/themes_palettes.py` (jamais à la main) ; les animations s'y éditent.

## 2. Travail pour Codex, dans l'ordre

Chaque entrée renvoie à son ticket, qui dit le détail. Pour chacune : rouge d'abord, passe courte, « prêt » à l'orchestration, push seulement à son « à toi » (partie 4).

### 2.1 Les commandes et options en anglais, avec alias — 3 h, libre

Ticket `docs/tty/tickets/2026-11-16-les-commandes-et-options-du-terminal-en-anglais.md`. `/help`, `/steps`, `/name`, `--repo`, `--task`, `--request`, `--log`… à côté des noms français, qui restent. Fichiers : `src/commandes.rs`, `src/main.rs`, le README du crate. Preuve : tests de lecture des deux jeux de noms, rouges d'abord ; le harnais entier reste vert (il lance avec les anciens noms).

### 2.2 La cause d'une attente dans le style du thème — 4 à 6 h, libre pour la première moitié

Ticket `docs/tty/tickets/2026-11-15-la-cause-d-une-attente-dans-le-style-du-theme.md`. Aujourd'hui, la cause (« le modèle répond… (gemini-3.8-flash, 12 s) », « run_command cargo check travaille… (40 s) ») s'écrit en sourdine, d'un seul style, après l'animation (`Vue::ligne_vivante`, `src/vue.rs`). À faire :
- dans `src/anim.rs`, une fonction qui rend la cause dans le style de l'animation en cours : pour la forme `glitch` (alien, les écritures), un ou deux caractères remplacés par un glyphe du thème à chaque image, tirés par la graine (`xorshift`), le reste lisible ; pour `cercle` et `transmutation`, la cause en couleur du halo, encadrée par le premier glyphe de l'image ;
- en mode ASCII (`Glyphes::Ascii`), la cause en clair ;
- pendant que le texte coule (`Etat::Ecrit`), rien ne change : le cœur seul (plafond d'octets, test `tui::tests::mesure_des_octets_pour_mille_tokens`, à ne pas relâcher).

Preuve : tests de `anim.rs` (une image en glitch garde au moins 80 % des caractères de la cause, deux graines différentes donnent deux images, l'ASCII reste ASCII pour les cinq thèmes), rouges d'abord ; harnais entier. La seconde moitié (quota, index, commande en fond, par un événement `waiting {cause, since, retry_in?}` de l'hôte) est bloquée par archi et ED : ne pas l'inventer.

### 2.3 Une frappe tapée avant l'invite d'une commande `!` — 3 h, libre

Ticket `docs/tty/tickets/2026-11-15-une-frappe-tapee-avant-l-invite-d-une-commande-se-voit.md`. Le risque : un mot de passe tapé avant que `sudo` coupe l'écho entre dans la sortie gardée. Prendre la deuxième piste du ticket : lire l'état d'écho du pseudo-terminal (`tcgetattr` sur le maître, `src/coquille.rs`, FFI libc déjà là) et retenir les frappes tant que l'écho est ouvert et que la commande n'a rien lu, ou au moins retirer leur écho de la sortie gardée. Preuve : le test `coquille::tests::a_password_read_with_echo_off_never_reaches_the_kept_output` sans son attente de 500 ms (un nouveau test, nom anglais) rouge d'abord, puis vert ; la séance 1g du harnais (`!` avec un faux sudo).

### 2.4 Le mode plain et la fin d'un pas relancé — 2 à 4 h, le témoin libre

Ticket `docs/tty/tickets/2026-11-16-le-mode-plain-et-la-fin-d-un-pas-relance.md`. D'abord le témoin sur le faux hôte (fin puis relance), à côté de `mesures/plain_stays_attached.py`. S'il est rouge, demander à archi s'il préfère un champ « fin de la demande » ou le délai de grâce sur `state`, puis corriger `src/main.rs`.

### 2.5 Les messages entre sessions dans le terminal — 8 à 10 h, le rendu libre, le branchement bloqué

Ticket `docs/tty/tickets/2026-11-16-les-messages-entre-sessions-dans-le-terminal.md`. La forme décidée par c7 (repliés dans l'historique, « › Message de @X : … (Ctrl-O pour tout lire) », « “…” → envoyé à X » puis l'accusé, la ligne discrète, Ctrl-O étendu aux messages) et la forme de ligne proposée à archi y sont. Le rendu, les tests et la route du faux hôte se font tout de suite ; ne rien pousser sur master avant que la page de relevé d'archi confirme la forme.

### 2.6 Ce que le terminal montre d'un agent pair — 4 à 6 h, bloqué

Ticket `docs/tty/tickets/2026-11-16-le-terminal-d-un-agent-pair-des-sessions.md`. Attend la conception du mode pair par archi.

### 2.7 La copie de l'hôte reconnue par un champ — 1 h, bloqué

Ticket `docs/tty/tickets/2026-11-16-la-copie-de-l-hote-se-reconnait-par-un-champ.md`. Attend le champ d'archi.

## 3. Ce qui attend Lucie (Codex n'y touche pas)

- **Sous tmux par défaut, Maj-Entrée envoie** (`docs/tty/tickets/2026-11-15-sous-tmux-maj-entree-envoie-au-lieu-d-aller-a-la-ligne.md`) : la seule voie est un réglage de tmux chez l'utilisateur, alors que « rien n'est à installer en plus de rag3weaver ». Accepter la limite (le repli Alt-Entrée est dit à l'écran) ou non, c'est à elle.
- **Windows et macOS** (`docs/tty/tickets/2026-11-13-le-terminal-sous-windows-et-macos-a-jouer.md`) : il faut une machine.
- **Les phrases des animations de repos** (« LE CERCLE T'ÉCOUTE… », « À TOI DE TRANSMETTRE… », « YOUR MOVE, CHOOM… », « EN VEILLE : À TOI… », « LA CLAIRIÈRE T'ÉCOUTE… ») : proposées par tty, à garder ou changer à son mot.
- **Un nom d'agent avec un blanc** (« Om bre » est accepté par `valider_le_nom_de_l_agent`) : à trancher avec ED.

## 4. Règles du périmètre

- **Langue** : tout code neuf en anglais : identifiants, `/commandes`, options et drapeaux, clés JSON et de manifeste, `kind`/`from`/`peer`/`direction`, noms de tests et de scripts. L'affichage du terminal, la prose, les commits, les tickets et les docs restent en français. L'existant français (le crate en est plein : `Vue`, `saisie`, `ligne_de_boite`…) ne se renomme pas en lot : un alias anglais quand on y touche.
- **Worktree et target à soi** : `git worktree add ~/git_workspaces/cx-<sujet> -b cx-<sujet> origin/master`, puis `git submodule update --init extension/rag3weaver/codeparsers` ; un `CARGO_TARGET_DIR` à ce worktree (`~/.cache/rag3weaver-build/target-<sujet>/<worktree>`), **jamais partagé** entre worktrees : un target partagé a rejoué le binaire de test d'un autre arbre sans rien recompiler (15 nov.). L'arbre de Lucie (`~/git_workspaces/rag3db`) ne se touche pas.
- **Rouge d'abord** : chaque correctif a un test vu rouge avant le code, et le rouge est commité et poussé en branche (`test(tty): … — rouge`). Une suite non jouée est rouge ; un test sauté n'est pas vert ; lire le compte de tests joués.
- **Passe et push** : par `docs/procedures/push-d-un-lot.md`, jamais sans le « à toi » de l'orchestration (c7, session « rag3db-5a »). La passe de référence est celle du dépôt, `tools/poste/passe_epinglee.sh` (la procédure dit comment la lancer) ; le knowledge dump donne en plus la passe courte propre au terminal (§ « La passe courte ») : crate avec et sans vue, `check_features`, et le harnais quand l'affichage change. Le push se fait par `poste push <session> -- ~/.cache/rag3weaver-build/push_garde.sh <worktree> <dossier de journaux>` dans un `if tail -1 <resume.log> | grep -qx VERT`, et se dit « poussé » avec le hash et les trois contrôles (status vide, aucun rebase en cours, FETCH_HEAD = HEAD).
- **Identité** : `git config user.email` = l'adresse gmail de Lucie, nom « Lucie Defraiteur » ; jamais l'adresse de travail ; aucune mention d'assistant dans les commits. Jamais de force push ni de suppression de branche distante ; une branche rebasée se pousse sous un nouveau nom (`cx-tty-20-2`).
- **Postes** : bâtis lourds sous `~/.cache/rag3weaver-build/poste lourd` avec `POSTE_MEM_MAX=32G`, jamais `-j$(nproc)` (`-j 8`). Un chemin absolu de journaux est refusé par la garde : passer un nom de dossier sous `~/.cache/rag3weaver-build/`.
- **Un défaut connu non corrigé** devient un ticket sous `docs/tty/tickets/`, contrôlé par `python3 extension/rag3weaver/scripts/check_tickets.py`.
