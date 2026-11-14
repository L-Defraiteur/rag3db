# Ce qu'il faut savoir pour reprendre l'interface terminal de rag3weaver

Écrit le 11 novembre 2026 par la session « tty », pour un agent qui n'a aucune mémoire de ce qui précède. Les chemins sont relatifs à la racine du dépôt `rag3db` (`~/git_workspaces/rag3db` sur le portable de Lucie, GitHub `L-Defraiteur/rag3db`).

## 1. Les mots

| Mot | Ce qu'il veut dire |
|---|---|
| **rag3db** | le moteur de base de données (C++, sous `src/`), un graphe avec vecteurs et plein texte |
| **rag3weaver** | le produit au-dessus (Rust, sous `extension/rag3weaver/`) : indexation de code, recherche, agents |
| **l'hôte** | le serveur HTTP local de rag3weaver (`127.0.0.1:7970`, lancé par une socket systemd). Il ouvre les bases et fait tourner les agents. Son adresse et son jeton sont dans `~/.local/share/rag3weaver/daemon.json` (`{address, token, …}`). Chaque requête porte `Authorization: Bearer <token>`. |
| **un dépôt attaché** | un dépôt git que l'hôte indexe et sert. Dans le JSON et les manifestes, on l'appelle « backend », mais ce mot ne s'affiche jamais à l'écran : c'est une décision de Lucie. |
| **l'équipier** | l'agent de code autonome de l'hôte. Il travaille par « pas » sur une **tâche**, qui vit dans un worktree git à elle (une copie de travail sur sa propre branche). |
| **la boîte** | la file de messages d'une tâche, entre le superviseur et l'équipier (`mailbox.jsonl`) |
| **le flux** | les événements de l'hôte poussés en SSE (Server-Sent Events, une réponse HTTP qui ne se ferme pas) sur `GET /ide/flux` |
| **un lot** | des commits poussés d'un coup sur `master` après leurs tests |
| **une passe** | le jeu de tests d'un lot avant son push |
| **un e2e** | un test de bout en bout, sous `extension/rag3weaver/tests/e2e_*.rs` ; tous sont montés dans un seul binaire de test |
| **c7** | la session d'orchestration, qui ordonne les pushes (« à toi ») |

## 2. Où vit le terminal

- **Le crate** : `extension/rag3weaver/tty/`, un paquet Rust à part, nommé `rag3weaver-tty`. Il ne dépend pas du moteur : la lib rag3weaver ne compile pas sans lui, d'où ce crate à part.
- **Ses dépendances** : `ureq` (HTTP sans TLS), `serde`, `serde_json`. Avec la feature `vue`, active par défaut, s'ajoutent `ratatui` 0.30 (avec son backend `crossterm`, qui marche sous Windows, macOS et Linux) et `unicode-width`.
- **Ses modules**, tous dans `tty/src/` :

| Module | Ce qu'il fait |
|---|---|
| `endpoint.rs` | l'adresse et le jeton de l'hôte : `daemon.json` local, une commande qui l'imprime (`ssh luciepc cat …`), ou une adresse de tunnel |
| `client.rs` | les appels JSON à l'hôte, le choix du dépôt (`resoudre_depot`, d'après `GET /ide/etat`), l'en-tête, et la coloration (`surligner` → `POST /highlight`) |
| `sse.rs` | le décodeur SSE |
| `flux.rs` | la lecture de `/ide/flux` par une connexion TCP à nous, et la reprise par `Last-Event-ID` |
| `plain.rs` | le rendu en texte brut |
| `vue.rs` | la vue : le modèle pur (états, lignes validées, blocs de code tenus jusqu'à leur clôture) |
| `tui.rs` | la boucle du terminal (au plus 30 images/s, un canal pour le flux, la boîte et le clavier) |
| `anim.rs` | les trois animations, en fonctions pures |
| `theme.rs` | le thème, lu de `templates/themes/daemon.json` par `include_str!` |
| `code.rs` | le rendu des lignes et des blocs de code colorés, repliés sans perte |
| `main.rs` | le binaire |

- **Le thème** : `extension/rag3weaver/templates/themes/daemon.json`. Les couleurs y sont en hex, les animations en données, et il porte la palette `code`. **L'IDE lit le même fichier** (`GET /ide/theme/daemon.json`, `ui/ide/src/lib/theme.ts`, lot d'archi `7dac6418b`). Règle convenue avec archi : on y ajoute des champs, on n'en renomme ni n'en retire aucun sans le prévenir.
- **Le témoin côté rag3weaver** : `extension/rag3weaver/tests/e2e_tty.rs`, trois tests :
  - le flux coupé au milieu d'un run de l'équipier se reprend sans perte ni doublon ;
  - `describe` porte la `policy` de chaque outil ;
  - un bloc se colore par les spans de l'hôte.

  rag3weaver prend le crate en dépendance de test sans la vue (`default-features = false`).

