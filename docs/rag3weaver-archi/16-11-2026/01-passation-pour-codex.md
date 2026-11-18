# Passation d'archi pour Codex — le lundi 16 novembre 2026

Cette page est pour une session Codex neuve, sans mémoire de nos échanges. Elle part des docs et des
tickets seuls. Le périmètre d'archi est **le pas de l'équipier et l'hôte de rag3weaver** : la boucle
de l'équipier (`extension/rag3weaver/src/daemon_host/teammate.rs`), l'invite et les outils d'un tour
(`daemon_host/chat.rs`, `teammate_tools.rs`), les tâches (`daemon_host/taches.rs`), le socle d'un
agent de code (greffé par `backend.rs`), le pont navigateur (`src/pont_navigateur/`). Les rapports
détaillés sont dans `docs/rag3weaver-archi/15-11-2026/` (pages 01 à 12) ; la vision qui guide le
tout : `docs/orchestration/visions/15-11-2026-l-agent-est-un-agent-de-code-qui-gere-ses-worktrees.md`
et `docs/orchestration/visions/16-11-2026-algareth-pair-des-sessions-claude.md`.

## 1. État

### Sur master (le 15 et le 16 novembre)

| Lot | Commits | Ce qu'il fait |
|---|---|---|
| La file des écritures sous une fermeture qui tient | `95c643a20` | une écriture reçue pendant une fermeture se range et se rejoue à `POST /backend/open` |
| Le cœur Rust du pont navigateur | `36ee817a7` | `src/pont_navigateur/` : cadrage Native Messaging, protocole révision 5, baux, gardes |
| Le relai du pont navigateur | `5ff5bdfb2` | l'exécutable `rag3weaver-pont-navigateur` et l'écoute du socket de l'hôte |
| La garde B : une demande attend une ligne | `6d5fdb75c` | un équipier muet est relancé une fois, puis attend au plus `teammate.reply_wait_s`, puis échec nommé |
| Le côté pas du socle | `95eae09e6` | droit d'écrire relu à chaque ouverture, refus juste dans une tâche, `git status` sous Landlock, invite d'agent de code |
| Le socle d'un agent de code dans toute tâche | `8de626586`, `451200d3e` | `write`, `glob`, `replace_lines`, `insert_lines`, `replace_text` greffés ; l'empreinte de lecture dans le pas |
| Le nom de l'agent (Algareth) | `3e8587f0e` | le nom et la persona dans l'invite ; `name` sur les lignes de l'équipier, `from` reste `teammate` |

### En branche

- **`pas-va-au-bout`**, tête `12a7f6987` (worktree `~/.cache/rag3weaver-build/wt-delegate`) : un pas va
  au bout de la demande ; un pas qui répond sans `step_done` est relancé ; la copie de l'hôte porte
  `origin: "host_copy"`. **Vert** sur `7f9465fa9` : témoins 62/0, lib entière 2531/0/72, e2e_hooks
  et navigateur 45/0, e2e_equipier sous Vertex 2/0. **Pour atterrir** : rebase sur master (du code
  est arrivé depuis), rejeu court, « prêt » à l'orchestration (c7), push à son « à toi ». Archi le
  fait elle-même si elle reprend ; sinon c'est le premier lot de Codex.
- **`navigation-membres`**, tête `49d547a07` : les membres d'un type dans la navigation des symboles.
  Vert à son heure, 1 138 commits derrière master : à reposer et rejouer, après le travail
  d'appartenance de codeparsers.
- **Le fork du pont navigateur** : `https://github.com/L-Defraiteur/BrowserMCP`, branche
  `rag3weaver-gardes`, tête `0f9e2a5` (clone `~/.cache/rag3weaver-build/pont-fork`). L'extension n'est
  pas installée chez Lucie.

### Ce qui tourne, et les pièges

