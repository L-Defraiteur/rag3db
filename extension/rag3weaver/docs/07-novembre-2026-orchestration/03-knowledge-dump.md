# Knowledge dump de l'orchestration, 7 novembre 2026

De mémoire, pour la session qui reprendra. Le dump du 25 octobre
(docs/25-octobre-2026-orchestration/02-knowledge-dump.md) reste vrai pour tout
ce qui n'est pas redit ici ; celui-ci dit ce qui a changé depuis et ce qu'il
faut savoir pour orchestrer lundi.

## Les objets

- **rag3db** : fork C++ de Kuzu, dans ce dépôt (src/, build/). MVCC réécrit
  (plusieurs écrivains, V2-1b/1c : lecteurs non figés par le point de reprise,
  V2-2a+2b : générations gardées pour les lecteurs ouverts, retrait des
  générations en lot). Formats de stockage : 41, 42 (en service), 43 (CREATE
  d'index concurrent, colonne `state` dans SHOW_INDEXES ; sur master, pas encore
  en lib courante), 44 prévu (V2-2c : écrivains pendant le point de reprise).
  Correctif du 7 nov. : un bloc entièrement annulé garde ses versions.
  Correctifs en passe : PROJECT_GRAPH_* sous READ ONLY et DROP_PROJECTED_GRAPH
  qui honore `skip_if_not_exists` (sans eux, la recherche dense filtrée sous
  l'instantané L6 condamne la transaction ; le produit la sort de l'instantané
  en attendant, 25d9bb4aa).
