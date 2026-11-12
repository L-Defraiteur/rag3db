# ED — accords de commande, 12 novembre 2026

## Résultat du lot

Le [ticket d'accord extérieur](../../tickets/closed/2026-11-11-un-accord-de-commande-hors-domaine-redemande-le-meme-accord.md)
est corrigé et sa fermeture accompagne le lot d'intégration ED/archi. Source code éprouvée :
`b42a4ada90ab7f2b80f1137e75692b3d00e26fcc`, rebasée sur master3b2eab7ec ; range-diff19/19
identiques et fichiers Rust possédés byte-identiques au prérebase. Les commits suivants de
préparation du push ne changent que Markdown. Le lot a été intégré sur master `adcf01128527f4cb24027364489bd44e322ca089`, puis ses seules ressources ont été retirées.

Verdict composé vert : features5/5, native1609/0/15, large2176/0/65, navigateur40/0/0 ;
lectures24/0/0 et démon13/0/0 revalidés sous le confinement enfant attesté. Les sources,
rouges et preuves partielles restent distingués ci-dessous. Les worktrees ED ont été retirés après le push, par garde et archives administratives fidèles.

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

## Intégration et fin des ressources ED

Master `adcf01128527f4cb24027364489bd44e322ca089` poussé sous poste push et if VERT.
Status propre, aucun rebase et origin/master égal HEAD ont été lus après fetch. Dernier
rebase documentaire sur af2, nouvelle branche4 : écart depuis la source éprouvée b42
uniquement Markdown,220 gabarits comparés par SHA256 inchangés. SHA transmis à C7 et
mémoire immédiatement après les contrôles ; la reprise M8 n'attend aucun travail ED.

Dans le même script : garde courante SHA256
`dca13abb7a84cf3c1419f728697837a6b1b9b20e34e34c144c9fbf835252f628`,
rapports neufs VERT aux deux postes, archives fidèles des administrations privées
(le sous-module codeparsers PC est inclus dans l'archive de son parent), puis retrait
des deux worktrees accord, des branches locales concernées et du target privé PC.
La base privée cx_ed_accord_20261112 a été exportée en format personnalisé, SHA256
vérifié aux deux postes, puis supprimée ; zéro connexion avant dump et absence vérifiée après.
Les rapports e2e ignorés ont été préservés hors source avant le garde. Scratch privé final
conservé hors cible. Aucun GC, prune, réparation d'administration ou effacement distant.

Preuves aux deux postes dans `I-luciepc/accord-20261112/` du cache de build :
`master-integre`, `equivalence-code-assets-finale.json`, `verdict-compose.json`,
`garde-portable-finale.json`, `garde-luciepc-finale.json`, archives
`administration-portable-finale` (portable) et `administration-luciepc-finale`,
`cx_ed_accord_20261112.dump` avec son SHA, `nettoyage-final` et journaux de retrait.
Les remotes et preuves rouges restent conservés. Vérification finale de présence, sans
lire les données : ancien worktree reste-explicite toujours sur
`1acc6733823a8ac5883df525a4d08e142c2b5d10`, base cx_ed_reste_20261111 toujours présente.
L'arbre de Lucie garde uniquement son untracked lucie-tmp préexistant ; aucun checkout ni
commit n'y a été fait. Les deux tickets de dette restent ouverts à leurs porteurs.


## Gabarits BGE — préfixe isolé du 12 novembre

Ticket : `docs/everything-declarative/tickets/2026-11-12-des-manifestes-bge-visent-le-port-granite.md`.
Correction publiée `691ba039140f20b6460623fe9f931f04e2d5fbed`, base41497 : suppression des deux
adresses imposées, BGE/1024 et fournisseur service conservés. Aucun résolveur ajouté ;
adresse explicite > variable commune > alias, sélection par identité et refus sans repli
local implicite restent le contrat existant. Notebook requiert du vectoriel ; validated-result
(signaux vides) n'en requiert pas. Une ouverture en mots seuls n'atteste pas de dense prêt.
Les copies de configuration utilisateur restent non qualifiées et le ticket reste ouvert.

Préfixe à luciepc sous poste lourd, j8, lib29 complète épinglée, target privé unique (moins
32 Go), cgroup MemoryMax34359738368, Swap4G, KillModecontrol-group et TimeoutStop15.
Budget global900s : début monotonic854078.887042, échéance854978.887042, preuve finale854648.385073.
La borne n'a pas été réinitialisée entre les adaptations.

- Première recette native+daemon : compilation101 avant tout test, erreurs préexistantes
  chat.rs835/836 (fichier_nodes gardé par code) et import Boite inutilisé livraison_codex.rs92.
  Dette séparée publiée f3b7fba71, ticket `2026-11-12-la-lib-native-daemon-ne-compile-pas-hors-matrice.md`.
- Native seule, source5eef3fe01 : deux rouges attendus et un vert, aucun ignoré.
  Vert natif interrompu lors de la coordination de recette, aucun verdict vert revendiqué.
- Jeu exact matrice3 `rag3db-native,code,daemon,openai-llm`, source5eef3fe01 : deux rouges
  attendus et un vert, aucun ignoré. Correction691ba0391 : trois verts, zéro échec/ignoré ;
  témoin existant de déclaration embeddings/models.embed : un vert, zéro échec/ignoré.

ELF final `rag3weaver-58d880667e640359`, SHA256
`b0eebd6f7144748c32d1d9f04cc7b9f48f5773700a447f1d714bf2742cdd2df1`.
Preuves PC : `~/.cache/rag3weaver-build/I-luciepc/bge-routage-20261112/`, notamment
rouge.log, rouge-natif.log, rouge-matrice.log, vert-matrice.log, scripts exacts,
verdict-prefixe.json et quiescence-finale.json (`[]`). Aucun modèle, GPU, socket,
service réel, configuration ou base de Lucie utilisé. Arbre utilisateur inchangé.

Préfixe vert, pas verdict de passe globale. Plan soumis à C7 avant tout lourd supplémentaire :
check_features cinq jeux (2400s global), compilation large (1500s), lib large (900s),
navigateur isolé (600s) et e2e_postgres (300s, gabarits touchés). Borne globale proposée5700s
après acquisition, non réinitialisée ; même lib29/j8/Max32/target32Go. Préparation des noms,
comptes attendus et dépendances privées avant exécution ; pas de service modèle réel,
variables d'embarquement absentes, pas de nouveau juge/équipier/Gemini. Tout besoin d'un
modèle réel ou dépassement du target arrête ce plan pour diagnostic. Revue puis créneau
master « à toi » restent nécessaires. Aucun push master réalisé pour ce lot.

## Recette globale autorisée et figée avant lancement

C7 autorise la passe après revue90a/préfixe691, en ajoutant un vrai
`cargo check --locked --lib --no-default-features` (120s) dans la même borne globale5700s.
Source de production inchangée691 ; la source exacte de passe sera le commit de cette recette,
publié avant toute acquisition. Script externe conservé aux deux postes :
`I-luciepc/bge-routage-20261112/globale/passe.py`, SHA256 `fd234e22b77c18ab1e9fed591b295bc9798f85ab01ce3ae7876640dfe11360b0`.

Ordre : F5check2400 → minimal120 → compilationlarge1500 → liblarge900 → NAV600 → PG300.
Chaque phase est plafonnée par le minimum de sa borne et du reste global ; aucun reset.
La première erreur arrête et conserve son journal ; aucun warning masqué. Le surveillant
interrompt également check_features dès son premier ROUGE, sans attendre les autres jeux.

Comptes attendus : features5/5 ; minimal=code0 ; liblarge2244 témoins dont2179 succès
et65 ignorés déclarés (base du lot accord + trois nouveaux témoins BGE, aucune suppression) ;
NAV40 succès/0ignoré ; PG36 succès/0ignoré. La liste native du binaire large et les raisons
d'ignore seront conservées pour confronter ces comptes, pas corrigés arbitrairement après coup.
Les témoins navigateur ignorés sous liblarge sont réellement joués à l'étape séparée.
Aucun nouveau cargo test natif entier : le contrôle minimal est bien check --lib sans feature.

Tous les descendants attribués par PID/starttime sont surveillés : cgroup privé Max32,
MemoryHigh=max, pas de scope e2e sœur (BUILD_MEMORY_HIGH=0, verrou déjà réellement acquis).
NAV reprend le bus sans activation de268460373 : socket privé0700, XML sans include/servicedir,
dbus-run-session devant Chromium natif, aucun changement de ses options de sandbox.
PG : createdb refuse si `cx_ed_bge_20261112` existe ; current_database est vérifié puis URL
explicitement épinglée à cette base avant table_rase. Aucune base existante/default n'est ciblée.
Lib29 complète, j8, target unique32Go ; variables modèles et identifiants Gemini retirés,
pas de modèle réel/GPU/7879. Poste lourd respecte la priorité mesure. Après passe :
quiescence, empreinte ELF/220 gabarits, preuves aux deux postes, revue puis tour master.
