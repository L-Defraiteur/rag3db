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

## Arrêt d'instrumentation et inventaire exact

Source globale figée `efdf90fb9f94ca580e41b53c912c2c718b13f4ed`, début855116.22498967,
deadline860816.22498967. F5check VERT5/5 ; minimal sans features VERT36,92s ;
compilation large VERT3min46. Avant tout test, l'inventaire direct du binaire échoue127 :
`librag3db.so` absente du chemin de chargement. Le wrapper Cargo exporte ce chemin ;
la commande directe de l'instrumentation l'avait omis. Aucun test global n'a alors tourné.
Premier rouge conservé dans erreur/fin.json/verdict de la recette initiale, elapsed409,35s.

Diagnostic : inventaire seul repris avec exporter_la_lib/lib29, sans nouvelle compilation,
sous poste lourd/unité privée. Binaire exact rag3weaver-4950c451c9e109ca :
**2269 tests,65 ignorés**, donc **2204 succès attendus**. L'estimation2179 était périmée ;
aucune exclusion modifiée. Listes lib-large-liste.txt et lib-large-ignores-liste.txt,
SHA du même ELF dans lib-large-elf.sha256. Aucun test exécuté par cet inventaire.

Recette corrigée publiée avant reprise : reprise.py SHA256 `a7692242775d76d6f6bd822e733dba1d12cb9e448fecae4cc74b55483b873be4`
dans le même dossier de preuves. Elle reprend uniquement liblarge900→NAV600→PG300,
sans F5/minimal/compilation déjà verts ; source efdf inchangée et **deadline initiale
860816.22498967 chargée depuis debut.json**, aucun reset. Tous les autres plafonds,
arguments, exclusions et surveillances restent identiques. Logs et verdict de reprise distincts.

Complément demandé par C7 : l'observateur de reprise conserve aussi les maps réelles
des exécutables large/NAV/e2e observés, vérifie leur native29, leur empreinte source efdf
et les220 gabarits embarqués. RUNPATH sera conservé par readelf avant reprise.
SHA final de reprise.py, remplaçant le précédent avant toute exécution : `d599a6bee302f92d9e14a9f602f4625acdd31f0b7acff369d10a4be1fa0a4be2`.

## Interruption prudente de la lib large : avertissements RADV

La reprise a été arrêtée volontairement après31,467s (SIGTERM à sa seule unité privée),
sans verdict final de lib. Des avertissements RADV/Vulkan sont apparus ; ils ne prouvent
ni chargement d'un modèle ni calcul GPU. Journal partiel conservé séparément sous
lib-large-interrompue-radv.log, descendants-interrompus-radv.json ; quiescence-arret-radv.json=[].
Le premier ELF large a bien fourni ses maps native29/source efdf/220 gabarits.

Lecture : regime.rs376→driver_card_types388 crée une Instance puis enumerate_adapters(VULKAN)
et lit get_info, sans request_device ni tenseur. estimate.rs445 l'appelle dans estimate_here,
atteint par les parcours worktree de l'hôte. C'est un chemin compatible avec les avertissements,
pas une attribution dynamique exhaustive. BurnDevice::Cpu passe aussi par wgpu/Vulkan ;
la variable cpu ne signifie donc pas zéro initialisation de pilote. Les variables de rôle
priment sur la variable commune. Aucun correctif produit, aucune exclusion nouvelle.

Diagnostic et proposition de recette bornée soumis à C7 avant reprise : admettre explicitement
l'énumération matérielle de la lib large, maintenir aucun modèle/inférence réel, poser les
choix cpu commun et trois rôles si construction éventuelle. La surveillance conserverait les
provenances natives et descripteurs de périphériques, sans transformer une ouverture de
périphérique en preuve de calcul. F5/minimal/compilation acquis ; source efdf et deadline
860816.22498967 inchangées. Aucun usage7879. Cette section ne constitue aucun vert de lib.