- L'hôte de Lucie (7970) des deux postes est posé par l'optimiseur (G) ; ne jamais l'arrêter soi-même.
- La lib du moteur est épinglée : `RAG3DB_LIB=rag3db-build-373b5477b4fd413f96605061b8d09393439a1a30-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
  Un lot Rust seul joue ses e2e sous `RAG3WEAVER_MOTEUR_ANCIEN=1`, et le dit au journal.
- **Suites exigées par chemins** (G, 16 novembre) : un lot qui touche `code.rs`, `code_sync.rs`,
  `catalog.rs` ou `catalog/*`, `generations.rs`, `marque_d_ecrivain.rs` ou `tests/e2e_code_sync.rs`
  rejoue `e2e_code_sync` avant son push. Un lot qui touche l'hôte rejoue `e2e_demon_hote` et
  `e2e_lectures`. Un lot qui touche l'équipier rejoue `e2e_equipier` **sous Vertex** (variables
  ci-dessous), sinon il est sauté, et une suite sautée est rouge.
- Toute passe lourde passe par `POSTE_MEM_MAX=32G ./poste lourd …` et **sous `timeout`**, attente de
  la place comprise. Le portable n'a qu'une place lourde ; luciepc en a quatre.
- Un `cargo test` direct contourne la garde du target : les gabarits embarqués peuvent venir d'un
  autre worktree. L'étape « module » d'un script de passe appelle d'abord
  `. scripts/garde_target.sh && garder_le_target "$CARGO_TARGET_DIR" "$W"`.
- Au plafond du target (32 Go) : `cargo clean -p rag3weaver`, pas `rm -rf`.

## 2. Travail pour Codex, dans l'ordre

Chaque entrée est un ticket ouvert sous `docs/rag3weaver-archi/tickets/`. Le ticket dit quoi faire
(fichiers, forme, ce qui ne se touche pas), comment le prouver, sa taille et son blocage. Ordre
recommandé : la tâche d'écriture de la campagne D6 (`docs/orchestration/16-11-2026/01-essais-empiriques-algareth.md`)
doit passer avant le reste.

1. Faire atterrir **`pas-va-au-bout`** (ci-dessus) : 1 h.
   **Fait par archi le 18 novembre : `9057cad4e` sur master** (pas-va-au-bout-6, avec le superviseur d'e2e_equipier ; voir `18-11-2026/02-pas-va-au-bout-3.md`). Ci-dessous, l'état du 16 :
   **En cours par archi, prêt au push** (16 novembre, 10 h 12). La branche est reposée sur master
   sous le nom **`pas-va-au-bout-2`**, tête **`5e25a110d`** (worktree
   `~/.cache/rag3weaver-build/wt-delegate`), base testée `6b672afa6`. Le rejeu court est **vert** :
   - module `daemon_host::teammate`/`chat` : 62/0 ;
   - check_features, e2e_demon_hote 24/0, e2e_lectures 27/0 (pilefile) ;
   - e2e_equipier sous Vertex : 2/0.

   Tout tourne sous `RAG3WEAVER_MOTEUR_ANCIEN=1` et la lib 373b5477b. Les journaux sont dans
   `~/.cache/rag3weaver-build/bout-module/`, `bout-pilefile/`, `bout-e2eseul/` et `bout2-rejeu.log`.
   **Pour finir** :
   1. Préparer le dossier de journaux du push, avec `resume.log` (ligne 1 `commit 5e25a110d — …`,
      dernière ligne `VERT`) et `base` (`6b672afa6`).
   2. Pousser par la garde au « à toi » de l'orchestration. La garde rejoue si du code est arrivé
      sur master depuis `6b672afa6`.
2. `2026-11-16-le-plafond-d-appels-d-un-pas.md` (S1) : 4 h.
3. `2026-11-16-le-pas-suivant-ne-voit-pas-les-retours-d-outils.md` (S2) : 1 jour.
4. `2026-11-16-une-request-remplace-la-demande-en-cours.md` (S3) : 3 h.
5. `2026-11-16-la-garde-b-pour-une-consigne.md` : 3 h.
6. `2026-11-16-step-done-avant-report-relance-un-pas-entier.md` (S4) : 3 h.
7. `2026-11-16-l-agent-se-presente-a-chaque-run.md` (S5) : 1 h.
8. `2026-11-16-la-demande-figee-au-journal-des-runs.md` (J1, et le nom au journal) : 2 h.
9. `2026-11-16-la-position-du-worktree-de-la-tache.md` : 5 h.
10. `2026-11-16-l-attente-d-index-d-un-pas.md` : 1 jour (le contrat de recherche est sur master).
11. `2026-11-16-delegate-l-equipier-delegue.md` : 3 jours, en trois lots.
12. `2026-11-16-le-mode-pair-par-le-registre.md` : relevé 2 h, conception 4 h, code 3 jours.
13. `2026-11-16-l-affichage-des-messages-entre-sessions.md` : 1 jour, pour tty.

Après chaque lot qui touche l'équipier, la campagne D6 se rejoue (c7) : c'est elle qui juge.

**Comment jouer une passe.** Modèle de script : `~/.cache/rag3weaver-build/wt-bout-passe.sh`. Copier
en changeant `W` (le worktree) et `OUT`. Étapes : `ETAPE=module FILTRE="daemon_host::teammate
daemon_host::chat"`, `ETAPE=complete` (check_features, la lib entière, e2e_hooks, navigateur),
`ETAPE=pilefile` (rejeu court). e2e_equipier sous Vertex :
`GOOGLE_APPLICATION_CREDENTIALS=/home/lucied/git_workspaces/rag3db/.vault/vertex-sa.json
GOOGLE_CLOUD_PROJECT=lr-hub-472010 GOOGLE_CLOUD_LOCATION=global RAG3WEAVER_EXIGER_GEMINI=1
./run_e2e.sh --test e2e_equipier`, dans le crate, après `exporter_la_lib`. Le fichier de clés ne se
lit ni ne s'affiche jamais : on passe son seul chemin.

**Comment pousser.** `docs/procedures/push-d-un-lot.md`. Préparer un dossier de journaux sous
`~/.cache/rag3weaver-build/<nom>/` : `resume.log` (ligne 1 `commit <hash9> — …`, dernière ligne
`VERT`), `base` (le parent du premier commit), et les journaux de la passe qui nomment la lib.
Puis `if tail -1 <journaux>/resume.log | grep -qx VERT; then ./poste push <session> --
/home/lucied/.cache/rag3weaver-build/push_garde.sh ../.cache/rag3weaver-build/<worktree> <journaux>; fi`,
au « à toi » de l'orchestration seulement.

## 3. Ce qui attend Lucie (Codex n'y touche pas)

- **L'extension du pont navigateur** : la charger non empaquetée dans son Chrome, et poser le manifeste
  d'hôte natif (`hote_natif.rs` le compose) dans son profil.
- **Le registre de Claude Code** : son mot couvre l'entrée témoin `algareth-temoin` (16 novembre) ;
  toute autre écriture dans `~/.claude/sessions` lui revient.
- **Les canaux de Claude Code** : non pour l'instant (« ça voudrait dire tous vous redémarrer ») ; une
  page de préparation seulement.
- La fin de session : rien ne s'efface (worktrees, targets) avant son mot.

## 4. Règles du périmètre

- **Langue** : identifiants, commandes `/…`, options, clés de manifeste et de JSON, noms d'outils, de
  tests et d'unités systemd, descriptions d'outils, gabarits produit et textes que l'hôte écrit pour
  le modèle : **en anglais**, pour tout code neuf. Prose des docs, commits et tickets : en français.
  On traduit l'existant quand on le touche, pas en lot.
- **Un worktree et un target à soi**, branches `cx-…`. L'arbre de Lucie (`~/git_workspaces/rag3db`)
  ne reçoit aucun commit et ne change jamais de branche.
- **Rouge d'abord** : chaque témoin se voit rouge avant le correctif. Une suite non jouée, ou sautée,
  est rouge.
- **Passe et push** par `docs/procedures/push-d-un-lot.md`, jamais sans le « à toi » de
  l'orchestration. Chaque commit se pousse sur sa branche dans la minute. Jamais de force push.
- **Identité** : `luciedefraiteur@gmail.com`, **zéro attribution** d'assistant dans les commits.
- **Une question à Lucie ne bloque jamais le reste** : elle s'écrit dans la page, l'orchestration la
  porte, et le travail continue sur ce qui n'en dépend pas.

## État final d'archi (16 novembre, 10 h 2x)

Archi s'arrête sur le mot de Lucie. Voici ce qui est posé :
- la passation (`11b7e8da3`) ;
- le relevé de la trame du bus (`33d532035`, page 02) ;
- `pas-va-au-bout-2`, vert, poussé sur sa branche, pas sur master.

Aucune passe ne tourne. La seconde entrée témoin (qui déclare `notify_idle`) attend le mot de
Lucie : la question est dans la page 02.
