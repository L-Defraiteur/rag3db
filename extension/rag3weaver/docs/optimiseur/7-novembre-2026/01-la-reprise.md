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

- **16 h 31**, 120695b37 (hooks, lot 3 bis), binaire 24faff78. Le premier tour de hooks-essai porte le
  nouveau binaire : 17 h 05 m 55 s, `rag3weaver-backend@1794065083`, recall_fiches en 1 999 ms.
- **17 h 22**, 37904f103 : l'IDE React entier (vues 5, 6, 7 et 3b) et lucivy 4.3.1 publiée, binaire
  10878f01. Le témoin l_ide_d_un_vrai_hote est vert (63 s). Le parcours d'everything declarative est joué
  par CDP sur 7970 : arbre, fichier avec sa marge, symbole, graphe, DAG, réactions, propositions, tâches,
  bilans, édition ouverte puis fermée sans rien taper, ancienne page ; la console est sans erreur. Lucie
  entre par `rag3weaver-backend ide`.
- **La garde lucivy de l'essai** (essai-hote-nouveau.sh) : depuis 37904f103, elle exige `lucivy-core-4.3.1`
  du registre et aucune révision git dans le binaire. Elle rougissait parce qu'elle cherchait encore a925d94.

## lucivy 4.3.1 publiée sur master (37904f103)

Rebasée deux fois. L'index des tickets était à reprendre sur celui de master : les trois tickets lucivy
passent en fermés. Elle est poussée sous les noms lucivy-publiee-2 puis lucivy-publiee-3. La passe entière
de luciepc tourne sur la tête exacte, lib 04063ee06 épinglée et MOTEUR_ANCIEN=1 :
- check_features 0 ;
- lib entière 1954 passed ;
- navigateur 12 ;
- e2e_lectures 2 + 19.

La première passe de la journée avait rougi sur la garde du target (controle bâti par un autre arbre) ; elle
a été rejouée après RAG3WEAVER_TARGET_REPRENDRE=1.

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
| 21 h 40 (c7) | luciepc | **6** | Lucie : « quand personne n'a besoin de bancs, on peut abuser de luciepc, dans une mesure relative ». Charge 10-12 sur 24 cœurs avec 2 lourds. Retour à 4 si la charge dépasse ~20 ou si une mesure est annoncée ; la mesure ne se prend qu'au mot « fenêtre » de c7 |
| 15 h 35 m 06 s | luciepc | **4** | c7 : à deux places, luciepc tournait à un tiers (charge 3,4 sur 24 cœurs). Si « pas indexé en 60 s » revient dans l'heure, on redescend à 3 |

## Gemini 4 Argon sous lr-hub-472010 (vers 16 h 15)

Sans aucun appel de génération : le catalogue `publishers/google/models` de Vertex (jeton du compte de
service de `.vault`) liste `gemini-4-argon` en GA en global et en us-central1, et pas en europe-west1. Le
seul Gemini 4 de la liste est plus récent que gemini-3.8-flash. Un `countTokens` (gratuit) donne :

- **global : 200**, 7 jetons. Le projet l'atteint.
- **us-central1 : 404**, « not found or your project does not have access to it ». Le catalogue le liste,
  mais le projet ne l'a pas dans cette région.

Nom à déclarer : `google/gemini-4-argon` (interface OpenAI de Vertex), région **global**. C'est le défaut du
code quand un manifeste ne pose pas `location`, et les manifestes de Lucie n'en posent pas. Le manifeste
d'essai de l'équipier déclarera sa région à part. Le tarif et les crédits du programme startup ne se lisent
pas par l'API : c'est la console de facturation de Lucie.

## La bascule 43 (19 h 21 – 19 h 57)

Lib unique de l'arbre principal : `rag3db-build-17a094d12…`. Sa librag3db.so a la md5 16d68492…, la même aux
deux postes. Le verdict de l'étape 4 est vert (e2e_code 27, e2e_lectures 22, VERDICT_FORMAT=43). L'ABI
change par rapport à 04063ee06 : 3 symboles retirés, 17 ajoutés, et trois vtables (ListChunkData,
ColumnChunkData, InternalIDChunkData). Les hôtes sont donc rebâtis contre la lib neuve.

1. **Hôtes** : binaire dcda4247 (cf14a7771, qui porte le creux de la recherche), essai vert aux deux postes.
   L'accueil de l'IDE se rend sur 7991 contre la lib neuve.
2. **Arrêt à 19 h 21**, les binaires posés (les anciens sont en .avant-43), l'env et le filet sur 17a094d12.
3. **Instantanés avant-43**, rc 0, sans .wal : memoire-longue (30 Mo) et code-rag3db-depot (1,48 Go, plus
   un .fts de 1,86 Go) ici ; code-principal et code-lourd à luciepc.
