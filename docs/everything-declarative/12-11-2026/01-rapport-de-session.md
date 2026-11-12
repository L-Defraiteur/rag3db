# ED — accords de commande, 12 novembre 2026

## Résultat du lot

Le [ticket d'accord extérieur](../../tickets/closed/2026-11-11-un-accord-de-commande-hors-domaine-redemande-le-meme-accord.md)
est corrigé et sa fermeture accompagne le lot d'intégration ED/archi. Source code éprouvée :
`b42a4ada90ab7f2b80f1137e75692b3d00e26fcc`, rebasée sur master3b2eab7ec ; range-diff19/19
identiques et fichiers Rust possédés byte-identiques au prérebase. Les commits suivants de
préparation du push ne changent que Markdown. Aucun push master n'est annoncé par cette note.

Verdict composé vert : features5/5, native1609/0/15, large2176/0/65, navigateur40/0/0 ;
lectures24/0/0 et démon13/0/0 revalidés sous le confinement enfant attesté. Les sources,
rouges et preuves partielles restent distingués ci-dessous. Les worktrees ED isolés portent
le lot jusqu'au push, puis sont retirés par garde et archives administratives fidèles.

## Ce que le correctif garantit

Un accord retient la ligne complète exacte et le domaine canonique. Il ne devient ni un acquis
de famille ni un droit sur les sous-commandes isolées. Jugement et laissez-passer composent
la même primitive ; les chemins synchrone et en fond l'utilisent. Le mode standard ne reçoit
pas d'accord différé. Une portée bornée CetteFois est consultable sans consommation au jugement,
puis consommée atomiquement lors de la création des laissez-passer ; deux appels concurrents
ne peuvent obtenir le même accord unique.

L'accord de ligne ne donne aucun droit noyau supplémentaire. Le témoin de reprise déclare
`workspace.sandbox.extra_read` pour le seul script extérieur, avant ouverture, avec contrôle
de la configuration réellement chargée et sans écriture extérieure. Le script reste dehors ;
la variante ajoute un argument, doit attendre un nouvel accord et ne produit aucun effet.

## Preuves ciblées réellement lues

- `6cbbb22a9` : rouge 0/2, reprise extérieure et collision entre argv au même affichage.
- `b4c1d13ef` : rouge 0/2, accord CetteFois permanent et deux autorisations concurrentes.
  Aucun laissez-passer destructeur exécuté.
- `aed5ef115` : commande **50 réussis, 0 échec, 1 helper ignoré** ; run_nodes **18/0/0**.
  Le helper `role_hote_qui_sort` est exercé avec `--ignored` par son test parent.
- `157a2f272` : témoin causal du vrai bac **1/0/0**. Les fichiers sont lisibles côté hôte ;
  Landlock est exigé actif. Sans lecture déclarée, le processus accordé rend une erreur
  Permission denied ; avec le seul script déclaré, il rend sa sortie ; son frère reste illisible.
- Navigateur initial `5611fc98e` sans lecture déclarée : rouge produit **0/1/0 en 1,13 s**
  après clic, deux commandes identiques et nouvelle demande d'accord. Les refus de démarrage
  Chrome antérieurs sont des montages, pas cette preuve produit : TMPDIR trop long pour le socket.
- Navigateur `aed5ef115` sans lecture déclarée : rouge **0/1/0 en 30,73 s** ; accord franchi,
  wait_output atteint, puis Python Errno13. Ce résultat reste conservé ; il n'est pas vert.
- Baseline déclarative `8b9499006` : rouge **0/1/0 en 0,90 s**, malgré extra_read du script.
  Candidat `996af8ee4` : vert **1/0/0 en 1,32 s**, même helper et même témoin byte-identiques.
  Vrai clic, commande identique, sortie différée et cwd réel, puis variante sans effet.

Preuves luciepc : `~/.cache/rag3weaver-build/I-luciepc/accord-20261112/` pour ED ;
`~/.cache/rag3weaver-build/cx-accord-exterieur-<source>-<étape>-20261112/` pour archi.
Les chemins précis et les préconditions sont consignés dans le ticket canonique.

## Passe finale et confinement attesté

Sourceb42, lib29 épinglée, -j8, target privé9,0Go (budget32Gio). Globale b42-r2 :
features5/5, native1609/0/15 en20,79s, large2176/0/65 en41,92s, navigateur40/0/0 en131,60s.
Le nouvel ignoré est précisément le navigateur d'accord extérieur, effectivement vert parmi40
à l'étape séparée. Liste exhaustive des ignorés conservée dans les journaux de preuve.

Le premier montage utilisait un client createdb absent de l'hôte ; ce refus avant tests est
archivé. Le client du conteneur PostgreSQL existant a créé seulement la base privée du lot.
Une passe b42 avait ensuite échoué19 fois sur le segment commandes/index attendu par les
fixtures ; même source/assertions, racine privée corrigée en commandes : lectures24/0/0.
Le rouge et les logs sont conservés ; la nouvelle globale entière b42-r2 est fonctionnellement
verte. La [dette de fixture](../../recherche/tickets/2026-11-12-les-temoins-de-lecture-imposent-le-nom-du-dossier-des-journaux.md)
est séparée et transmise à recherche/mémoire ; aucun correctif de fixture dans les accords.

La revue du driver a identifié une limite de sa preuve mémoire : run_e2e peut créer une scope
sœur. Ni le cgroup parent ni son pic ne prouvent alors ceux des enfants. Sur autorisation
orchestration, seules les deux e2e ont été revalidées, sans refaire features/lib/nav : verrou
réel externe, RAG3WEAVER_VERROU_TENU, BUILD_MEMORY_HIGH=0 pour éviter la scope intérieure,
Max32 extérieur maintenu, KillModecontrol-group et budget600s après acquisition.

Résultat : lectures24/0/0 en9,49s et démon13/0/0 en12,26s, deux suites sur deux, zéro sauté.
PID2697195 et2702388 du vrai binaire e2e, cgroup de l'unité privée et memory.max34359738368
attestés ; unité23,482s, pic11,4Gio, aucun swap. Quiescence exeTarget/cwdWT=[] et santé7879
après la suite. Le [ticket commun de scopes](../../orchestration/tickets/2026-11-12-une-scope-e2e-imbriquee-peut-echapper-au-plafond-du-lanceur.md)
reste ouvert à orchestration ; aucun changement de runner produit dans ce lot.

Preuve finale aux deux postes après rapatriement :
`~/.cache/rag3weaver-build/I-luciepc/accord-20261112/verdict-compose.json`.
ELF navigateur98bc3103ec21dffeca418a9cb27f2e5f37565fb7369fab2217e379a86f19ed94 ;
source embarquée b42a4ada9,220 gabarits, aucun absent. Les preuves des enfants sont
`e2e_lectures-enfant.json` et `e2e_demon_hote-enfant.json`, et le scan de quiescence est conservé.
Le verdict est composé explicitement ; les anciens passages ne sont pas réattribués.

## Autres suites et limites

Le [lot de coordination clos](../../tickets/closed/2026-11-12-paralleliser-les-suites-autorisees-sans-chevauchement.md)
atteste l'affectation. M8 reste porté par mémoire, avec sa
[revue canonique](../../memoire/12-11-2026/01-relecture-m8-adoption-fusion-et-reprise.md).
Le lot équipier figé et son rouge Gemini restent inchangés : aucun nouvel essai Gemini,
aucune intervention sur la mémoire réelle, aucune activation ni autre lot repris ici.
Les premiers refus de montage et la régression de compatibilité shell du correctif sont
consignés dans le ticket clos, avec rouges distincts ; shell littéral explicitement accordé
reste CetteFois, sans aucun shell exécuté par le nouveau témoin de décision.

## Nettoyage archi attesté

Archi confirme le retrait aux deux postes de son worktree cx-wt-accord-exterieur, de son
target cx-archi-target, des scratches et de ses trois branches locales, après quiescence,
archives administratives fidèles (sous-module PC inclus) et garde courante dca13abb… verte.
Remotes et preuves conservés dans cx-accord-exterieur-nettoyage-garde-20261112 ; copies
PC et portable distinctes. Une branche PC sans upstream a d'abord refusé -d ; son upstream
distant exact a été vérifié et renseigné avant le -d normal, sans -D. Cette attestation
vient d'archi ; aucun nettoyage ED n'y est assimilé.
