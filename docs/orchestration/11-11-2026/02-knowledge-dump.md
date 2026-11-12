# Orchestration — knowledge dump du 11 novembre 2026

Tout ce que l'orchestrateur des sessions sait et qu'un agent qui reprend doit savoir. Les règles ici
ont chacune coûté un rouge, un push perdu ou une soirée ; aucune n'est décorative.

## Les dépôts et les postes

- **rag3db** (`~/git_workspaces/rag3db`, GitHub `L-Defraiteur/rag3db`, branche `master`) : le moteur C++
  (fork de Kuzu, `src/`, `test/`, bâti par CMake) et, sous `extension/rag3weaver/`, le crate Rust
  rag3weaver (hôte, manifestes déclaratifs, IDE React dans `ui/ide`, atelier, mémoire longue, équipier,
  juge, hooks Claude Code). Sous-module `extension/rag3weaver/codeparsers` (grammaires, IR). lucivy
  est un dépôt à part, publié (4.3.1).
- **L'arbre de Lucie** (`~/git_workspaces/rag3db`) est intouchable : aucune session n'y commite ni n'y
  bascule une branche. Chaque lot se fait dans un worktree (`git worktree add`), avec son target cargo
  sous `~/.cache/rag3weaver-build/<nom>` (plafond 32 Go), effacés par le script de push après les trois
  contrôles. Exception : les docs de l'orchestration se commitent par chemins (`git commit -- <chemins>`)
  depuis l'arbre, docs seules.
- **Deux postes** : le portable (où Lucie travaille, pas de service d'embarquement lourd : « le portable
  devient lourd dingue ») et **luciepc** (ssh `lucied@luciepc`, shell fish : passer par `bash -c` ou
  `bash -s < script`), qui porte les bâtis, les passes, les services d'embarquement et les bancs. Lucie
  y joue parfois (seat1, card0) : un banc plus lent n'est pas un défaut. Pas de sudo sur luciepc sans sa
  main ; les réglages sans sudo s'y font sans demander.
- **Le magasin de libs** (`~/.cache/rag3db-libs/`, les deux postes) : chaque lib du moteur a un
  identifiant complet `rag3db-build-<hash>-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF` ;
  `courant` est un lien ; l'hôte et les binaires sont liés par RUNPATH. Toute passe épingle
  `RAG3DB_LIB=<identifiant complet>`, jamais `courant` ; `exporter_la_lib` avant tout cargo. Quand
  master porte du C++ plus neuf que la lib, les e2e se refusent (« librag3db.so est plus vieux que le
  dernier commit de ses sources ») : il faut bâtir la lib suivante et la nommer à toutes les sessions.