- **rag3weaver** (extension/rag3weaver) : le crate Rust, hôte des backends par
  socket systemd 7970 (jeton stable, env ~/.config/rag3weaver/rag3weaver-daemon.env
  avec GOOGLE_CLOUD_PROJECT et GOOGLE_APPLICATION_CREDENTIALS depuis le 25),
  manifestes déclaratifs, graphes Mermaid, IR du Plan rendu en Cypher ou SQL,
  journal des runs (~/.local/share/rag3weaver/runs, zstd, 5 Gio / 90 jours,
  runs épinglés, rendu entier), juge de session + bilan des outils (fiches
  `outil:<nom>`), équipier (mini-plan, step_done, mailbox, suggest), chat qui
  code dans une tâche, IDE en React (vues 1-5 sur master, 6/7/3b en file ; shadcn
  + lucide, bundle bâti par scripts/ide_bundle.sh et commité sous templates/ide/app/,
  vérifié par blake3 et --check), `connect_llm` qui rend `Arc<dyn Llm>` pour
  Gemini (Vertex), OpenAI-compatible (serveurs locaux) et Anthropic (depuis le 7),
  `models.llm` déclaré au manifeste et servi aux graphes, ForEachNode et
  sous-graphes cités par fichier (`@graphs/x.mmd`), whois (Plan du gabarit memory),
  hooks Claude Code (`rag3weaver-backend hook`, module claude_code_hook,
  templates/hooks/*.md copiés sous ~/.config/rag3weaver/hooks/, hooks.json,
  journal ~/.local/state/rag3weaver/hooks.log, route POST /journal/run).
- **La mémoire longue de Lucie** : backend memoire-longue de l'hôte 7970, fichiers
  sous ~/memoire-longue/ (manifeste.yaml, fiches/, sources/, genres/), config
  ~/.config/rag3weaver/memoire-longue/, instantané migration-fiche-25-oct. Contenu
  réel au 7 nov. : deux fiches (« régime GPU », « hooks-essai ») ; genres : doc,
  fichier, agent, alias, handle, same_as. À peupler par l'archiviste (M4 :
  pas 1 models.llm, pas 2 SplitSections sur master ; pas 3 `archive` en essai ;
  tentative 5 sur Gemini à venir) avec procédures, page 08, fiches de session,
  visions, nouveautés, et les ~160 fiches de la mémoire Claude Code de ce projet.
- **La base de code de Lucie** : code-rag3db-depot (~/.local/share/rag3weaver/
  code-rag3db-depot.rag3db + .fts), rebâtie entière le 25 à 23 h 55 après la
  corruption de LOCKS/QUERIES (tables nées tardives) ; 0 écart après douze jours
  (relevé ancré, hotfixs) ; instantané locks-25-oct de l'ancienne ; manifeste
  ~/.config/rag3weaver/code-rag3db/… avec `outline` ajouté le 25.
- **Le magasin de libs** ~/.cache/rag3db-libs/<rag3db-build-<commit moteur>…>/,
  `courant` → 04063ee06 aux deux postes (3724accba a été blanchie mais jamais
  reposée) ; le magasin nomme par le dernier commit **moteur**. La lib unique de
  la bascule 43 : master avec V2-1c + V2-2a+2b + (A) + bloc annulé + projection +
  retrait ; procédure docs/procedures/bascule-de-lib.md (arbre), essai IDE de G
  avant la pose, instantanés (vrai changement de format), courant, hôtes.
- **codeparsers** : sous-module extension/rag3weaver/codeparsers ; INCLUDES en
  forme exacte sur master (9fd37ab1c), oracle clangd corrigé (progrès déclaré,
  fichier ouvert avant d'attendre), verdicts Rust/Python/TS retirés jusqu'au
  rejugement, série 7e/9e et rejugements à luciepc à faire.
- **lucivy** : 4.3.1 publiée le 25 (crates.io, npm, PyPI) ; rag3weaver passe au
  registre par lucivy-publiee-2 (vert, en file) ; tickets ouverts dans lucivy :
  apostrophe typographique, clés de requête ignorées.
- **Gemini** : gemini-3.8-flash par défaut, Vertex projet lr-hub-472010, région
  `global` par défaut du code ; `gemini-4-argon` GA en global (countTokens 200),
  pas en us-central1 pour ce projet ; essai borné en cours (archi), facturation
  à lire par Lucie le 8.

## Les postes

- **Portable** (lucie-tablette) : 1 To, 357 Gio libres après le ménage ; en mode
  éco la nuit (aucune mesure) ; le llama-server de Lucie (port 8080, modèle
  122B) tient ~77 Gio de GTT amdgpu quand il tourne : rien de lourd ici alors ;
  `poste lourd` à 1 place ; tunnels 7979-7981 vers luciepc par
  ~/.config/systemd/user/rag3weaver-tunnels.service.
- **luciepc** (ssh, fish : passer par `ssh luciepc bash -s <<'EOF'`) : 24 cœurs,
  zram ; ~600 Gio libres ; `poste lourd` à 4 places (fichier lourds-max relu à
  chaque tour, POSTE_LOURDS_MAX l'emporte), la mesure exclusive et prioritaire ;
  services d'embarquement en unités systemd rag3weaver-embeddings@{granite-278m
  (7878, 768), granite-107m (7880, 384), bge-m3 (7879, 1024, reranker, OCR,
  /embed_sparse)} ; PostgreSQL de test 5433 ; worktrees frères détachés sous
  ~/git_workspaces/rag3db-<nom> ; les données Magic (hors-arbre-18-oct,
  archive-mtga-18-oct) restent telles quelles ; un `target/` de 208 Gio d'août
  dans l'arbre principal, à la décision de Lucie.

## Les sessions (clé stable → nom du 7 nov.)

c7 → l'orchestration (moi ; Fable) ; 73 → « arbre principal » (exécuteur,
L4-L6, lib unique, ForEach, whois, plafond L6) ; I → « everything declarative »
(IDE React, tâches, équipier, juge, journal des runs, chat) ; 96 → « mémoire »
(gabarit memory, archiviste, identité, roadmaps, migration) ; 97 → « recherche »
(recherche, surlignages, creux, vue « mes recherches », models.sparse) ; G →
« optimiseur » (bâtis, poses, hôtes, poste, paquet npm, lucivy-publiee) ;
cœur → « coeur c++ » (transactions, MVCC, bloc annulé, projection, retrait,
liste de la semaine) ; cc → « coeur c++/hotfixs/tickets » ((A) CREATE concurrent
format 43, (B) demi-arêtes, relevé ancré, tickets, liste de la semaine) ;
codeparsers → « codeparsers » ; banc → « banc » (vingt clients, pente, hors
graphe, banc-temoins-v22) ; embarquement → « embarquement » ; lucivy →
« lucivy » ; archi → « rag3weaver archi » (chantier E anthropic-llm, hooks
Claude Code, essai Argon, pont superviseur ↔ équipiers) ; hooks-essai → la
session Claude Code d'essai de Lucie (clé `hooks-essai`, worktree
~/git_workspaces/rag3db-hooks-essai, lot chercher-sans-dossier). Toutes sur
Opus 5.5 sauf moi.

## Les règles qui tiennent (en plus de celles du 25)

- Lucie : identité gmail, pas d'attribution IA, master fast-forward sous `if
  grep -qx VERT`, jamais de force, un nom neuf après rebase, chaque commit
  poussé dans la minute, rien vers les amonts, rien dehors sans son mot, `.vault`
  jamais affiché, pas de `pgrep -f`/kill par nom, luciepc sans sudo libre.
- **docs/procedures/push-d-un-lot.md** fait foi : un worktree par lot, un
  target par session, lib épinglée + MOTEUR_ANCIEN=1, check_features, lib
  entière sous le jeu large (rag3db-native, burn-embedder, burn-ocr, code,
  daemon, postgres, openai-llm, anthropic-llm), étape navigateur (paquets React
  par scripts/paquets_react.sh, RAG3WEAVER_PAQUETS_REACT), les e2e du changement
  sous le jeu large, **une suite non jouée est ROUGE**, rebase lu à trois points
  et rejoué si du code est arrivé, relire master juste avant le push, garde du
  conflit, trois contrôles, le push efface worktree et target, un worktree
  atterri ou à >200 commits de retard se refait.
- Un e2e neuf qui exige le moteur neuf porte sa garde de capacité (« non jugé »,
  jamais #[ignore]) ; un témoin de service passe par le chemin de l'hôte ; un
  rouge connu se rejoue seul une fois et se nomme (ticket « tests sous
  charge ») ; un défaut connu est un ticket ; un seuil ne bouge pas sans mesure.
- Permissions : la suppression de branches distantes et (pour G) le retrait de
  worktrees sont refusés par les outils ; c'est à Lucie ; on ne contourne pas.
- Fenêtres de push : accordées par moi quand un lot perd des courses ; sinon
  l'ordre par hash, chacun rebase sur le précédent.
- Les briefs aux sessions Opus nomment fichiers, commandes et témoins.

## Les visions (toutes sur master, extension/rag3weaver/visions/)

00-vision-generale.md (§7 : le juge sur les outils déclarés, la voix sur le
chat) ; 2026-11-06 « le dépôt qui sait pourquoi » (la thèse : la valeur
augmente quand les agents deviennent interchangeables ; sans modèle ; sans
repartir de zéro ; find_tasks) ; 2026-11-06 « un index par dépôt, des vues par
worktree » (versions par empreinte au grain du scope, vue = chemin → version,
apports `diff`, diffs sémantiques, worktree jetable et mini-machines à parts
égales, les six difficultés dont la cohérence temporelle, successor_of, résumés
de scopes, RagForge). Plan du week-end : docs/06-novembre-2026-orchestration/01.

## Ce qu'il faut faire lundi, dans l'ordre

1. Finir la file (vues, lucivy-publiee, chercher-sans-dossier, M4 pas 3,
   models.sparse), puis la lib unique et la bascule 43, puis le relevé ancré de la
   base de code, puis effacer les instantanés 42 (bascule-04063ee06,
   bascule-66994dd43) et locks-25-oct quand (B) est fermé.
2. Lire la facturation Argon avec Lucie ; décider les quatre mots en attente.
3. Les six lots de l'équipier agent de code (everything declarative), le pont
   superviseur ↔ équipiers (archi), la vue « mes recherches » (recherche), le
   rangement de la mémoire (96), la semaine « moteur ennuyeux » (cœur, hotfixs).
4. Le prototype vertical des vues par scope (recherche, everything declarative,
   arbre) quand la file est vide.

## Complément du soir (22 h 40), ce qui a changé depuis l'après-midi

- **Moteur** : `courant` = rag3db-build-17a094d12… (format 43) depuis 19 h 57
  aux deux postes, instantanés avant-43 en retour ; master porte en plus
  c4f8610b9 (COPY de relations), e86d2f688 (projection en lecture seule) et
  b3bc6e479 (retrait des générations) : lib ordinaire bâtie le soir par
  l'arbre, fenêtre G à 8 h 30 le 8 nov. avec le redémarrage des granite.
  Passes : 17a094d12 + MOTEUR_ANCIEN=1 jusqu'à cette lib, puis la nouvelle,
  MOTEUR_ANCIEN vide. Figement des verrous CSR corrigé (e526cc4b6) ;
  instruction en échec (17a094d12) ; décalages provisoires d'un COPY de
  relations (c4f8610b9, inatteignable dans l'hôte : une connexion écrivaine par
  backend ; second processus refusé par le moteur, témoin de recherche) ;
  tickets LOCKS fermé (094aee5f8), validation à moitié appliquée et annulation
  sous tampon plein fermés non reproduits avec témoins en garde.
- **Hooks Claude Code** : lots 3 bis, 3 ter (a) (requête extraite, budget 1 s,
  déjà-vu, journal), (b) (signaux lus par ligne, le plein texte fait le
  candidat, dense seul non jugé, un rendu par tour, juge Flash différé sous
  `recall_judge`), (c) (plafond horaire, jetons du juge au journal, requête
  nettoyée) posés chez Lucie ; `--rejouer --session` ; premier bloc juste à
  18 h 48 ; le juge n'a fait aucun appel. Cause du rappel muet : `chunk.score`
  lu au lieu de `signals`. Clé `recall_judge: false` pour couper le juge.
- **IDE / chat** (hooks-essai) : parties dans l'ordre d'arrivée, heure à l'appel,
  résultat déplié, scroll, colonne fluide, pleine hauteur, barre « Dépôts /
  Mémoire / Projets » (kind par gabarit, repli par l'espace de travail), récap
  d'usage et fiches écrites par conversation, journal de l'hôte horodaté et
  attribué, flux de l'IDE qui ne perd plus de trame (`Flux::publier`),
  agent_ports sans horloge, `/call` à délai (`deadline_ms`, en file).
- **Équipier** : boîte adressée (id, to, reply_to, MailboxLine sur /ide/flux,
  pending/seen/list), étanchéité (base fermée → mode fichiers, pas raté →
  reprise, `teammate.max_failed_steps`), reprise après la mort de l'hôte (en
  file), `role` d'outil et de réaction, `call_models` en politique, le pas 1
  qui servait enfin (ec9a333e3), index des tickets généré (en-tête par ticket,
  `check_tickets.py --index/--lot`).
- **Pont et canal** (archi, en file, hooks-pont-7) : `mail send|reply|pending|
  seen`, fils Codex (`codex queue --thread <uuid>`, index ~/.codex/
  session_index.jsonl), `rag3weaver-backend canal` (serveur MCP `claude/channel`,
  flag `--dangerously-load-development-channels server:rag3weaver-canal`, pas
  ce soir), règle de la boîte (seule une ligne adressée lance l'équipier).
  Codex 0.167 installé chez Lucie ; essai Claude → Codex réussi à 16 h 53.
- **Terminal** (tty) : tty-1 (client SSE avec reprise), tty-2 (vue inline
  ratatui), tty-3 (Daemon Code Summoning en cellules, --depot/--tache) sur
  master ; thème local au crate en attendant le mot de Lucie.
- **Recherche** : creux servi à tout lecteur, filtre du creux juste, requête
  longue bornée (L = 100, K = 64, mesurés), second-ecrivain, surlignage par les
  spans du service (139 ms contre 429), `models.sparse` posé chez Lucie (hôte
  sans burn → spans) ; lucivy 4.3.2 (PR #21 : passage exact long, flou long
  15,8 s → 96 ms, apostrophe, clés nommées) au mot de Lucie, un seul tag.
- **Règles neuves** : le verrou `poste push` ne couvre que le dernier pas ; une
  suite non jouée ou sautée est rouge ; un contrôle qui ne juge rien est rouge ;
  luciepc à 6 places lourdes, la mesure exclusive au mot de c7 ; « ne pas
  s'empêcher de progresser » ; vocabulaire git à l'écran (dépôts, worktrees,
  branches, PR ; projet rag3weaver pour le déclaratif pur) ; `kind` par
  gabarit ; ligne disque à chaque pose (portable 352 Gio, luciepc 580).
- **Visions ajoutées** : session multi-dépôts (page 02 d'ED), « l'atelier ouvre
  les dépôts » (page 03, accord de Lucie attendu), gabarits de mémoire épinglés
  avec `templates upgrade` (M8), géo plus tard (temps 1D, canevas, géographie,
  ville du code).
- **En attente de Lucie** : confirmation lucivy 4.3.2 ; thème partagé
  daemon.json ; page 03 ; effacement des instantanés et branches distantes
  (refusés aux sessions par permission) ; `bilans.memoire` sur son manifeste
  de code ; facturation Vertex le 8.

## Complément de la nuit du 7 au 8 (05 h 30)

Ce qu'on sait maintenant et qu'on ne savait pas hier soir.

- **Le figement des passes n'était ni un commit ni un test** : un client sans délai (relecteur
  /rerank, OCR /ocr) devant un service d'embarquement en file, sur un runtime tokio du crate à deux
  fils de travail où les réactions s'exécutent sans `block_in_place` — deux réactions qui attendent
  tiennent les deux fils, et `stop_within` comptait son délai par un minuteur de ce même runtime,
  donc ne sonnait plus. Reproduit à volonté par un faux service qui accepte et ne répond jamais.
  Depuis d8aeffea7 : le délai de l'embarqueur (`RAG3WEAVER_EMBED_TIMEOUT_SECS`, 60 s, valeur à
  mesurer) vaut pour les trois clients, `stop_within` compte hors du runtime, et la lib entière se
  joue avec ou sans service. Le cas réel de la nuit : la charge de luciepc entre 0 h 45 et 1 h 46.
- **Le quota Vertex de l'équipier** : le fil entier renvoyé à chaque tour (5 275 jetons au pas 1,
  122 242 au pas 10, 950 000 en trois minutes au pas 5), plus les lignes d'attente en boucle, un
  appel par ligne. e2e_juge et le juge des bilans passaient à la même minute sur le même projet.
  Correctifs : l'état d'attente (b0301a0e3), le fil par pas (9fef96de9, en attente de la fenêtre).
- **Le bac à sable des commandes d'une tâche** est Landlock : /usr, /lib, /lib64, /bin, /sbin, /etc
  en lecture, la racine du workspace en écriture, réseau fermé. Déclaratif : `workspace.sandbox`
  (`extra_read`, `extra_write`, `network`, et depuis 927ae9680 `env` avec `{task_dir}` et
  `{repo_target}`). Pour cargo : la chaîne Rust en lecture, `CARGO_TARGET_DIR={repo_target}` (un
  target par dépôt sous verrou, 32 Go, `taches purge`), `TMPDIR={task_dir}/tmp`, `/dev/null`
  inscriptible, `CARGO_NET_OFFLINE`. Un arbre inscrit avant 927ae9680 garde son manifeste : rejouer
  `code-tree add` ou copier le bloc.
- **Une tâche de l'équipier** : `request` ouvre (la réponse porte `task: {branche, depart}`) et
  relance une tâche `ended` ; `stop` arrête ; `answer {approved}` répond à une `approval` ; les
  déclarations sont celles du HEAD du dépôt d'arbre ; l'hôte s'arrêtait d'inactivité malgré une
  tâche en cours (767dcb036 corrige, en attente). La boîte est le seul canal : tout ce qui n'y est
  pas est perdu.
- **Le juge des bilans** note chaque pas comme la tâche entière (2/10 au pas « lire ») ; ses fiches
  « bilan » remontent au rappel des hooks sur des mots courants (ticket) ; l'empreinte du déjà-vu
  peut changer d'un binaire à l'autre (`DefaultHasher`, ticket, blake3 proposé).
- **L'hôte de Lucie ne prend un lot qu'à une pose de G** (binaire installé) : rien ne se recharge
  tout seul ; une pose du binaire seul au portable prend une minute (3 h 43 cette nuit).
- **Tout cargo hors `run_e2e.sh`** passe par `. scripts/lib_du_magasin.sh && exporter_la_lib`,
  sinon le binaire lie une autre lib que `RAG3DB_LIB` (vécu : moteur 7cd11e5fa, extension 17a094d12).
- **Un lot écrit par l'équipier se relit comme un lot de n'importe qui** : les verts du superviseur
  ne voient pas un `drop` qui efface une base sous un démon vivant (hotfixs l'a vu).
