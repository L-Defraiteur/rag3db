# Deux chronologies conservées, sans nouveau run

12 novembre 2026. Même [ticket annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md),
porteur cœur c++. Exécution des phases documentaires 1–4 de la recette 12.
Copies intégrales : [manifest et empreintes](preuves/ddl-minute-existant/manifest.json),
[extraction structurée](preuves/ddl-minute-existant/extraction.json).
Deux logs, deux XML, base.sh, departage.sh et suites-codes.txt sont conservés.
Les scripts sont des preuves historiques, **pas des commandes à rejouer**.
Aucun moteur exécuté, aucun build, aucune réservation.

## Définition exacte des mesures et fenêtres

Source moteur 0511dece7 :
`test/transaction/concurrence/twenty_writers_scenario_test.cpp:306–313` lance un
échantillonneur qui conserve `(now(), bufferManager->getUsedMemory())`, puis dort
100 ms. Ce n'est pas un échantillonnage continu ni une cadence temps réel garantie.
`now()` est une durée en millisecondes de Clock depuis le démarrage des acteurs ;
le chargement initial de la base précède ce démarrage.

`buffer_manager.h:218` expose le compteur atomique usedMemory, pas le RSS ni une
mesure exhaustive de la mémoire du processus. Initialisation : capacité de file
d'éviction × sizeof(EvictionCandidate) (`buffer_manager.cpp:71`). `reserve(:365)`
incrémente avant tentative d'éviction ; des réservations concurrentes en cours
peuvent transitoirement faire dépasser la taille du pool à ce compteur. Le pic
2052 Mio pour un pool configuré 2048 Mio ne prouve donc pas une allocation physique
stable hors limite.

Le banc calcule `peak(from,to)` (:771–792) comme le maximum des échantillons
avec `at >= from && at < to`, puis division entière par 1024². Sans échantillon,
le résultat est **0** : le journal ne distingue pas fenêtre vide et usage nul.
« avant DDL i » couvre `[fin du DDL précédent, début du DDL i)` ; pour le premier,
`[0,début)`. « pendant » couvre `[début,fin)`. Un pic avant n'est jamais une mesure
à l'instant de début du DDL. Pour la migration annulée, son début/fin existe en
mémoire mais n'est pas imprimé sous la forme des DDL ordinaires : ne pas inventer
ses bornes numériques à partir d'un zéro.

Les agrégats d'erreurs/latences utilisent le chevauchement strict
`sample.start < ddl.end && sample.end > ddl.start` (:547–606). Une transaction
longue peut compter dans plusieurs fenêtres DDL ; « hors DDL » ne chevauche
aucun DDL. Les nombres ne sont pas des événements horodatés ni des catégories
mutuellement exclusives. L'ordre d'impression des agrégats est fixé après les
threads ; il n'est pas l'ordre réel des erreurs.

## Passage base — chronologie propre

Log [base](preuves/ddl-minute-existant/base-TwentyWriters.DdlEveryMinute.log),
XML : 1009,646 s au total, contrôle de latence rouge ; les temps ci-dessous
sont ceux imprimés depuis le démarrage du scénario, en ms.

| DDL | Début | Fin | Pic de fenêtre avant / pendant, Mio |
|---|---:|---:|---:|
| 1 ADD note | 60000 | 60012 | 1071 / 0 |
| 2 ADD rang | 120000 | 121912 | 1152 / 398 |
| 3 CREATE REL TABLE | 180000 | 180089 | 1176 / 1122 |
| 4 CREATE_VECTOR_INDEX | 240000 | 992409 | 1177 / 1341 |
| 5 CHECKPOINT | 992410 | 992898 | 0 / 108 |
| 6 ADD b | 992907 | 992911 | 0 / 0 |

Le DDL 4 retarde les suivants : sleep_until ne décale pas l'origine du calendrier.
Les lignes 7–8 impriment déjà un report de checkpoint (journal environ 16 Mio),
sans timestamp ; ce n'est pas une erreur d'annulation. La première exception
textuelle imprimée (:27) est **attendue** : second ADD a refusé car a existe,
COMMIT dit transaction annulée ; le ROLLBACK suivant ne trouve plus de transaction.
Ne pas la classer comme premier défaut du moteur.

L'agrégat :73 rapporte un conflit concurrent pendant le DDL 4, sans timestamp
individuel ; c'est la première erreur d'acteur explicitement imprimée, pas
nécessairement sa position chronologique parmi tous les événements.
Les contrôles rouges :74–81 concernent l'attente Doc/Note au DDL 3. Les comptes
avant/après réouverture :83–86 concordent : Doc 59927, Link 447312, Note 59683 ;
valeurs par défaut correctes. Aucun message d'annulation inachevée/réouverture
obligatoire dans ce log. Ce silence ne prouve pas l'absence de tout rollback.

