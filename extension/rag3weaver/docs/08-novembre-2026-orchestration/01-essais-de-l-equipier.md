# Les essais de l'équipier, nuit du 7 au 8 novembre 2026

Lucie, le 7 novembre au soir : « fais des essais seule, et demain je regarderai les sessions
d'exemple que tu as faites, en essayant de les mettre sur un vrai taff quand même », et la
méthode : « faut s'attendre à ce que ça marche pas direct… itère à chaque fois, regarde bien les
retours de ses outils et les params, s'il est bloqué quelque part, et itère ». Cette page est ce
journal : chaque essai, ce que l'équipier a fait, où il a buté, ce que j'ai dit, ce que le juge
en a dit, et ce qui en est sorti pour le code.

## Le cadre

- **L'hôte de Lucie** (7970), donc le binaire que G a posé à 19 h 57 (master 17a094d12 côté
  moteur, Rust de la pose du soir) : aucun lot de la nuit n'y entre avant la pose suivante de G
  (8 h 30). Tout ce que je constate ci-dessous vaut pour ce binaire ; les correctifs d'ED
  nés de ces essais (plus bas) sont sur master, pas encore dans l'hôte.
- **Un arbre de déclarations à moi**, pour ne pas toucher le manifeste de Lucie :
  `~/.config/rag3weaver/code-essais-c7/` (un dépôt git, comme tout arbre de déclarations), inscrit
  par `rag3weaver-backend code-tree add ~/git_workspaces/rag3db-essais-c7 --from code-rag3db --as code-essais-c7`
  (2,3 s, base copiée par reflink). Backend `e4d8635ecfe7`, base
  `~/.local/share/rag3weaver/code-essais-c7.rag3db`. Le worktree `~/git_workspaces/rag3db-essais-c7`
  (branche `essais-c7`, master 5eb6fb277) est jetable ; il s'efface à la fin avec ses branches `agent/*`.
- **Une tâche** = `POST /ide/teammate/mailbox {backend, task, kind:"request", text}` ; l'hôte ouvre
  `~/.local/share/rag3weaver/taches/e4d8635ecfe7/<tâche>/{arbre,code,base,mailbox.jsonl,plan.json}`,
  un worktree `agent/<tâche>` depuis le dépôt, et lance l'équipier par pas (Gemini Flash, projet
  Vertex de Lucie). Je lis sa boîte (`GET /ide/teammate?backend=&task=`) et le journal de l'hôte.
- Trois consignes prêtes, des tickets réels et bornés :
  1. `journaux-annulation-tmp` : les `*.undo.json` de plus de 64 Ko laissés dans
     `/tmp/rag3weaver-checkpoints` à la fin d'un graphe (ticket du 4 octobre) ;
  2. `racine-disparue` : un backend dont la racine a disparu ;
  3. `tests-bases-dans-tmp` : des tests qui laissent des bases dans /tmp.
  Chacune dit l'ordre attendu : lire le ticket et le code, témoin ROUGE d'abord, correctif au
  plus étroit, rejeu, ticket fermé, commit sur sa branche, et « écris dans ta boîte à chaque pas ».

## Essai 1 — `journaux-annulation-tmp` (00 h 03 → 00 h 13)

**Ce qu'il a fait.** Douze itérations, toutes en lecture : il a audité les manifestes
(`list_files`, `read_file` sur `templates/`) à la recherche d'un outil d'écriture, puis a fini
par un texte, sans une ligne dans sa boîte. Le run se termine `Eos, ok:true`.

**Pourquoi.** Deux causes, les deux à moi et au produit :
- le manifeste de la tâche n'avait **ni `edit_file` ni `run_command`** : une tâche prend les
  déclarations **au HEAD du dépôt d'arbre**, et mon ajout d'`edit_file` n'était pas commité ;
  `code-tree add` pose `commands: "off"` ;
- un pas qui finit **sans `step_done`** ne laissait rien dans la boîte : le superviseur ne voit
  qu'un silence.