GO de C7 reçu pour la recette adaptée, publiée avant exécution : reprise-adaptee.py
SHA256 `59332ced215e16c0f980ec60fae079826c4fc2c973042ad1acc29e1fe869188a`. Mêmes source efdf, ELF et deadline860816.22498967 ;
lib2204/65→NAV40→PG36 seulement. CPU posé explicitement sur variable commune et trois rôles ;
énumération matérielle autorisée, RADV connu conservé sans nouvel arrêt sur ce seul message.
L'observateur garde PID/starttime/maps et ouvertures /dev/dri ou /dev/kfd ; celles-ci
ne prouvent pas une charge GPU. Aucun modèle/inférence/service7879 ni charge seat1 prévu.
Tout signal nouveau arrête pour diagnostic. F5/minimal/compilation ne sont pas rejoués.

## Verdict composé BGE — prêt à intégrer

**VERT**, source exécutée `efdf90fb9f94ca580e41b53c912c2c718b13f4ed`,
production `691ba039140f20b6460623fe9f931f04e2d5fbed`. Les commits suivants sont uniquement
Markdown ; aucun runtime ou gabarit modifié depuis cette source. F5check5/5 ; vrai minimal
check --lib --no-default-features rc0 ; liblarge **2204/0/65** (36,74s), NAV **40/0/0**
(133,53s), e2e_postgres **36/0/0** (7,88s) plus ses deux filets préalables verts.
Les65 ignorés sont listés par le binaire exact ; NAV réellement jouée séparément.
Pas de saut ajouté ni de compte ancien imposé.

Trois ELF, source efdf/220 gabarits chacun, maps réelles chargeant exclusivement native29 :

- large rag3weaver-4950c451c9e109ca : f0b7f59dd411d9ed954fc3e24eba4d4676fd996fe7bb2065b5ba72e0be0775dc ;
- NAV rag3weaver-32ae91ef3133ff98 : 04d2d7e975ca7d34205c8e33e060b2b628e3fa3745c8be13c6d74929a47cd95f ;
- PG e2e-b3458ee334fbabdd : 18efabce7b1150d767c740db1157f644945958a0d0b24cd9a44783fdf320f242.