## Passage corrigé isolé — chronologie propre

Log [isolé](preuves/ddl-minute-existant/isole-TwentyWriters.DdlEveryMinute.log),
XML : 376,830 s, 14 lecteurs refusés au contrôle de création d'index et exception
Binder finale.

| DDL | Début | Fin | Pic de fenêtre avant / pendant, Mio |
|---|---:|---:|---:|
| 1 ADD note | 60000 | 60061 | 1068 / 275 |
| 2 ADD rang | 120000 | 120009 | 1145 / 0 |
| 3 CREATE REL TABLE | 180000 | 180017 | 2051 / 2049 |
| 4 CREATE_VECTOR_INDEX | 240000 | 240172 | 2052 / 2048 |
| 5 CHECKPOINT | 300000 | 300000 | 2052 / 0 |
| 6 ADD b | 360067 | 360117 | 0 / 0 |

Le premier refus explicite imprimé (:17) est BEGIN/pool plein, situé textuellement
après fin DDL 2 (:14) et avant début DDL 3 (:95). **Aucun timestamp individuel** ;
ce placement n'est pas une mesure précise de sa survenue. Indépendamment, le pic
2051 est un échantillon de la fenêtre `[120009,180000)` : il atteste une pression
dans cet intervalle, sans donner son instant exact ni sa durée.

L'agrégat hors DDL :1184–1187 rapporte 23 conflits, 4 doublons, 79436 refus pool
plein et 17 « No more frame groups can be added to the allocator ». Pas de dates
ni de piles de ces 17 refus : ne pas les absorber dans l'hypothèse du retrait PK.
Les agrégats pendant DDL 3/4/5/migration/ADD b rapportent aussi des refus pool.
ADD b est refusé (:1167). Le SET b final (:1201) lève Binder, propriété absente :
conséquence compatible avec ce refus, pas preuve d'un échec d'annulation.
Aucun message d'annulation inachevée/réouverture obligatoire n'apparaît dans ce
log. Les comptes finaux/relecture après réouverture ne sont pas disponibles,
le test s'arrête sur l'exception ; ne pas transposer les comptes du passage base.

## Limites de provenance et résultat du départage documentaire

base.sh restaure explicitement 65486c02d, vérifie src/test, compile puis lance
le test filtré ; departage.sh lance le binaire déjà construit sans nouvelle
attestation de source. Les documents de session relient ce dernier à 0511 ; le
BuildID historique dirty ne suffit pas seul à en reconstruire le diff exact.
Les scripts conservent argv et répertoire mais ne fixent pas l'intégralité de
l'environnement hérité. XML fournit date locale de lancement et résultat ; ne
pas ajouter un fuseau absent du fichier. suites-codes.txt concerne la suite
précédente, pas une preuve de code retour de chacun de ces deux passages isolés.

trace.log n'est pas copié comme preuve TwentyWriters : il décrit le témoin COPY
et sa pile d'allocation. Les nouvelles copies de logs n'ajoutent aucune pile au
scénario temporel. La correction de sûreté 0511 conserve sa preuve ciblée ; elle
n'est ni annulée ni transformée en preuve de coût global par cette analyse.

**Départage obtenu** : la pression du passage isolé est déjà présente dans la
fenêtre après DDL 2 et avant DDL 3 ; la construction de l'index au DDL 4 ne peut
être sa cause initiale. Les logs ne montrent pas d'annulation inachevée. Ils ne
permettent pas de décider si le coût PK a retenu des transactions, si des volumes
plus grands/allocations moins évictables ont rempli le pool, ou si un autre chemin
est en cause. Les refus d'allocateur constituent une piste distincte sans pile.

## Discriminant minimal encore nécessaire, avant toute capture autorisée

La capture la plus petite utile vise **la première saturation**, entre fin DDL 2
et début DDL 3 : un instant monotone et une phase d'origine pour le premier refus
d'allocation, associés à la même fenêtre aux compteurs cumulés appels/temps PK,
transactions actives/ancienneté et volume validé. Sans temps PK, on ne tranche pas
l'hypothèse de rétention ; sans occupation/volume, on confond vitesse et charge ;
sans phase/exception d'origine et résultat d'annulation, on mélange allocation
ordinaire et faute d'undo. Un snapshot de U n'a pas d'intérêt pour fusionner Doc
(au plus une plage), mais un compteur d'appels reste utile pour attribuer le coût.

Si aucune annulation n'échoue au premier refus, ne pas demander une instrumentation
complète de tous les préfixes pour expliquer ce passage. Si elle échoue, capturer
started/halfDone et transférer la question de quarantaine au ticket diagnostic
hotfix distinct déjà ouvert. Une seule photographie tardive au DDL 4 serait
insuffisante. Recette d'instrumentation, diff, build et runtime restent à relire
séparément ; aucun n'est exécuté ou réservé par ce livrable documentaire.