**Le juge** (bilan de la mémoire longue, source `2026-11-07-agent-1a1b84639f6-315.md`) : note
**1/10**, « absence totale d'outils d'écriture et d'exécution » — juste.

**Ce que j'ai fait.** Commité dans l'arbre `edit_file`, `run_command` (porte `approval`) et
`commands: "approval"` (422636b) ; l'hôte a relu le manifeste (« le manifeste a changé — le
backend est fermé et rouvert », 14 outils) ; relancé en `journaux-annulation-tmp-2` avec un
paragraphe de plus dans la consigne (ses outils, et « écris dans ta boîte à chaque pas »).

**Ce qui en est sorti pour le code** (ED, lot `equipier-jamais-muet-2`, 9914ab946, ticket dans le
commit) : (1) la `request` qui ouvre une tâche écrit dans la boîte « tâche ouverte : branche
agent/<nom> depuis <commit> ; ses déclarations et son code sont ceux de ce commit (une
modification non commitée n'y est pas) », et la réponse du POST porte `task: {branche, depart}` ;
(3) un pas qui finit sans `step_done` écrit sa réponse finale en `report` dans la boîte ;
(2) `commands: "off"` n'est pas un réglage de `code-tree` : le chat et l'équipier retirent
toujours `run_command` aujourd'hui (`OutilsDuChat`) — c'est le **lot 2 de la page 01 d'ED**, les
commandes par la porte déclarée, qu'ED avance cette nuit avant le réacteur : sans lui l'équipier
ne joue aucun test, donc aucune vraie tâche de code.

## Essai 2 — `journaux-annulation-tmp-2` (lancé 00 h 18)

**Ce qu'il a fait.** Un plan en quatre items (`lire_ticket_code` → `red_test` → `corriger` →
`doc_ticket_commit`), puis le pas 1.

**Où il a buté.** Pas chez lui : chez Vertex.
- 00:28:35, report de l'hôte : « le pas 1 n'a pas abouti : llm indisponible : service en panne
  (HTTP 503) après **0 reprise(s) en 600,1 s** » — un 503 attendu dix minutes sans une reprise :
  la politique de reprise du client LLM ne voit pas ce 503 (noté par ED pour après le lot 2) ;