4. **Contrôle des deux sens** (outil ancré 339cbb5dc, tiré d'origin/master) : **aucun écart sur les 4
   bases**, 2 bases contrôlées par poste.
5. **`courant` = 17a094d12** aux deux postes ; services et tunnels joignables ; **réveil à 19 h 57**.
6. Les 4 bases se rouvrent au format 43 sans refus. Une recherche dense bornée à Rust et à
   `extension/rag3weaver/src/` répond, recall_fiches aussi. Le témoin l_ide_d_un_vrai_hote est vert.

Deux réserves :
- **models.sparse** : la ligne est posée dans le backend.json de code de Lucie, à son mot (sauvegarde
  `.avant-sparse-7-nov`). Mais le surlignage creux se replie sur les mots : le binaire des hôtes est bâti
  sans la feature `burn-embedder`. C'est à décider : environ 88 Mo sans burn, environ 240 Mo avec.
- **Les anciens instantanés** (bascule-04063ee06, bascule-66994dd43, locks-25-oct, et les copies
  avant-43-controle) : l'effacement est refusé par ma permission. Rien n'est effacé ; c'est à Lucie.

## Les disques (relevé de 21 h 35)

Lucie (par c7) : « tout pour avancer vite, en surveillant bien les disques de temps en temps ». Seuils
d'alerte : luciepc sous 300 Gio libres, le portable sous 200 Gio.

| poste | libre | rag3weaver-build | instantanés |
|---|---|---|---|
| portable | 352 Gio (63 % pris) | 49 Gio | 15 Gio |
| luciepc | 585 Gio (85 % pris) | ~196 Gio | 14 Gio (avant-43, avant-43-controle, deux bascule-*) |

### Les targets de luciepc (`~/.cache/rag3weaver-build`)

La règle : un target par lot, effacé à la fin du lot. Chaque session efface les siens ; je n'efface que les
miens. Les tailles sont en Gio, la date est celle de la dernière écriture, et la session est déduite du nom.

| target | Gio | dernière écriture | session | remarque |
|---|---|---|---|---|
| target-codeparsers-clic | 24 | 7 nov. 18 h 28 | codeparsers | |
| target-coeur-c | 20 | 7 nov. 21 h 16 | cœur C++ | en cours |
| target-A-plan | 18 | 7 nov. 19 h 10 | arbre principal | |
| target-96-fiches4 | 13 | 7 nov. 21 h 19 | mémoire (96) | en cours |
| target-spans | 12 | 7 nov. 21 h 28 | embarquement | en cours |
| target-G | 11 | 7 nov. 21 h 11 | G (moi) | hote-lucie et hote-lucie-tests, servent aux poses |
| target-A-l4 | 10 | 19 oct. 02 h 12 | arbre principal | **vieux de 19 jours** |
| target-A-l5 | 9 | 19 oct. 02 h 54 | arbre principal | **vieux de 19 jours** |
| target-I-apres | 9 | 7 nov. 21 h 33 | everything declarative | en cours |
| target-hooks-essai | 7 | 7 nov. 21 h 33 | hooks-essai | en cours |
| target-A-l6 | 7 | 19 oct. 08 h 40 | arbre principal | **vieux de 19 jours** |
| target-97c | 7 | 7 nov. 20 h 18 | recherche | |
| target-A-l4b | 6 | 18 oct. 17 h 52 | arbre principal | **vieux de 20 jours** |
| target-97e | 6 | 7 nov. 21 h 33 | recherche | en cours |
| target-97d | 5 | 7 nov. 20 h 36 | recherche | |
| build-asan | 4 | 25 oct. 14 h 26 | banc (asan-commit.sh) | vieux de 13 jours |
| build-vingt | 1 | 7 nov. 20 h 18 | banc (scénario des vingt) | |

Effacé par moi à 21 h 35 : `target-G/controle` (14 Gio ; la passe de lucivy-publiee est sur master) et
`target-G-hote` (4 Gio, du 12 oct.). Les quatre `target-A-l*` d'octobre (32 Gio) et `build-asan` sont les
premiers candidats, pour leurs sessions.

## À suivre

- lucivy-publiee-1 : rebase, lib entière, push sous if VERT à mon tour (après les vues d'everything
  declarative, hors fenêtre du cœur de 16 h 10 à 16 h 40).
- La bascule 43 sur la lib unique, après le correctif du cœur (lecture sale sous V2) : instantané
  avant-43 pour hotfixs, essai de l'IDE, pose aux deux postes.