Les fichiers dynamic-*.txt gardent les métadonnées ELF, provenances/*.json et *.maps le
chargement réel et le cgroupMax32. Chromium natif et helpers D-Bus observés dans la portée
privée ; aucun signal de sortie de cgroup. Ouvertures de périphériques conservées séparément,
sans les confondre avec une inférence. Pas de modèle réel/service7879 lancé ou appelé.

Quiescence finale **[]**. Target9134875147 octets. Global1025,232s/5700 depuis le début
initial, fin856141.457<deadline860816.225 : interruptions et adaptations comprises.
F5/minimal/compilation n'ont pas été rejoués. Rouge rc127 et lib interrompue RADV conservés ;
le verdict composé distingue leurs diagnostics des étapes finalement vertes. Preuves copiées
aux deux postes dans I-luciepc/bge-routage-20261112/globale, verdict-compose.json.

Intégration demandée à C7 après ces preuves. Pas de master avant son « à toi ».
Le ticket BGE reste ouvert pour activation/correction des copies utilisateur non inspectées ;
ce lot ne les migre pas. La dette de features native+daemon reste séparée et ouverte.

## Intégration BGE et retrait des ressources

Master **6dd116e8f70fd328acf3b45a89401e06f71365cb**, sous poste push ED-BGE après
« à toi » C7. Dernier fetch avant if VERT, trois contrôles lus après push : status propre,
aucun rebase, origin/master=HEAD. Rebase documentaire sur87799f681, branche neuve
cx-ed-bge-routage-2 publiée ; diff depuis efdf limité aux Markdown, production/gabarits
éprouvés inchangés. Un premier appel poste push mal formé a refusé avant toute commande ;
appel corrigé avec nom de session et séparateur, aucun push sans verrou.

Les deux worktrees et branches locales du lot sont retirés ; targetPC9134875147 octets
supprimé. Base privée cx_ed_bge_20261112 exportée en dump personnalisé, SHA vérifié aux
deux postes, aucune connexion avant drop, absence vérifiée après. Pas d'autre base touchée.
Trois ELF, journaux, rouges et inventaires conservés aux deux postes. Archives fidèles
des administrations privées (module codeparsers inclus à luciepc) avant gardes fraîches
SHA dca13abb7a84cf3c1419f728697837a6b1b9b20e34e34c144c9fbf835252f628 VERT.
Le premier git worktree remove PC sans --force a refusé la présence du sous-module après
un garde vert ; rapport neuf vert puis --force selon la procédure, aucune garde contournée.
Branches supprimées par -d normal, remotes gardés ; aucun GC/prune/réparation.

Preuves : I-luciepc/bge-routage-20261112/globale : master-integre, trois-controles,
equivalence-finale.json, verdict-compose.json, ELF/, provenances/, dumps+SHA, gardes
portable/PC, administrations fidèles et copies d'archives pax entre postes, nettoyages.
L'arbre Lucie conserve submodule codeparsersM et docs/lucie-tmp préexistants.
Le vieux lot reste-explicite et sa base sont hors périmètre du retrait.

Le ticket BGE reste ouvert : gabarits corrigés/intégrés, copies utilisateur et activation
toujours non qualifiées. La dette native+daemon reste également ouverte, sans fix parasite.

## Publication concurrente d'un accord — préfixe déterministe

Reprise parallèle autorisée par C7 sur base43db9634a, WT/branchecx-ed-accord-publication,
propriété atelier_fichiers/testsaccords, archi en revue seule ; aucun backend_schema/migration.
Ticket canonique : docs/rag3weaver-archi/tickets/2026-11-12-qualifier-la-publication-concurrente-d-un-accord.md.

Source b708f7c34 extrait sans changer l'ordre les phases réellement utilisées par
decider_accord (snapshotNone→publication→commit). Deux snapshots de la même demande,
A finit puis son usage est consommé, B tardif republie avant son erreur : rouge attendu
**4réussis/2échecs/0ignoré**. Même défaut quand A refuse et B accorde. Les permis de
la vraie Garde sont créés/consommés puis détruits, jamais exécutés.

Correction03ee5cdcf : mutexArc par demande, attente hors registre, relecture avant
publication et verrou tenu jusqu'au commit ; registre libre pendant with_backend
(préserve ordre backend→accords). Aucun changement de politique/portée ni révocation.
Septième témoin : A retenu après publication avant commit, mutexdemande tenu/registredisponible,
B annoncé puis callbackperdant0 et uniqueusage. Ne prouve pas quel thread l'ordonnanceur
a bloqué à cet instant ; les deux témoins stale-snapshot restent déterministes.

Préfixe vert source03ee : **7/0/0**, compile28,56s, runtime0,00s. Source/recettes publiées
avant les étapes, lib29 complète/j8/Max32/Highmax/Swap4/targetunique32. Même budget600
après acquisition réelle : début857923,253385098, deadline858523,253385098 ; fin du
log estimée858340,7068, pas de reset. ELF SHA256
6bdcc333c02a04e65db701c1eb747edfe2e0d8131075434c1eef76b046a99f1d, source03ee et220gabarits
embarqués vérifiés. Quiescence[], target1642372104 octets. Pas de modèle/GPU/service,
commande/accord réel ni configuration/base utilisateur ; hôtes fixtures sans backend
et journaux/XDG strictement privés. Preuves2postes I-luciepc/accord-publication-20261112.
Archi a relu sources et logs rouge/vert directement, avis favorable borné au ciblé.

Reçu en erreur : fixture journal impossible après commit laisse l'accord appliqué,
retry déjàdécidé. Ticket séparé ouvert : docs/everything-declarative/tickets/2026-11-12-un-accord-peut-etre-applique-malgre-une-erreur-de-journal.md.
Pas de rollback ajouté. Absence d'API révocation/annulation documentée, pas inventée.
Reload garde l'état vivant ; nouveau registre/hôte ne rejoue pas les accords du journal.

Plan proposé à C7, **pas lancé** : F5check≤2400→minimalchecklib≤120→compilationlarge≤1500
→liblarge≤900→NAV≤600→lectures≤120→demonhôte≤120 dans global6000 après acquisition,
nonreset, lib29/j8/Max32/HIGH0/target32 unique. Inventaire exact des futurs binaires et
ignorés avant runtime, aucune estimation ancienne imposée. NAV busprivé sans activation
et surveillance descendantsréels. Lectures contient une preuve sparse BGE réelle :
son besoin du service7879 existant doit être coordonné explicitement, santé avant/après ;
ne pas la présenter comme CPU/factice. Pas de nouveau service ni modèle à lancer.
Premier rouge STOP/diagnostic. Aucun lourd supplémentaire avant revue du plan ;
intégration seulement APRÈSA et tourmaster C7, sans réservation pendant développement.

## Globale accords6f527 — rouge hôte arrêté et diagnostic

F5VERT5/5, minimalVERT37,09s, compilations/listes3ELFvertes publiéesfa01dfb8f avant
runtime ; lib2229/0/65 en36,11s, NAV40/0/0 en131,29s et lectures27/0/0 en9,29s
(+deux filets) verts. NAV terminée : aucun Chrome privé vivant, un seul DBus commun.
7879 BGE-M3/dim1024/facticefalse/sparsetrue validé avant/après, rendu immédiatement.

Premier rouge inattendu : e2e_demon_hote dépasse120s,12finsok/13, dernier test
code_tree_pose_enregistre_clone_et_retire_un_arbre ; aucun verdict suite13. Driver
arrêté, primitive finaleROUGE/code1/descendantslaissés aprèscommandes, restes[],
nettoyétrue, quiescence indépendante[]. Pas de skip, timeout étendu ou retry inchangé.
Aucun verdict global vert. Deadline originale866610,959853126 conservée.

Diagnostic concret : ce témoin coupe journal au premier espace (lignes605-611) ;
root « racine de journaux » devient chemin inexistant « racine ». Le journal réel
PID227077/1794515589662-227077.out contient déjà « indexation terminée » avec scopes
et relations posés. Le délai ne démontre pas un blocage de publication d'accord.
Dette distincte créée AVANT transmission : docs/recherche/tickets/2026-11-12-le-temoin-code-tree-du-demon-tronque-le-journal-au-premier-espace.md.
Aucune fixture du lot accords modifiée. ReçuHTTP exact et pile non capturés ;
lecture statique + vrai journal terminé documentés, propriétairesPID/start/cg/maps
déjà capturés avant arrêt dans les preuves de supervision et provenances.

Proposition soumise à C7 : reprise de la SEULE suite démon13, même source/ELF gelés,
avec RAG3WEAVER_JOURNAUX_DIR privé sans espace, borne120 incluantcleanup, deadline
globale initiale conservée (aucun nouveau6000). F5/min/lib/NAV/lectures ne sont pas
rejoués. Nouvelle supervision commune valide sous poste réellement tenu ; pas
d'outerservice/marker. Pas de service7879 ni modèle/GPU réel, CPU4 et ENV Cargo
complet inchangés. Ce changement d'environnement contourne une dette de fixture
explicitée, pas une preuve de portabilité corrigée. Aucune reprise lancée à ce stade.

## Verdict composé et préparation d'intégration accords

Verdict **VERT**, preuves deux postes I-luciepc/accord-publication-20261112/globale/verdict-compose.json.
Initiale6f527 : F5check5/5, minimalchecklib0, lib2229/0/65(36,11s), NAV40/0/0(131,29s).
Corrective2a31 : F5montages5/5, helpers3/0/0, lectures27/0/0(9,20s), hôte13/0/0(12,30s),
filets2/0/0 dans chaque suite. Racine EXACTE « racine de journaux » conservée ;
index/usages/clone/retrait tous exercés. Les3vecteurs encore dus restent explicitement
une dette, pas du dense prêt. Aucun nouveau modèle/service pour masquer le défaut.

Le rouge initialhôte120 et la primitiveROUGE restent conservés séparément ; la nouvelle
fixture commune a justifié F5+consommateurs, sans rejouer lib/NAV produit inchangés.
PrimitivecorrectiveVERT/code0/restes[]/nettoyétrue, quiescence indépendante[].
Budget total2385,7608/6000 (temps de préparation/reprise et premier120 compris),
fin862996,72065<deadlineinitiale866610,959853126. Target8623326051octets.

Trois ELF initiauxsource6f/220gabarits préservés avant recompilation (ELF-initial/) ;
nouvel e2esource2a/220 (correctif-journal/ELF/) SHA921167177138e8b2d721190fbd72db47db02d2c090e298822b9bad8c1d3556de.
Provenances runtime PID/start/mapsnative29/cgroupMax32 et reçus de supervision archivés.
Quatre ELF/logs/rouges/proofs aux deux postes ; 7879 rendu avec santé BGE1024/sparse
avant/après les deux passages lectures. Aucun usage ED restant.

Archi a relu directement les logs helpers/lectures/hôte et le verdict de supervision ;
C7 a relu verdict-compose et donné « à toi » après bilan/README/clôtures/copies.
Tickets clos : docs/tickets/closed/2026-11-12-qualifier-la-publication-concurrente-d-un-accord.md ;
docs/tickets/closed/2026-11-12-le-temoin-code-tree-tronque-le-chemin-du-journal-aux-espaces.md.
Doublon fermé comme remplacé, preuves fusionnées au canonique. Dette journal après
effet reste ouverte, sans rollback ni révocation inventée. Limites du ciblé sur HTTP
concurrent/relance du perdant et absence d'API révocation restent documentées.

Code Rust/tests/templates/Cargo inchangé depuis143a21706 ; les commits suivants sont
Markdown uniquement. Arbre B empile sa validation sur cette base, ordre masterED→B.
Après push : trois contrôles puis retrait immédiat gardé de nos deuxW/target/branches
locales et PGprivée après dump+SHA, sans toucher ancien reste-explicite, sources de
Lucie ou ressources B. Les remotes/logs/rouges/ELF/admins restent préservés.

## Intégration accords et fin des ressources privées

Master **0bf06b21a5fda28520f6ae0b2b2ad07acbeac331** poussé sous poste push
ED-accord-publication, dernier fetch avant if VERT. Trois contrôles lus après push :
status propre, aucun rebase, origin/master=HEAD. Code/tests/templates/Cargo identiques
à143a21706 (équivalence JSON vérifiée), préservant la base de validation empilée B.
Le SHA et la quiescence PC ont été transmis immédiatement à C7 et Arbre ; aucune
attente ED ne bloque sa suite. Les deux tickets corrigés sont clos, le doublon
remplacé ; la dette journal après effet reste ouverte.

Dans le même script après les contrôles : archive fidèle de l'administration privée
PC, module codeparsers inclus, garde fraîche
SHA dca13abb7a84cf3c1419f728697837a6b1b9b20e34e34c144c9fbf835252f628 VERT,
retrait W initialisé avec --force seulement après ce garde, trois branches locales
-d normal et target8623326051octets retirés. PGprivée cx_ed_accord_publication_20261112
exportée format personnalisé, SHA vérifié aux deux postes, zéro connexion avant
drop, absence vérifiée après. Aucun service partagé arrêté.

Portable : archive fidèle, garde fraîche VERT puis retrait W/branche3 par -d.
Quatre ELF, journaux/rouges, source/220gabarits/mapsnative29, reçus et verdicts gardés
aux deux postes. Archives administratives pax croisées avec SHA ; contexte synthétique
code-tree rouge sauvegardé avec sa propre administration Git. Rapports e2e ignorés
déplacés hors source avant le garde. Aucun GC/prune/réparation ou suppression remote.

Preuves : I-luciepc/accord-publication-20261112/globale : master-integre,
trois-controles, verdict-compose.json, equivalence-code-stable.json, ELF-initial/,
correctif-journal/ELF/, gardes portable/PC, administrations fidèles et pax+SHA,
cx_ed_accord_publication_20261112.dump+SHA, fixture-code-tree-227077.tar, nettoyages.
L'arbre de Lucie conserve codeparsersM et docs/lucie-tmp préexistants. Ancien
reste-explicite et ressources B sont hors périmètre, jamais retirés par ce script.
Pas de nouveau Cargo/NAV/service7879 après cette validation.

### Nuit du 12 au 13 — parcours agent IDE (en cours)

Ticket [parcours réel](../../tickets/closed/2026-11-12-prouver-le-parcours-agent-de-code-complet-de-l-ide.md).
Worktree `cx-ed-parcours-agent-ide` sur les deux postes, branche portable homonyme,
source publiée `36e284824`, base `a783013f1`. Nouveau témoin ignoré : vrai Vertex
via chat/outils, exploration, Cargo nommé, édition/diff et relecture tâche/chat.
Préfixe initial lancé luciepc sous primitive commune, global1900s (compile1500,
réel360), target privé `target-cx-ed-parcours-agent-ide`, lib29/j8/Max32/HIGH0.
Preuves `~/.cache/rag3weaver-build/I-luciepc/parcours-agent-ide-20261112`.
Aucun verdict acquis à cette publication ; pas de preuve UI revendiquée.
Aucun usage 7879/GPU prévu : fixture non indexée, rattrapage désactivé explicitement.

Revue courte parallèle du ticket Archi « vidange de portée rouge dans la lib » :
plusieurs assertions possibles ; source/log sans panic ne permettent pas d’établir
une cause. Proposition transmise : même ELF et concurrence avec `--nocapture`,
sans augmenter les délais ni changer le runtime. Aucun fichier commandes édité.

Actualisation parcours : rouge réel `36e284824` conservé après test nommé rouge
attendu puis vert réel ; diff Git bloqué par l’administration externe du worktree,
reprise non atteinte. Corrections communes publiées jusqu’à `4d6045cdb`, en revue
RO Archi/C7/Mémoire : lectures Git vérifiées, environnement global isolé, alias
internes et gitfile symbolique refusés. `b0090e2be` noyau rouge config globale,
`b310b5c0c` noyau vert partiel puis arrêt RO, `b68569d3a` noyau vert partiel puis
arrêt RO ; jamais de Vertex dans ces trois correctifs. Arrêts attribués pidfd,
restes vides et nettoyage vrai. Preuves initiales/correctives distinctes,
synchronisées sur les deux postes. Nouvelle source4d à qualifier ; aucun master.

Limite de conservation des préfixes : SHA et logs des ELF successifs conservés,
mais les premiers exécutables36e/b009/b310 ont été remplacés au même chemin par
les rebâtis Cargo avant copie indépendante. L’ELF b685 est désormais archivé hors
target ; les suivants doivent l’être immédiatement après compilation. Les relevés
`ldd` tardifs sont nommés « observés après b685 », sans attribution rétroactive.

### Parcours IDE acquis et globale en file

Préfixe source4d : noyau1/0/0 puis3toursVertex46.05s verts. Reprise sourcefe0,
sur la MÊME fixture : bouton de chat réel dans Chrome, quatrième tour Vertex,
Cargo exact1passed/diff0 et relecture visuelle, 1/0/0 vert19.71s, CDP0.23s.
Deux supervisions vertes/restes[]/nettoyé, code/principal/historique conservés.
Rouges Git/config/alias et Chrome socket trop long conservés ; aucun retry inchangé.

Branche2 `cx-ed-parcours-agent-ide-2`, rebase65bC7 préservé, source8e366e4aa
publiée, quatre Rust identiques àfe0. Revues RO Mémoire/C7 favorables Git, puis
Mémoire favorable delta UI/stderr. Globale6000 en file derrière mesure exclusive,
F5/min/3ELF/lib/NAV/lectures27/hôte13/PG36+filets ; gate7879 seulement aprèsNAV.
Base privée `cx_ed_parcours_20261113`, target5.8G initial, TMPDIRcourt obligatoire.
Le ticket reste ouvert jusqu’à passe, intégration C7 et nettoyage gardé2postes.

### Union validée et livraison prête

Sourcee4eaf4ef2 surmaster8f604328a, branche3 neuve. Préunion8e F5/lib2244/NAV40
verte puis arrêt volontaire au gate avant7879 ; verdict de portée rouge15 gardé,
restes[]/nettoyé. Union : F5 5/5, minimal0, lib2253/0/66(46.86s), NAV40(131.07s),
lectures27(9.54s), hôte13(12.32s), PostgreSQL36(7.48s), filets2 chaque entrée.
Supervision unionVERTE0/restes[]/nettoyé, quiescence indépendante[]. Borne de
contrôle1446.97≤6000, même deadline878111.26244974, aucune remise à zéro.
7879 rendu santéBGE1024/sparse/facticefalse avant/après, aucun usage ED restant.

Preuves `globale-union/verdict-compose.json`, équivalence SHA du helper Git4d et
des témoins fe0 ; 223gabarits, mapsnative29/CG32/HIGH0, target29704553416octets.
Les e2e canoniques ont rebâti sous CARGO_INCREMENTAL0 : quatrièmeELF archivé,
les trois suites réellement jouées dessus, distinct du préparé. Tous logs/rouges
conservés. Ticket déplacé fermé avec source/proofs, README et nouveauté/recette
actualisés. Revues C7/Mémoire Git et Mémoire UI favorables ; tour master à demander
après copie finale2postes, puis contrôles et cleanup gardé W/target/PG après dump.

### Empilage C7 Pub→ED→Embar, union C9 finie

Avant tout master, C7 a demandé basePub31 (parent8f) ; branche4 nouvelle, sourcec9,
seul conflit news résolu en gardant les deux nouveautés. Les neuf fichiers Pub
strictement inchangés ; aucun Rust après gelc9, SHA/tree transmis à Embarquement.
Union complète : F5/min/3ELF verts, lib2267/0/66(47.57s), NAV40(131.66s),
lect27(9.73s), hôte13(12.45s), PG36(7.62s), filets2, scopeVERT/restes[]/quiescence[].
Même deadline878111, reste3809 aprèsposte, bornecontrôle2834.58/6000 inclut
préunions/préparation. Relevé budget au mauvais chemin séparé/restauré depuis
copie fidèle, noteaudit conservée sans effet sur délai. Target29846025589octets.

Rawproofs `globale-union-pub`, quatreELF(SHAs/mappings), 223gabarits, équivalences
Git/UI4d/fe0 et Pub31 ; CLI5/tx25 restent preuve Arbre. PGdumpfinalrafraîchi, ancien
avant-Pub conservé. Aucun master ED avant Pub+C7. Signalshow_progress d’Embar842
reçu/diagROtransmis, aucun runtimeédité : fixture demande index actif sansbarrière.
Routage C7 en cours ; préserver sourcec9 immutable pour sa qualification parallèle.

### Base Pub livrée, candidat ED rebâti sans changement de code

Pub intégrée masterbc1a81500a05489673b64c2e5930d335510e6dda. RebaseMD sur cette
base, branche5 neuve ; delta sourcec9 uniquementMarkdown/JSON de bilans. Le produit
Pub31 et les Rust/Cargo/gabarits ED restent identiques à la qualification C9.
Les scripts3024 sont historiques et seront remplacés pour ce candidat avant push.

Correctif de fixture COMPAGNON, hors sourceC9 figée : `08c379b11` sur
`cx-ed-carte-index-fixture`, parentc9, seulworktrees.rs/test+63lignes. Préinscription
fermée+débit de fixture évitent la sonde synchrone, serviceGranite factice CPU à
barrière maintient un vrai index actif, libérationRAII avantjoin. Assertions originales
conservées + état terminé texte réel sans carte. Ciblé exact1/0/0 vert3.70s après
compileF3/lib29 30.75s, scopeVERT0/restes[]/quiescence[], aucuneAPIruntime modifiée,
aucunGPU/Vertex. Commit/preuves transmis à Embar pour consommation dans son union,
sous ticket activation ; il ne modifie pas le candidat ED déjà qualifié.