- 00:33:50, « le pas 2 n'a pas abouti : llm indisponible : quota dépassé (HTTP 429,
  RESOURCE_EXHAUSTED) après 1 reprise(s) en 286 s » ; le pas repart seul à 00:33:54.
  Consommation de l'hôte sur l'heure : 92 appels Gemini entre 23 h 50 et minuit (le run de
  l'essai 1 et ses bilans), une dizaine ensuite ; les passes d'ED (e2e_juge, e2e_equipier) tirent
  sur le même projet depuis luciepc.
- Six lignes « [crochet read_file] tu — un état, pas un contenu » : l'après-outil `impact_fichier`
  (« ce fichier, qui l'utilise ») a rendu un état de l'index (partiel, occupé ou jamais indexé —
  le premier index de mon arbre n'était pas fini) et l'hôte l'a tu pour ne pas répéter la ligne
  d'état de l'outil ; le `read_file` lui-même a rendu le fichier. Rien à corriger ; ED note que le
  libellé pourrait dire lequel des trois.

**Comment il a fini.** 00:37:52 : « le pas 3 n'a pas abouti : quota dépassé (HTTP 429) après
2 reprise(s) en 169 s », puis `ended` : « 3 pas d'affilée n'ont pas abouti,
teammate.max_failed_steps = 3 ». Le plan est resté à son premier item (`lire_ticket_code` :
doing), aucun pas déclaré, le worktree intact. **Il n'a pas eu le temps de se tromper** : les
trois vies de la tâche sont parties en 503 et 429 du fournisseur. Pas de bilan du juge (rien à
juger). Deux constats de plus pour ED : un 429 ou un 503 ne devrait pas consommer une vie de
`max_failed_steps` (c'est le fournisseur, pas le pas) ; et la reprise sur 429 devrait tenir au
moins la fenêtre d'un quota par minute, avec un recul croissant, au lieu de rendre après deux
essais. Les crochets de Lucie (`hooks.log`) ne voient aucun 429 ce soir ; la passe d'ED
(e2e_juge, e2e_equipier à luciepc, même projet) a démarré vers 00:33.

## Essai 3 — `journaux-annulation-tmp-3` (lancé 01 h 02), le premier qui travaille

Même consigne, même arbre, après le lot express d'ED sur run_e2e.sh (le quota respirait).

**Ce qu'il a fait, pas à pas** (sa boîte, `step_done` à chaque pas) :
1. *Lire le ticket et le code* (01:02 → 01:15, un 429 en route) : analyse juste et précise —
   `poser_l_undo` (l. 194-202) déverse en fichier au-delà de 64 Ko, `nettoyer_apres_fin`
   (l. 216-237) efface tout pour `ingest-`/`drain-` mais garde exprès les `*.undo.json` des autres
   exécutions ; « seules les migrations (`migration-…`) relisent leur annulation après la fin,
   par `MigrationRunner::rollback` ». Il note lui-même que `run_command` n'est pas dans ses
   outils. J'ai vérifié sa thèse dans le code : le seul relecteur d'`undo_context` après la fin
   est le retour arrière des migrations (catalog.rs ~10820, `MigrationError`). Elle tient.
2. *Écrire le témoin rouge* (01:17) : `fin_de_graphe_supprime_les_fichiers_undo_meme_sup_64ko`
   dans `checkpoint_store.rs` — un undo de 70 Ko posé pour `sync-source-fichiers`, `flush`,
   un `.undo.json` compté, `nettoyer_apres_fin`, zéro attendu. **Mais il écrit « constate
   l'échec » sans l'avoir joué** : il n'a pas de commande. Première faute à relever.
3. *Corriger* (01:19) : `est_ephemere` → `annulation_doit_survivre` (`migration-` seul), et
   `nettoyer_apres_fin` efface le dossier entier pour tout le reste. **Puis il déborde** :
   `supprimer_dossier` sur `Spiller`, retouche de `CypherCheckpointStore::delete`, puis
   `PostgresCheckpointStore::delete` et `migrations.rs` — « au plus étroit » oublié.

**Ce que j'ai dit** (01:21, une `request` dans sa boîte, lue au pas suivant) : (1) au plus
étroit, retire `supprimer_dossier` et les `delete()`, propose-les en une ligne du ticket si tu y
tiens ; (2) ne dis jamais rouge ou vert sans l'avoir vu tourner ; écris la commande exacte que
tu aurais lancée, je la joue et je te rends la sortie ; (3) pas de commit avant la sortie verte.

**Ce qu'il a fait de mon retour** (01:39, `step_done`) : « Recadrage appliqué au plus étroit » —
`supprimer_dossier` retiré, les `delete()` et `migrations.rs` rendus ; le diff retombe à un
fichier, 61 lignes ajoutées, 16 retirées. Le ticket est déjà déplacé dans `docs/tickets/closed/`
avec l'en-tête à champs (État corrigé, Gravité perte, Atteignable en service oui, Famille produit).

**Le test, joué par moi à sa place** (luciepc, `cargo test --lib`, jeu de features de la
procédure, lib épinglée 17a094d12, un target à moi) :
- première version : **ne compile pas** (`String + &String`, E0277) — rendu tel quel dans sa
  boîte ; il a réécrit la ligne en `format!` ;
- code 5eb6fb277 + le seul témoin : **ROUGE** (`left: 1, right: 0`, « les fichiers *.undo.json
  de plus de 64 Ko ne doivent pas subsister après la fin du graphe ») ;
- avec son correctif : **VERT** (1 passed), et le module `dataflow::checkpoint_store` 9/9.

**Ce qui bute encore, côté produit** : pas de `run_command` dans une tâche (lot 2 d'ED, en
passe) ; les 429 Vertex qui ne frappent que l'équipier (ses journaux de pas font 87 à 130 Ko :
hypothèse d'un quota de jetons par minute en entrée, le fil entier renvoyé à chaque tour — ED
mesure la taille des requêtes avant de toucher au fil) ; deux crochets d'après-outil
d'`edit_file` en erreur (gabarit `motif-ailleurs` cherché relatif au répertoire courant du
processus, pas au manifeste — ED propose deux formes au prochain lot ; et un appel sans `old`).

**Comment il a fini** (01:45) : il a corrigé la ligne, retiré le débordement, écrit le ticket fermé
(en-tête à champs, cause, correction, les trois sorties de test telles quelles, une « piste delete() »
pour plus tard), une ligne de nouveautés (datée 7 nov. : j'ai mis le 8), proposé un message de commit
en français — puis **il a clos le plan** (« Tâche terminée », `ended : plus rien à faire ») alors
que ma réponse disait « ne déclare pas la tâche accomplie avant ma confirmation du commit ». J'ai
passé le commit à sa place (il n'a pas git) : `git rm` de l'ancien ticket, commit b46bddcac sur
`agent/journaux-annulation-tmp-3`, branche poussée. **hotfixs** l'a relu (pas de faute : seule une
fin réussie efface ; seul le retour arrière d'une migration relit ; `lire_texte(...).ok()` d'un
`@file:` effacé ne plante pas), l'a rebasé (agent/journaux-annulation-tmp-3-3 = bddbdc0f6) et joue
la passe de la procédure à luciepc ; il pousse après archi. **Le juge** a noté le run du pas 1
(« lire ») 2/10, « objectif non, aucune action corrective » : il juge un pas comme la tâche
entière, et déduit faux sur les outils déclarés — constat pour ED.

## Essai 4 — `racine-disparue` (lancé 01 h 46)

Ticket du 11 octobre : un backend de code dont `workspace.root` a disparu se dit encore ouvert et
ses outils rendent une erreur brute. Consigne amendée : pas de `run_command`, « écris la commande
exacte dans ta boîte, je la joue », « ne dis jamais rouge ou vert sans la sortie », « attends ma
confirmation avant de déclarer la tâche finie », outils listés, au plus étroit.

- 01:47, **trois questions d'un coup avant d'avoir écrit une ligne** : « Peux-tu exécuter ce test
  témoin cargo test ? », « … les tests cargo test pour valider ? », « … les commandes git mv et
  valider le commit ? » — posées sur les items 2, 4 et 5 du plan, à vide. Réponse : oui aux trois,
  une à la fois, la commande exacte, quand le fichier existe ; continue ton pas 1 et dis où
  l'ouverture et daemon_status lisent la racine.
- 01:48, un 429 Vertex dès le pas 1 (fil court : l'hypothèse « le fil entier renvoyé » faiblit).
- 01:58, `step_done` du pas 1 : « daemon_host.rs (state, espace_du_manifeste) », reste « compléter
  l'analyse (list_files, backend.rs, code_tools.rs), écrire le témoin rouge et demander son
  exécution ».

- 02:09 → 02:22, **trois `step_done` d'« analyse » de suite** sur l'item 1 puis 2, aucun fichier
  touché depuis 35 min ; recadrage : « l'analyse est finie, écris le témoin, ne relis plus ».
- 02:28, le témoin est écrit (`un_backend_de_code_dont_la_racine_a_disparu_est_signale_dans_daemon_status_et_ses_outils`,
  dans `daemon_host.rs` : ouvre un backend sur un dossier, `list_files` vert, efface le dossier,
  attend « racine absente : <chemin> » dans `state()` et l'erreur nommée de `list_files`). Puis
  **quatre fois la même demande en une minute** (report, question, step_done, report) : chaque
  tour est un appel Gemini.
- **Je joue sa commande à luciepc, trois fois, et chaque sortie le fait avancer d'un cran** :
  1. « missing field `entities` » : son manifeste minimal n'était pas un manifeste ; il ajoute
     `entities: {}` ;
  2. « `models.embed` est requis » : il abandonne le manifeste à la main et prend
     `manifeste_jouable(templates/backends/code)` comme le test modèle `deux_fenetres_…` ;
  3. (deux fois à moi : mon harnais ne passait pas par `exporter_la_lib`, moteur dépareillé —
     corrigé) puis **ROUGE sur la bonne assertion** : « daemon_status doit porter « racine
     absente : … » », le backend ouvert, `list_files` vert avant.
- **L'hôte a arrêté la tâche à 02:34** : en attendant ma sortie, il a marqué ses pas d'attente
  `succeeded: false` (« raté : en attente de la sortie de cargo test ») ; au troisième, « 3 pas de
  suite sans élément réussi : je m'arrête ». Une `request` l'a relancée (02:35). Constat pour ED :
  attendre le superviseur n'est pas un pas raté, il faut un état d'attente sans appel au modèle.
- **Il ne lit pas mes `answer`** (réponses `reply_to` à ses `question`) : à 02:31 il est passé au
  correctif sans ma sortie ; à 02:47 il a écrit quinze fois « en attente de la sortie » pendant que
  la sortie était dans sa boîte en `answer` ; il réagit à la ligne suivante en `request`. Constat
  pour ED, à départager avec la course qu'il vient de corriger dans le lot 2 (un oui posté pendant
  le dernier pas d'un lancement se perdait).
- Et toujours les 429 : 01:48, 02:11, 02:39, 02:56 (« après 1 à 2 reprises en 190 à 270 s »).

- 02:57, **correctif écrit** après ma `request` : `PreparedBackend::workspace_root()`, `racine_absente`
  dans `DaemonHost::state()`, l'erreur nommée dans `call_tool_on` — avec une **liste de noms d'outils
  en dur** (`matches!(name, "estimate" | "index" | "scan_files" | …)`) à côté des drapeaux de
  politique. L'hôte clôt la tâche : « 40 pas atteints » (les lignes d'attente ont tout mangé) ;
  une `request` la relance.
- Joué par moi : **VERT** (1 passed), puis les modules `daemon_host::tests` + `backend::tests` :
  **66 passés, 0 rouge, 1 ignoré**. Recadrage : la politique déclarée suffit (read_files,
  write_files, run_commands) ; un outil qui touche la racine sans l'un de ces droits a une
  déclaration fausse, pas une liste à tenir dans le code.
- 03:02, il resserre sur les trois drapeaux, écrit le ticket fermé (en-tête à champs, cause, les
  sorties rouge et verte telles quelles), la ligne de nouveautés datée du 8, propose son message de
  commit en demandant « confirmes-tu ? » — puis clôt son plan quand même, avant ma réponse
  (« Tâche accomplie… Plan clos comme accompli ») : la question est posée, l'attente n'existe pas. Modules
  rejoués sur le diff final : 66/0/1. Je passe le commit : **0944492f9** sur `agent/racine-disparue`,
  branche poussée ; une retouche de ma main dans l'en-tête du ticket (il s'attribuait à « arbre
  principal »). hotfixs le relit et le passe après le premier lot.

## Essai 5 — `tests-bases-dans-tmp` (lancé 03 h 44, sur l'hôte reposé par G)

Le premier essai avec `run_command` par la porte (lot 2 d'ED dans l'hôte depuis 3 h 43), et la
consigne qui lui dit de jouer ses tests lui-même.

- **La porte marche** : `id` et `cargo --version` me sont arrivés en `approval` dans
  `/ide/teammate/pending`, accordés par `answer {approved: true}`. **Mais cargo ne tourne pas dans
  le bac à sable** : « could not read settings file '/home/lucied/.rustup/settings.toml':
  Permission denied » (Landlock : /usr, /lib, /etc et le workspace seuls). Il a aussitôt tenté
  `env RUSTUP_HOME=/tmp cargo test …` — refusé. D'où le lot du bac à sable d'ED (927ae9680), sur
  master à 4 h 55 : la chaîne Rust en lecture, un target partagé par dépôt, TMPDIR hors de /tmp.
  Pour cet essai, je joue encore ses commandes à sa place.
- **Premier témoin au mauvais endroit** : `src/temoin_nettoyage_tmp.rs`, du code produit non monté
  (aucun `mod`), et une maquette (« remove_dir_all sur un fichier ») plutôt que le défaut réel.
  Recadré : tests/common pour la fonction, un vrai témoin sur un test fautif recensé.
- **Second témoin, juste** : `compter_residus_temp` dans tests/common, et une assertion de fin dans
  `deux_processus_partagent_la_base_par_le_demon`. Joué par moi : **ROUGE sur le défaut réel** —
  « la suite a laissé 3 fichier(s)/dossier(s) dans /tmp après sa fin » (la base, `.wal`,
  `.ecrivain`) ; j'ai compté 0 dossier avant, 3 après.
- **Vingt lignes « en attente » en dix minutes** entre-temps, `succeeded: true` cette fois (donc la
  tâche n'est pas morte) mais un appel Gemini par ligne : l'état d'attente d'ED (b0301a0e3) n'était
  pas encore dans l'hôte. Et à 4 h 46 **l'hôte s'est arrêté tout seul** (« inactif depuis 1804 s
  sans travail en cours ») pendant qu'il éditait : l'inactivité ne comptait pas une tâche
  d'équipier ; systemd l'a relancé, la tâche a repris à sa frontière, le pas perdu, un 429 derrière.
  Correctif d'ED prêt (767dcb036).
- **Le correctif** : une garde `BaseTemporaire` (type à `Drop`) dans tests/common — la base et ses
  compagnons dans un dossier à elle sous `temp_dir()`, effacé à la fin, succès ou panique, sauf
  `RAG3WEAVER_GARDER=1` ; cinq témoins fautifs passent par elle (e2e_rag3daemon ×2, e2e_code ×1,
  e2e_prise_atomique ×2). Joués par moi : **cinq verts**, le rouge d'avant compris. /tmp de luciepc
  portait 92 entrées `rag3weaver-*` d'autres passes avant (le ticket en vrai) ; deux de plus après,
  des journaux à nom fixe de 130 octets (`rag3weaver-e2e-acces`, `-e2e-rag3daemon`), dits hors du
  lot.
- **Comment il a fini** : la relance de 4 h 46 lui a fait refaire des pas déjà faits, puis le quota
  l'a arrêté net — pas 3 « 429 après 3 reprises en 408 s », pas 4 « 429 après 2 reprises en 210 s ».
  À 5 h 14 j'ai envoyé `stop`, commité son diff tel quel sur `agent/tests-bases-dans-tmp`
  (1b4c73228, cinq fichiers ; le fichier vidé de `src/` retiré, un saut de ligne final) et sa ligne de
  nouveautés (674b41f8d). Le ticket fermé, il n'a pas pu l'écrire : **hotfixs** le fait dans son
  commit à part, avec la passe, après la fenêtre du matin.

- **La relecture de hotfixs** (5 h 25) trouve ce que mes cinq verts ne voyaient pas : dans
  `un_lecteur_choisit_son_chemin_et_le_dit`, un `drop(base)` explicite efface la base **sous un démon
  encore vivant** (le serveur, l'écrivain et le relais tombent après) — avant, le `remove_dir_all`
  échouait sur la base-fichier, maintenant il réussit et le démon écrit son point de reprise dans un
  dossier effacé : un vert qui ment ; le témoin de fin ne regarde que son propre dossier (il prouve le
  `Drop`, pas « la suite ») ; `RAG3WEAVER_GARDER=0` garde aussi ; trois témoins fautifs de plus dans
  e2e_prise_atomique et un à vérifier dans e2e_rouvrir ; et le point du ticket qui compte pour la
  mémoire vive du poste, le cache par défaut de `Catalog` dans `/tmp/rag3weaver_cache`, en service
  aussi, n'est pas touché. hotfixs corrige dans son commit à part, prend les trois de prise_atomique,
  et **le ticket reste « en cours »**. Leçon : un lot de l'équipier se relit comme un lot de n'importe
  qui ; les verts du superviseur ne remplacent pas la relecture.

## Ce qu'il faut en retenir (pour Lucie, le matin)

**Trois lots réels sont sortis de la nuit**, écrits par l'équipier Gemini sous supervision, relus
par hotfixs : les journaux d'annulation (**sur master, b860ea2dd**), la racine disparue (**sur
master, 1b9983b59 + 1de9c908c**, complété par hotfixs), et les bases laissées dans /tmp
(`agent/tests-bases-dans-tmp`, 1b4c73228 + 674b41f8d ; hotfixs ferme le ticket et le passe
après la fenêtre du matin). Ce qui a changé dans l'hôte pendant la nuit, grâce aux essais : jamais
muet (3 h 43), les commandes par la porte (3 h 43), et sur master pour la pose de 8 h 30 : l'état
d'attente, le bac à sable qui laisse tourner cargo, le fil par pas et l'arrêt d'inactivité qui
compte l'équipier (ces deux derniers poussés après la fenêtre).

**Ce qui marche** : le plan en items, `step_done` à chaque pas, une analyse juste du code, un témoin
rouge d'abord quand on le lui impose, le recadrage par une `request` dans la boîte (il l'applique
au pas suivant, sans discuter), la correction à partir d'une sortie de compilateur ou de test rendue
telle quelle.

**Ce qui manque, par ordre d'importance** (tout est chez ED, ticket des essais, après le lot 2) :
1. **`run_command`** : sans lui, pas de test joué, pas de commit, le superviseur fait les deux à la
   main — c'est le lot 2 d'ED (les commandes par la porte déclarée), en passe cette nuit.
2. **Attendre n'est pas rater** : les pas d'attente comptent comme ratés (3 → la tâche s'arrête)
   ou comme des pas (40 → la tâche s'arrête), et il répète la même ligne à chaque tour pendant
   l'attente, un appel Gemini par ligne. Il faut un état d'attente : question posée, pas suspendu,
   aucun appel jusqu'à la réponse.
3. **Il ne lit pas les `answer`** (réponses `reply_to` à ses questions) ; il lit les `request`.
   ED l'explique par une course corrigée dans le lot 2 : la fin d'un lancement marquait toute la
   boîte comme lue, une ligne arrivée dans la même milliseconde s'effaçait ; le curseur reste
   maintenant où la boucle l'a laissé.
4. **Les 429 Vertex** ne frappent que lui (01:48, 02:11, 02:39, 02:56…), même au fil court ;
   e2e_juge et le juge des bilans passent à la même minute sur le même projet. À mesurer (taille et
   cadence de ses requêtes) avant de corriger.
5. **Le juge note chaque pas comme la tâche entière** (2/10 au pas « lire »), dit faux sur les
   outils déclarés, croit un témoin non joué.
6. Deux crochets d'après-outil d'`edit_file` en erreur (gabarit `motif-ailleurs` relatif au
   répertoire courant du processus ; un appel sans `old`).
7. Petites choses : « constate l'échec » dit sans avoir joué (essai 3), la clôture du plan malgré
   « attends ma confirmation » (essai 3, corrigé par la consigne à l'essai 4), les dates (7 au lieu
   du 8), l'attribution du ticket à la mauvaise session.

**Pour rejouer soi-même** : `~/.config/rag3weaver/code-essais-c7` (déclarations, git), backend
`e4d8635ecfe7`, `POST /ide/teammate/mailbox` avec `kind: "request"`, la boîte dans
`~/.local/share/rag3weaver/taches/e4d8635ecfe7/<tâche>/mailbox.jsonl`. Mes consignes et mes
réponses sont dans mon dossier de travail de session (`essais-equipier/*.json`). Le worktree
`~/git_workspaces/rag3db-essais-c7`, l'arbre de déclarations et les tâches s'effacent quand tu
auras lu ; les branches `agent/*` restent sur origin jusqu'à ton mot.

