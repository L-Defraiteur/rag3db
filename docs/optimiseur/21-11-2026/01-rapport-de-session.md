# G (optimiseur), 21 novembre 2026 — note de pause avant le redémarrage de la tablette (15:26:22)

## Où j'en suis

- **Pousseur** (luciepc, unité rag3weaver-pusher) : tourne ; repris à 15:10:16 après la boucle du confineur.
  `confiner-poste-classes` (60ff666eaa, check_features vert 8/8) était au rang 1, puis `arbre-non-git-folder`
  (remis en file après un rouge 143 qui n'était pas le sien).
- **Atterri aujourd'hui** (mes lots) : poste-load-sampler (relevé de charge, installé aux deux postes à 13:24),
  pusher-french-identifiers et suites, pusher-dequeue-during-pass, pass-skipped-tests-not-green, poste-gpu-class (14:20).
- **luciepc sert par code-rag3db-luciepc** : hôte sans sommeil (RAG3WEAVER_DAEMON_IDLE_MIN=0), index rag3db-master
  sur ~/git_workspaces/index-master/rag3db (branche index-master), tunnel 7971 depuis la tablette
  (rag3weaver-tunnel-hote-luciepc.service, activé : revient seul au démarrage), ~/.claude.json pointé sur
  rag3db-master (copie avant : ~/.claude.json.avant-mcp-luciepc-20261121). Les sessions doivent relancer leur client
  MCP pour viser rag3db-master.

## Ce qui tourne à luciepc

- Le pousseur (unité) et ses passes.
- Un guet de G par ssh (il tombe avec la tablette, sans effet) qui attend l'atterrissage d'index-master-ff-2 pour
  installer index-master-ff.timer ; **à refaire à la main** si le lot atterrit pendant la coupure :
  `git show origin/master:tools/poste/index-master-ff.{service,timer}` vers ~/.config/systemd/user, daemon-reload,
  `systemctl --user enable --now index-master-ff.timer`, puis vérifier ~/.cache/rag3weaver-build/index-master-ff.log.
- Aucune passe de G hors du pousseur.

## Mes lots en file (têtes)

| Rang (15:1x) | Lot | Tête | Parent |
|---|---|---|---|
| 1 | confiner-poste-classes | 60ff666eaa | - |
| 4 | poste-gpu-english-names | c4a901ec62 | poste-gpu-class |
| 6 | index-master-ff-2 | a69365c99d | - |
| 7 | poste-sccache-server-outside | 9934173151 | poste-gpu-english-names |

## Prochaines actions

1. À l'atterrissage de confiner-poste-classes : vérifier qu'une passe Rust joue sa validation une fois (plus de
   « un ancêtre tient » en boucle).
2. À l'atterrissage de poste-gpu-class (fait) et de ses enfants : poser à luciepc `gpu-card` = card2,
   `gpu-reserve-gib` = 19, `gpu-weekdays` ; rien à la tablette ; dire `poste gpu` à banc.
3. À l'atterrissage d'index-master-ff-2 : installer le timer, puis la preuve « un nom arrivé sur master trouvé par
   code-rag3db-luciepc une minute après », et le dire à c7.
4. Ticket du disque (docs/optimiseur/tickets/2026-11-21-un-disque-presque-plein-ne-retient-aucune-passe.md) : le
   correctif dans poste et le relevé.
5. Demain : la page de charge (poste-load.tsv), places pleines + attente + CPU < 70 %, la mémoire de luciepc.

## Après la reprise (22 nov., 00 h 3x)

- Les outils de pose d'hôte, perdus avec le scratchpad au redémarrage, sont rapatriés et durables à la tablette :
  `~/.cache/rag3weaver-build/notes-G/outils/` (preparer-binaire.sh réécrit, essai-hote-nouveau.sh, pose-hote.sh,
  bascule-pose.sh, bascule-reveil.sh, hostcall.py, bati-lucie-luciepc.sh).
- Lib 2c4bddcd3c (fsm-finalize-lock de hotfixs) : bâtie, verdict e2e_code et e2e_lectures VERTS, e2e_demon_hote rouge à
  l'identique sous 424ce93e56 (lucivy 4.4, commit_tagged : lot (A) de lucivy) ; courant aux deux postes ;
  pin-lib-2c4bddcd3c au rang 1.
- Hôte du matin : bâti vers 5 h 15 sur master (dangling-links compris s'il a atterri), pose vers 6 h aux deux postes.