## 3. Comment le lancer

```bash
cd extension/rag3weaver/tty
CARGO_TARGET_DIR=~/.cache/rag3weaver-build/target-tty cargo run --release -- \
  --depot <nom du dépôt attaché ou chemin de son manifeste> --tache <nom> \
  [--demande "<texte>"] [--plain] [--daemon-json <chemin> | --daemon-cmd "<commande>"] [--adresse hôte:port] [--depuis <id>]
```

- **Les deux modes.**
  - Sans `--plain`, c'est la vue inline : l'historique coule dans le terminal, trois lignes vivantes se redessinent en bas (la ligne en cours, l'état animé avec l'en-tête « dépôt · branche · tâche », puis la saisie).
  - Avec `--plain`, on n'a que du texte brut.
- **Les touches.** Entrée envoie au premier coup une `request`, qui ouvre la tâche, puis des `instruction`. Échap envoie `stop`, Ctrl-C quitte (l'équipier continue).
- **`--demande` lance un vrai équipier**, donc un vrai modèle (Gemini par Vertex).
- **Un dépôt inconnu** : la réponse liste les dépôts attachés.
- **L'hôte sur une autre machine** : `--daemon-cmd "ssh luciepc cat .local/share/rag3weaver/daemon.json" --adresse 127.0.0.1:17970`, derrière un `ssh -L 17970:127.0.0.1:7970 luciepc`.
- **Les variables.** `RAG3WEAVER_TTY_GLYPHES=tous` garde les symboles alchimiques du plan astral (sinon `🜂` devient `△` et `🜄` devient `▽`), et `=ascii` donne de l'ASCII pur. `NO_COLOR=1` enlève la couleur. `COLORTERM` et `TERM` décident entre la couleur vraie, 256 et 16 couleurs.

## 4. Ce que le terminal partage avec l'IDE et le chat de référence

- **Avec l'IDE React** (`extension/rag3weaver/ui/ide/`) :
  - le même flux (`/ide/flux`, sujets `agent`, `tokens` et `mailbox`) ;
  - la même boîte de l'équipier (`GET /ide/teammate`, `POST /ide/teammate/mailbox`, `GET /ide/teammate/list` avec l'état `running|waiting|ended|error`) ;
  - le même thème (`daemon.json`) et la même coloration (`POST /highlight`, 15 sortes de spans en octets UTF-8) ;
  - le même choix d'animation, selon la `policy` de l'outil : lire → Transmutation, écrire ou exécuter → Glitch, et le modèle qui réfléchit → Cercle.
