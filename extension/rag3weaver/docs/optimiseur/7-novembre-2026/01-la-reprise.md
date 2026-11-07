# La reprise du 7 novembre 2026 (G)

Le plan est celui du week-end (`docs/06-novembre-2026-orchestration/01-plan-du-week-end.md`), et les
consignes viennent de c7 (rag3db-74).

## Le ménage (avant tout nouveau lot)

- **Caches effacés** : ici, target-G (15 Gio), paquet-npm-garde (1,4), demon (2,0), temoin-ide (1,6),
  hote-neuf et deux petits dossiers, soit 347 → 359 Gio libres. À luciepc, target-G passe de 63 à 22 Gio
  (565 → 605 Gio libres) ; je garde hote-lucie, hote-lucie-tests et controle.
- **Worktrees et branches distantes** : les verdicts sont rendus. Tout est absorbé par master, sauf
  lucivy-publiee-1 (poussée, pas fusionnée). L'effacement est **refusé par ma permission** (« Git
  Destructive ») ; il attend Lucie (fiche pas-de-suppression-de-branche-distante). Ici : rag3db-paquet-npm,
  rag3db-paquet2, rag3db-docs-G, rag3db-hote-socket, rag3db-poste-G. À luciepc : rag3db-hote,
  rag3db-hote-ab, rag3db-hote-socket, rag3db-hote-socket-2. Branches : paquet-npm, paquet-npm-2,
  controle-de-bati(-2), etape-navigateur-1/2, portage-b1-2, embarqueur-absent(-portes), poste-n-detenteurs.
  rag3db-service (luciepc, 5,5 Gio) attend aussi Lucie.
- **Instantanés** (règle de c7 : on garde le dernier retour possible) : locks-25-oct part quand hotfixs
  ferme (B) ; bascule-04063ee06 et bascule-66994dd43 partent après la bascule 43 et son réveil vert ;
  migration-fiche-25-oct est à 96, gardé jusqu'au mot de Lucie sur ~/memoire-longue/.

## Les poses

- **14 h 49**, d42787227 (hooks, lots 1 et 2), binaire 21f7f58b, lib 04063ee06, aux deux postes. Preuve :
  un vrai SessionStart sur stdin contre 7970, code 0, bloc de 525 caractères, journal hooks.log.
- **15 h 42**, 8f60895b8 (hooks, lot 3), binaire 1d11e6ef. `hook --help` répond ; `--poser-les-gabarits`
  pose SessionStart.md, UserPromptSubmit.md et PreCompact.md dans ~/.config/rag3weaver/hooks/ ;
  `hook --rejouer <transcription de la session d'essai> SessionStart < /dev/null` rend en 0,3 s un bloc de
  993 caractères. Sans `< /dev/null`, `--rejouer` attend stdin depuis un terminal ; c'est corrigé par le
  lot 3 bis d'archi. Le `.claude/settings.local.json` de la session d'essai, c'est Lucie qui le repose.

## Le relevé des tables de liens

À 14 h 55, un instantané à chaud releve-7-nov (base 1,37 Go, .wal 5,2 Mo, .fts 1,1 Go ; taille et date
identiques avant et après la copie). Le verdict de hotfixs : **0 écart sur les 18 tables** après douze
jours de montre (LOCKS 512, QUERIES 22 023). Le défaut du 25 octobre venait de la naissance tardive des
deux tables, pas de l'écriture courante. Instantané effacé ensuite.

## `poste lourd` : N places (e006f4cff)

Au plus N lourds tiennent la place en même temps ; les suivants attendent dans l'ordre d'arrivée (un billet
`<ns>-<pid>` par lourd, dans `file/`). La mesure garde sa priorité exclusive, et `push --lourd` compte
parmi les N. N se lit dans `~/.cache/rag3weaver-build/lourds-max` à chaque tour. Témoin test_poste.sh §7 :
rouge sur master, 27 OK ici.

| heure | poste | lourds-max | pourquoi |
|---|---|---|---|
| 15 h 05 | luciepc | 2 | pose du lot (sept passes ensemble, la lib entière de codeparsers à 235 s au lieu de 40) |
| 15 h 05 | ici | 1 | pose du lot |
| 15 h 35 m 06 s | luciepc | **4** | c7 : à deux places, luciepc tournait à un tiers (charge 3,4 sur 24 cœurs). Si « pas indexé en 60 s » revient dans l'heure, on redescend à 3 |

## À suivre

- lucivy-publiee-1 : rebase, lib entière, push sous if VERT à mon tour (après les vues d'everything
  declarative, hors fenêtre du cœur de 16 h 10 à 16 h 40).
- La bascule 43 sur la lib unique, après le correctif du cœur (lecture sale sous V2) : instantané
  avant-43 pour hotfixs, essai de l'IDE, pose aux deux postes.