- **L'hôte** rag3weaver : socket systemd `rag3weaver-daemon.socket` (127.0.0.1:7970) et service
  `rag3weaver-daemon.service` ; jeton dans `~/.local/share/rag3weaver/daemon.token` ; `/sante`,
  `/etat`, `/backends`, `/backend/open|close`, `/ide/`. Une pose du binaire = annonce, coupure de
  quelques minutes, réveil contrôlé (lib dans les maps, backends rouverts, IDE, outils de l'atelier),
  instantané `~/.local/share/rag3weaver/instantanes/avant-<hash>/` avec `RETOUR.sh`.

## Les sessions

Quatorze périmètres, chacun une session d'agent (noms dans `docs/README.md`). L'orchestrateur tient
**la file** : un lot atterrit à la fois ; « à toi » est le mot qui donne le tour de push ; une session
dit « poussé » avec le hash et les trois contrôles lus. Les docs seules se poussent sans tour. Une
session muette plus de 10 minutes sur un travail en cours se relance ; une session « waiting » attend
une approbation de Lucie, et rien ne la débloque sinon elle. Un refus de permission dans une session
n'est jamais contourné par une autre session (blanchiment interdit) ; il remonte à Lucie.

## La passe et le push (le détail dans `docs/procedures/push-d-un-lot.md`)

- **Rouge d'abord** : un témoin vu rouge pour la bonne raison, puis vert. Un témoin qui se saute n'est
  pas vert ; une suite non jouée est rouge ; un contrôle qui ne trouve rien à juger est rouge.
- **La passe** à luciepc sous `poste lourd` (`extension/rag3weaver/scripts/poste`) : `check_features.sh`,
  la lib entière sous le jeu large de features
  (`rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm`), l'étape
  navigateur (`RAG3WEAVER_PAQUETS_REACT`), les e2e du changement (`run_e2e.sh`, une suite par appel).
  Un e2e entier qui embarque en creux est un lourd GPU ; un banc à services ne fait jamais partie de
  l'e2e entier (il se joue seul avec sa voie). Plafond de lourds par poste : `~/.cache/rag3weaver-build/lourds-max`.
- **Le push** : rebase sur master du moment ; lire le diff du rebase ; si du Rust est arrivé sous
  `extension/rag3weaver` (hors .md, hors `src/` et `test/` du moteur), rejeu court (check_features, lib
  entière, e2e du changement) ; docs ou C++ seuls : rebase sans rejeu. Dernier pas sous `poste push`
  (trente secondes), push dans `if grep -qx VERT`, puis les trois contrôles : status propre, aucun
  rebase en cours, `origin/master` = HEAD. Le script efface worktree, branche locale et target.
- **Vert est aucun rouge** : un rouge d'environnement se rejoue vert seul et se nomme avec sa cause ;
  jamais un push avec un rouge ; un seuil ne se relâche pas sans mesure ; un défaut connu non corrigé
  est un ticket.
- **La CI** (GitHub Actions, `rag3weaver` et `CI Workflow`) : le run d'un commit se lit sur le run qui
  l'a joué ou le premier run plus récent fini ; un « cancelled » est un run remplacé.

## Les mesures et les ressources

- **Précision ultérieure de Lucie, 12 novembre 2026** : préserver le GPU de jeu
  seat1 de toute charge du banc ; les 40 KiB de contexte d’initialisation
  peuvent être tolérés s’ils restent insignifiants. La seule présence d’un FD
  card0 ne suffit plus à refuser. Poids, cache et calcul du banc restent sur
  card2 ; vérifier mémoire et activité attribuables au processus et au PCI,
  sans confondre charge du jeu et charge du banc. Le filtre ROCm n’évite pas
  toutes les ouvertures d’initialisation : les sources 7.2.4 expliquent le
  parcours des périphériques avant sélection des GPU visibles.
  Cette précision prévaut sur les exigences historiques de zéro accès ci-dessous.
  Zéro requête du banc le 12 novembre ; le warmup interne du serveur a eu lieu,
  sans attribution d’exécution par compteur PCI. Aucun retry autorisé par
  cette clarification ; recette de contrôle à qualifier avant nouvelle fenêtre.
  Sources et suite : [ticket banc k10](../../banc/tickets/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md).

- **Consigne actuelle de Lucie, 12 novembre 2026** : card0/seat1, PCI
  `0000:07:00.0`, ROCm GPU[1], est interdite aux sessions. Seule card2, PCI
  `0000:04:00.0`, ROCm GPU[0], peut être utilisée, avec
  `ROCR_VISIBLE_DEVICES=0` explicite et coordination selon la charge.
  Card1/PCI 00:02.0 est l’iGPU hors ROCm. Ne pas déduire l’identité PCI d’un
  numéro Vulkan : relire sysfs et les preuves du processus.
- Card2 porte 7878/7879/7880/7882. Avant un banc, lire la VRAM et les consommateurs ;
  ne pas ajouter une cinquième charge aux quatre services. Si nécessaire,
  coordonner la libération temporaire du privé 7882 avec son porteur et
  l’optimiseur, recette de retour prête avant arrêt ; aucune coupure d’autrui
  sans remise explicite. Un budget mémoire ou une confirmation antérieure
  ne remplace pas le relevé réel après libération. Les chronométries comparatives
  prennent `poste mesure` dans une fenêtre coordonnée, délai après acquisition.
- **Filtre ROCm insuffisant pour garantir l’exclusion de seat1** : le passage
  banc du 12 novembre a trouvé un contexte de 40 KiB sur PCI 07:00.0 malgré
  `ROCR_VISIBLE_DEVICES=0`, alors que poids/cache étaient sur PCI 04:00.0.
  Garde arrêtée avant inférence, zéro paire, aucun retry ; cela ne prouve
  pas un calcul du modèle sur card0. 7882 a été restauré et le verrou rendu.
  Garder l’exclusion stricte et qualifier une frontière d’accès aux périphériques
  avant nouveau chargement, sans relâcher la garde ni modifier globalement seat1.
  Suivi : [ticket banc k10](../../banc/tickets/2026-11-11-mesurer-le-juge-local-avec-dix-fiches.md).

- Un index de mesure (copie d'une base, index complet) tourne sur **son propre service d'embarquement**,
  jamais sur 7878 (les worktrees de Lucie y passent par le tunnel). Copier une base = base, wal,
  extensions, .fts et dossier ensemble, hôte fermé (ou l'instantané de G).
- Lire la carte : `/proc/<pid>/fdinfo` (part du moteur par service), `ss -tnp` (clients par port),
  `/sys/class/drm/card2/device/gpu_busy_percent`. Lire le disque : PSI io, `btrfs filesystem`,
  états D et wchan.
- Un script distant survit à son ssh ; `timeout` tue le parent et laisse l'enfant orphelin : un
  lanceur arrête tout son groupe (`e2e_entier.sh` le fait depuis le 9).
- `/tmp` est de la mémoire vive ; scratch sous `~/.cache/<session>-notes/`.
- On ne tue que ses propres pids ; `pgrep -f` se trouve lui-même ; `systemctl stop` d'une unité
  `systemd-run` ne suit pas ses enfants.

## Les décisions de Lucie qui tiennent (avec la date)

- 11 nov. : tout nouveau travail à faire reçoit un ticket, y compris un signalement entre
  sessions. Réutiliser l’existant ; transmettre son chemin et le porteur ; clôturer par
  déplacement vers `docs/tickets/closed/` avec état et preuve ou raison d’abandon. Le message
  seul ne porte pas l’action. La règle complète vit dans `docs/tickets/README.md`.
- 6 nov. : un index par dépôt, des **vues par worktree** (une vue est un filtre : worktree, branche,
  pyramide ; page `extension/rag3weaver/docs/08-novembre-2026-arbre/01-conception-des-vues-marche-1.md`).
- 7 nov. : fusionner dans master sans attendre ; ne pas s'empêcher de progresser ; vocabulaire git à
  l'écran, pas « backend » ; « dire à Lucie quand regarder ».
- 8 nov. : le README suit les nouveautés ; brancher un thème sans demander l'accord ; pas de
  barrière imaginée aux essais de modèles ; l'atelier a la ligne de commande et des outils de
  worktree tenus par le framework ; tout dynamique, sans recharger ; progressions par SSE, pas par
  polling, générique ; l'atelier « reste capable de tout faire », périmètre = dossier de lancement
  (`~/rw-code/` par défaut), sortie par déclaration d'un espace ou par la porte d'accord ; Landlock
  déclaré `home|atelier` (domaine tout son home possible, restreint à tester).
- 9 nov. (c7, à relire par elle) : dans un worktree que l'atelier a ouvert, il écrit sans question ;
  dans l'arbre principal ou un worktree tenu dans l'IDE, carte d'accord.
- Toujours : identité gmail seule ; aucune attribution ; rien vers les amonts ; tag lucivy `v*` à son
  mot ; fusions lucivy à son mot dans la session lucivy ; branches distantes jamais supprimées sans elle ;
  `.vault/` et `.env` jamais lus ; son sudo par sa main ; ses réglages Claude par sa main.

## Les chiffres qui comptent (9 novembre)

- Juge local (banc) : Qwen3-4B 0,76 / AUC 0,92 / 504 ms (forme courte 0,73 / 90 ms) ; Gemini 0,81–0,84
  à 4,4 s ; dense seul 0,41. Recommandation : rappel k=5 → Qwen3-4B en cache de préfixe → 1 ou 2 fiches.
- Rappel des hooks aujourd'hui : 92 % des rappels vides, 1 fiche pertinente sur 12 glissées, 19 fiches
  dont une de sujet : le contenu manque, pas le juge.
- Équipier Gemini : un appel à 1 004 s de reculs invisibles, tâche close sans livrable (tickets de 96 ;
  le recul est borné depuis a6637eeee : chat 300 s, pas 120 s, juge 600 s).
- Outils de code par les sessions : 0 appel sur 500 runs ; mémoire par les hooks : 461 appels. Page
  de hooks-essai : search_code en plein texte p50 258 ms, p95 532 ms.
- Deux sens : 0 écart sous 29a7bc52a par le chemin de l'hôte (658/658, 546/546, 20093/20093).

## Où sont les choses

- Rapports d'orchestration des jours d'avant : `extension/rag3weaver/docs/07-novembre-2026-orchestration/`
  (02-rapport-de-session.md, 03-knowledge-dump.md), `08-novembre-2026-orchestration/`,
  `docs/08-novembre-2026-orchestration/`.
- Visions : `docs/orchestration/visions/` (les feuilles de route de septembre déplacées ici, et le
  README qui pointe les pages de conception de novembre). Nouveautés : `docs/nouveautes.md`, résumé
  sous `docs/orchestration/news/`.
- Tickets : `docs/tickets/` (ouverts sans session), `docs/<session>/tickets/` (ouverts d'une session),
  `docs/tickets/closed/` ; index par `check_tickets.py --index`.
- Procédures : `docs/procedures/`. Journal des chantiers : `docs/journal-des-chantiers.md`.