- **Avec le chat de référence** (`extension/rag3weaver/ui/chat/`, servi par `scripts/chat_app.py`, un autre serveur que l'hôte) : seulement la forme d'un chat (texte au fil de l'eau, appels d'outils, raisonnement). Le terminal n'y est pas branché.
- **La forme d'une trame du flux** : `id: N` puis `data: {"id","sujet","backend":"<manifeste du worktree>","at_ms","evenement":{"kind":…}}`.
  - Les genres utiles : `Token{run,seq,text}`, `LlmCall`, `ToolCallStarted{tool,arguments}`, `ToolCallFinished{ok,error_kind,ms,bytes,result_excerpt?,truncated?}`, `RunStarted`, `RunFinished`, `MailboxLine{parent,task,line}`.
  - `event: lacune` arrive quand le tampon de l'hôte (2048 événements) ne garde plus ce qu'on demande : il faut alors relire l'état.
  - Le terminal ne garde que les trames dont `backend` est le manifeste du worktree de sa tâche (`result.backend` de `GET /ide/teammate`).

## 5. Les procédures

- **Fait foi** : `docs/procedures/push-d-un-lot.md`.
- **L'identité git** : `user.email=luciedefraiteur@gmail.com`, `user.name="Lucie Defraiteur"`, jamais d'adresse sairen et aucune ligne d'attribution à une IA.
- **Le worktree** : un worktree frère par lot (`git worktree add ../rag3db-tty -b <branche> origin/master`), puis tout de suite `git submodule update --init extension/rag3weaver/codeparsers`, car un worktree neuf n'a pas ce sous-module. On n'écrit jamais dans l'arbre de Lucie (`~/git_workspaces/rag3db`).
- **Les commits** : chaque commit est poussé sur sa branche dans la minute. Après un rebase, la branche prend un nom neuf (`-2`, `-3`) ; jamais de force, jamais de suppression d'une branche distante sans Lucie.
- **Le verrou du poste** : `~/.cache/rag3weaver-build/poste` sert pour tout travail lourd (`poste lourd timeout N <commande>`), et `poste --qui` dit qui tient le poste. `-j 8` au plus. Le target va sous `~/.cache/rag3weaver-build/`, jamais dans `/tmp`, qui est de la mémoire vive sur ce poste.
- **La lib du moteur**, épinglée par son identifiant complet. Exemple au 8 nov. : `RAG3DB_LIB=rag3db-build-cbc06c3e1c78b40d58d3644b1f2539a6e615b991-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`, avec `RAG3WEAVER_MOTEUR_ANCIEN=1` quand master porte du C++ plus récent que la lib. Demander à c7 la lib du moment.
- **La passe d'un lot du crate seul** :
  - `cargo test` et `cargo test --no-default-features` dans `extension/rag3weaver/tty`, puis `./run_e2e.sh --features openai-llm,anthropic-llm --test e2e_tty e2e_tty` dans `extension/rag3weaver` ;
  - plus `e2e_catalogue_gabarits` si `templates/` bouge ;
  - si le lot touche l'hôte, `rag3weaver/Cargo.toml` ou `src/`, c'est la passe entière (le script commun de luciepc `~/.cache/rag3weaver-build/passe_epinglee.sh <worktree> <journaux> <filtre>`).
- **Le push** : sous `poste push <session> -- <script>`, seulement si le verdict est exactement `VERT`. Ensuite, les trois contrôles : `git status` propre, aucun rebase en cours, `HEAD` égal à `origin/master` après un `fetch`. Le hash part à c7.
- **Les docs seules** se poussent sans passe. Une capacité nouvelle prend une ligne dans `docs/nouveautes.md`.
- **Les tickets** : un défaut connu = un fichier dans `docs/tickets/`, avec un en-tête (`- **État** :`, `- **Famille** : moteur|produit`, `- **Gravité** :`, `- **Atteignable en service** :`, `- **Touche rag3weaver** :`), contrôlé par `python3 extension/rag3weaver/scripts/check_tickets.py`.

## 6. Les pièges appris

- **La base d'une passe** : une passe ne vaut que sur la base qu'elle a jouée. Avant de pousser, `git diff --name-only <base> origin/master` hors `.md` : du Rust arrivé sous `extension/rag3weaver` oblige à rejouer. Le premier push de tty-1 a dû être rejoué pour ça.
- **Le pointeur du sous-module** : après un rebase, `codeparsers` peut avoir bougé, et le script de push refuse l'arbre « sale » tant que `git submodule update` n'est pas passé.
- **Les deux tables fondues** : un rebase peut fondre deux ajouts dans une même table de docs, comme l'ancien compte de l'index des tickets. Il faut relancer les contrôles après chaque rebase.
- **Les glyphes du plan astral** (`🜂`, `🜄`) n'ont pas la même largeur partout : on les remplace par défaut. ⛧ et les runes passent.
- **mosh** ne transmet aucune image (kitty, sixel) et n'a la couleur vraie qu'à partir de la 1.4. Tout ce qui compte se dessine donc en cellules de texte.
- **`insert_before` de ratatui** redessine toute la zone vivante : c'est ce qui coûte en octets.
- **`run_e2e.sh` ne filtre que sur son dernier nom positionnel**, et une suite non jouée faute de feature compte comme rouge.
- **La durée d'`e2e_tty`** : environ 4 s, mais une fois 52,9 s avec les mêmes comptes, sans doute la charge de luciepc. À surveiller ; un ticket si ça revient.
- **La lib rag3weaver ne compile pas sans `rag3db-native`** (ticket ci-dessous). Un client portable doit vivre hors de ce crate.
- **luciepc** (`ssh luciepc`, le shell y est fish) : on n'y efface rien sans le dire, on n'y fait aucune action `sudo`, et `~/LR_CodeRag` s'y lit en lecture seule.

## 7. L'histoire du design

- Le Daemon Code Summoning vient d'une page HTML de Lucie de décembre 2025 (ragforge, commit `a25f3ecf9`), copiée telle quelle dans `extension/rag3weaver/docs/07-novembre-2026-tty/05-daemon-code-summoning.html`. On en garde les trois animations, les couleurs `#ff3333` sur `#050505` et les cadences (150, 200 et 100 ms). Rien du vieux code n'est repris.
- Les décisions de Lucie :
  - ratatui, avec plus tard une couche déclarative « façon JSX » à nous ;
  - Windows, macOS et Linux natifs ;
  - le plus interactif possible (la 3D en bonus, par `bevy_ratatui_camera`) ;
  - un rendu se branche et se montre, on ne demande pas d'accord avant ;
  - à l'écran, le vocabulaire git (dépôts, worktrees, branches, PR), jamais « backend ».

## 8. Les tickets ouverts qui sont à moi

| Ticket | En une ligne |
|---|---|
| `docs/tickets/closed/2026-11-07-la-lib-rag3weaver-ne-se-batit-pas-sans-rag3db-native.md` | la lib ne compile pas sans `rag3db-native` (`src/backend.rs` vers la ligne 4411, `src/dataflow/record_nodes.rs` vers la ligne 4567) ; le correctif ou la déclaration « la lib exige le moteur » est à trancher par Lucie |


## Actualisation du 13 novembre 2026 — compilation minimale

Le blocage de compilation sans `rag3db-native` décrit dans l'état historique
ci-dessus est levé. Ticket clos :
`docs/tickets/closed/2026-11-07-la-lib-rag3weaver-ne-se-batit-pas-sans-rag3db-native.md`.
Correctifs `9231f07dc`, `c74e3310d`, garde minimale `5a4049b9e` ; F8 canonique
8/8 sur APP `9ec421069`, preuves publiées par `cdcc032df` dans le rapport
Recherche `docs/recherche/13-11-2026/13-preuves-finales-union-garde-target.md`.
Tty a relu les sources, la garde et les journaux minimaux conservés ; aucun
Cargo, test, service ni bâti nouveau. Ce défaut n'attend plus de décision de
Lucie. Le crate tty séparé est conservé ; aucun nouveau lot autorisé par cette
régularisation. Les propositions tty-4, mesures et thèmes restent des pistes
historiques en attente d'objectif établi, pas des actions démarrées.

## Ce que la semaine du 13 novembre 2026 a appris

### Les thèmes (`56deb90ea`, `87061f03c`)

- **Cinq thèmes**, chacun un fichier de `extension/rag3weaver/templates/themes/` : `daemon`, `foret`, `cyberpunk`, `cyborg`, `alien`. Ils ont tous la même forme : couleurs en hex, trois animations en données (`reflechit` le Cercle, `lit` la Transmutation, `ecrit` le Glitch), `replis`, `ascii` et la palette `code`. Le terminal les embarque par `include_str!` (`tty/src/theme.rs`, `THEMES`), et on les choisit par `--theme <nom>` ou `RAG3WEAVER_TTY_THEME`. L'IDE lit les mêmes fichiers.
- **La règle de forme** convenue avec archi : on ajoute des champs, on n'en renomme ni n'en retire aucun sans le prévenir, puisque l'IDE les lit par `lib/theme.ts`.
- **La règle de contraste**, la même pour tous les thèmes (`theme::tests::every_theme_keeps_a_minimal_contrast_against_its_background`, WCAG 2.x contre le fond) : 4,5 au moins pour le texte et le code qui se lit, 3 pour la sourdine, les commentaires, la ponctuation et le cadre. Le halo n'y entre pas. Elle a relevé daemon de `#b45a5a` à `#b85d5d` (attribute, label, module).
- **Les témoins qui couvrent chaque thème** : le fichier embarqué octet pour octet, une colonne par glyphe après les replis, l'ASCII pur, et les couleurs du Cercle jusqu'aux cellules.

### La boîte de l'équipier (`212893a17`)

- **La logique** vit dans `tty/src/boite.rs`, sans ratatui, pour que le témoin e2e la joue aussi : une `question`, une `approval` (avec sa `command`) ou une `proposal` de l'équipier attend jusqu'à ce qu'une ligne porte `reply_to: <son id>`. La réponse est `{backend, task, kind: "answer", text, reply_to, approved?}`, et l'état de l'équipier se lit dans `GET /ide/teammate/list?supervisor=supervisor`.
- **Dans la vue** : la ligne d'état dit d'abord ce qui attend ; `o` accorde et `n` refuse une approbation, sans rien taper d'autre ; la saisie répond à une question.
- **Le témoin par le chemin de l'hôte** : `e2e_tty::le_terminal_accorde_une_commande_que_l_equipier_attend`. Un faux équipier (CallbackLlm) demande l'accord pour `echo accorde-moi` sur la porte des commandes du dépôt de test. Le terminal accorde, l'hôte écrit `granted`, la commande se joue, et la liste dit `ended`.
- **Le faux vert attrapé par la garde des résidus.** Le journal des runs de l'hôte s'écrit 250 ms après la fin d'un run, sur son fil : un témoin qui rend son dossier trop tôt laisse un résidu dans le TMPDIR, et `run_e2e.sh` rougit. Le remède est d'appeler `host.journal_des_runs().vider()` avant de rendre le dossier, comme `e2e_juge`.

### La mesure à travers ssh (`ec4f7aa6d`, page `docs/tty/13-11-2026/04-la-mesure-a-travers-ssh.md`)

- **Les outils** : `extension/rag3weaver/tty/mesures/faux_hote.py`, un faux hôte qui joue un pas scripté et note les POST, et `mesure.py`, qui lance le terminal sur l'autre poste par `ssh -tt` dans un pseudo-terminal piloté. Les deux n'utilisent que la bibliothèque standard de Python, et les deux sens ont été joués (portable ↔ luciepc).
- **Les chiffres** : environ 47 ms de l'hôte à l'écran (une image de 33 ms plus un aller-retour), 11 à 20 ms d'écho, 3,1 Ko/s pendant un pas. Le redimensionnement, les touches et les couleurs des cinq thèmes (10/10) sont vérifiés.
- **Le défaut trouvé : ESC[6n.** En vue inline, ratatui demande la position du curseur ; sans réponse, il paniquait. La vue s'ouvre maintenant par `try_init_with_options`, avant tout envoi à l'hôte, puis passe en `--plain` en le disant. Le harnais doit répondre à `ESC[6n` comme un vrai terminal ; sa séance 0 vérifie le repli.
- **Les pièges du harnais** : `setsid` rend la main tout de suite, si bien que le pid noté n'est pas celui du faux hôte, qui écrit maintenant le sien ; `script` lit le reste d'un `bash -s` comme des frappes ; à luciepc, le shell de connexion est fish, donc on passe toujours par `bash -s`.

### Les passes et les pushes de la semaine

- **Le repli moteur ancien, toujours dit.** Quand master porte du C++ plus récent que la lib épinglée, `run_e2e.sh` refuse sans `RAG3WEAVER_MOTEUR_ANCIEN=1`. On ne le pose qu'avec l'accord de c7 et en le disant dans le message de push et dans la page (« repli moteur ancien, lot sans effet sur le moteur, lib <id> »). C'est ce qu'a fait tty-5 sur `e93710f36`.
- **Un rouge identique sur master seul** (même lib, target neuf), avec son ticket, entre dans un push comme rouge nommé, sur le mot de c7. Le script de passe n'accepte alors que cet échec-là, par son nom exact. Tout autre rouge reste rouge. Exemple : `e2e_catalogue_gabarits::la_synchronisation_alimente_la_couche_du_projet`, que hotfixs a corrigé dans le test (`c13c76f72`).
- **Un service d'embarquement qui se tait** (7879, bge-m3, à luciepc, le 13 nov. vers 23 h 50) fait tomber au démarrage les tests qui l'exigent. On ne touche pas au service : on attend qu'il revienne, puis on rejoue la suite seule sur la même tête, et le premier essai reste dans le journal. Si ça revient, c'est un ticket pour la session « embarquement ».
- **Rebaser après un ticket déplacé par un autre lot** : `git -c merge.directoryRenames=false rebase origin/master`. Sinon git voit un dossier renommé, et range un ticket ouvert parmi les fermés.
- **Un worktree neuf à luciepc** doit lui aussi passer par `git submodule update --init extension/rag3weaver/codeparsers`, sinon le bâti de l'e2e échoue sur `codeparsers`.

### Les tickets

- Fermé : `docs/tickets/closed/2026-11-13-un-gabarit-masque-puis-retire-ne-revient-pas-a-la-bibliotheque.md` (hotfixs, `c13c76f72`). C'était le test et non `sync_templates`.
- Ouvert : `docs/tty/tickets/2026-11-13-le-terminal-sous-windows-et-macos-a-jouer.md`, à jouer quand une machine existe.

