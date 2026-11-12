# Sous vingt écrivains, un pool plein laisse une annulation inachevée

- **État** : ouvert (9 novembre 2026, cœur c++)
- **Famille** : moteur
- **Gravité** : indisponibilité — la base refuse toute transaction jusqu'à sa réouverture
- **Atteignable en service** : à établir. Le banc le reproduit seul, sur master avant et après
  29a7bc52a ; un hôte sous un pool de tampons serré et des DDL concurrents y ressemble.
- **Touche rag3weaver** : par le moteur (l'hôte devrait fermer et rouvrir la base)
- **Signalé par** : cœur c++ — banc et reprise décrits dans ce ticket
- **Pour** : cœur c++

## Ce que c'est

`TwentyWriters.DdlEveryMinute` (`test/transaction/concurrence/twenty_writers_scenario_test.cpp`)
rougit seul sous poste lourd, sur 813d7eda8 (master sans le correctif des deux sens) comme sur
52d73a301 (avec). La lib du banc est bâtie de 9bae60786 et suivants, en Release, à luciepc.

Ce que dit son journal (`~/.cache/rag3db-moteur-notes/csr/dep-base.log` à luciepc) :

| Message | Nombre |
|---|---|
| `BEGIN refusé : Buffer manager exception: Unable to allocate memory! The buffer pool is full and no memory could be freed!` | 9 |
| `BEGIN refusé : A rollback of this database could not complete, so it must be closed and reopened before it is used again` | 318 434 |

Le contrôle qui échoue est `index-creation-refuses-no-reader` (8 refus de lecteurs), puis une
exception non rattrapée (`unknown file: Failure`). Les fenêtres « 5 CHECKPOINT », « 6 ADD b
DEFAULT » et « 6 migration annulée » ne voient plus que des refus.

**Lecture** : un pool plein fait échouer une annulation, et la base se met d'elle-même en état
« à rouvrir » (la garde posée pour une annulation inachevée). Tout ce qui suit est refusé. La
question est de savoir si l'annulation peut échouer faute de mémoire, ou si elle devrait réserver
ce qu'il lui faut avant de commencer.

`TwentyWriters.DdlEveryFifteenSeconds` montrait les mêmes « buffer pool is full » dans le banc
entier, mais passe seul des deux côtés.

## Les seuils de temps notés ici

Deux cas de coût rougissent seuls, sur les deux mêmes versions. Ce sont des seuils à revoir avant
d'en faire un défaut :
- `Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64` ;
- `Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64WithRelation` : 7,95 ms
  par ligne à 15 000 lignes pour 5,86 permises, contre 1,93 ms à 1 500.

`TwentyWriters.KilledDuringTheCheckpoint` (enfant mort de SIGSEGV avant le point de reprise) n'a
été vu que dans le banc entier ; il passe seul des deux côtés. À rejouer sous charge, sans ticket.

## Témoin

`TwentyWriters.DdlEveryMinute` lui-même, rouge seul. Un témoin réduit reste à écrire : une
annulation qui échoue faute de pool, puis un BEGIN.

## Pour le fermer

La cause de l'annulation inachevée, le correctif, le témoin vert.

## Reprise du 11 novembre — témoin réduit rouge

Le témoin initial `6e7f11680` ne reproduit pas : `FullBufferRollback.*`, 3/3 verts
à luciepc (3,325 s), journal `~/.cache/rag3db-moteur-notes/annulation/rouge.log`
(le nom du fichier ne dit pas son verdict). La variante CREATE à clé STRING courte
passe aussi (4/4 au total, `cle-string.log`) : ses lignes sont encore locales.

Le COPY atteint au contraire `INSERT_INFO` puis `rollbackPKIndexInsert`. Le cas
existant INT64 et sa variante STRING courte passent (2/2, `copy-string.log`).
Avec 20 000 clés STRING préfixées de 128 caractères `k`, le défaut est reproduit :
`CopyRollbackUnderAFullBuffer.StringKeysRemainUsableAfterRollback`, témoin sauvegardé
sur `annulation-sous-pool-plein`, commit `65486c02d`.

Recette : pool des tests de 64 Mio, point de reprise automatique coupé, COPY dans
Doc sous transaction A ; B insère et valide dans Other, par lots depuis 5 000 avec
division par deux après refus. B valide 155 077 lignes et subit 7 refus.
Le premier ROLLBACK de A échoue explicitement avec « buffer pool is full », le second
et le BEGIN suivant exigent de rouvrir la base, COUNT est refusé. Le fils ferme sans
signal. Le voisin INT64 reste vert. Preuve : luciepc
`~/.cache/rag3db-moteur-notes/annulation/copy-long-string.log` (1 vert, 1 rouge).

Aucun correctif à ce stade. Prochain pas : tracer l’allocation durant le retrait des
clés longues, puis comparer les stratégies des moteurs avant proposition à l’orchestration.

### Cause tracée et suite

La pile de `annulation/trace.log` confirme : `rollbackPKIndexInsert` →
`scanIndexColumns` → `StringVector::reserveString` →
`InMemOverflowBuffer::allocateNewBlock` → `MemoryManager::allocateBuffer` →
`BufferManager::pin`, qui refuse. La copie des clés longues vers le vecteur de
lecture demande le nouveau bloc ; ce n’est pas une preuve d’allocation dans le
retrait de la clé elle-même. La trace reproduit le rouge, voisin INT64 vert.

Proposition transmise à l’orchestration avant tout correctif : lire les clés depuis
leur stockage existant pour l’annulation, sans recopier les chaînes dans un nouveau
tampon ; vérifier les verrous et la durée de vie de la vue empruntée.

Lectures de référence : PostgreSQL réserve 8 Kio dans
[ErrorContext](https://github.com/postgres/postgres/blob/master/src/backend/utils/mmgr/mcxt.c)
pour traiter les erreurs ; cela ne prouve pas une réserve pour tout rollback.
[SQLite](https://sqlite.org/lang_transaction.html) peut annuler toute la transaction
après SQLITE_NOMEM. [DuckDB](https://duckdb.org/docs/2.0/guides/troubleshooting/crashes)
invalide la base après une erreur interne. Ces politiques ne démontrent pas que
notre annulation exige une réserve forfaitaire.

Branche de reprise désormais `cx-coeur-annulation`, publiée, worktree
`~/git_workspaces/cx-coeur-annulation`, même commit `65486c02d`.
L’ancien worktree `rag3db-csr` reste sur disque, détaché, sans suppression.

### Correctif en essai, non validé

Commit `0511dece7` sur `cx-coeur-annulation` : les slots de l’index local détiennent
déjà les clés et leurs décalages. `discardLocalRange(first,count)` retire celles
qui appartiennent à la plage réservée par l’undo, sous verrou exclusif local,
sans lecture des colonnes ni modification du persistant ou des tombstones.
Les clés conservées sont déplacées dans les slots existants, en maintenant le
préfixe compact de chaque chaîne. La comparaison par soustraction évite de faire
déborder `first+count`. Le code impose un déplacement `noexcept`. Cela concerne
ce retrait de clés, pas une preuve que toute annulation du moteur n’alloue rien.

Compilation et tests ciblés en file à luciepc (`annulation/vert.log`) : aucun
verdict sur ce correctif à ce point. Les suites entières et la mesure de coût
restent à jouer ; le ticket reste ouvert.

## Vérifications entières du 11 novembre et travail restant

Porteur : cœur C++. Lot `0511dece7`, branche `cx-coeur-annulation`. Les sources et
tests distants ont été comparés à ce commit : `git diff 0511dece7 -- src test` vide.
Aucun rebuild supplémentaire avant ces suites ; mêmes binaires que les ciblés.

Preuves à luciepc : `~/.cache/rag3db-moteur-notes/annulation/` :
`vert.log`, `transaction.log`, `transaction.xml`, `concurrence.log`,
`concurrence.xml`, `suites.log`, `suites-codes.txt`. Les lanceurs 3809571 puis
1423326 sont terminés.

- Compilation entière réussie ; stockage 8/8 verts ; ciblés FullBuffer/Copy 6/6
  trois fois, avec réouverture et compte des lignes validées inchangé.
- `transaction_test` : code 0, 359 cas joués, 357 verts, 2 outils sautés attendus
  (`VectorIndexCrashReopenProbe.OpenInAFreshProcess`, `WalReader.PrintAFile`),
  1 cas désactivé ; 1 336,857 s.
- `concurrence_test` : code 1, XML 424 cas, 25 rouges ; 3 206,038 s.
  Comparaison aux listes du lot : 16 dans known_red, 5 dans probabilistic,
  4 hors listes. Aucun verdict global vert à ce point.

Les quatre hors listes sont précisément ceux déjà décrits dans ce ticket :
`TwentyWriters.DdlEveryFifteenSeconds`, `TwentyWriters.DdlEveryMinute`,
`Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64` et
`Builds/VectorIndexDeleteCost.DoesNotDependOnIndexSize/SetInBatchesOf64WithRelation`.
Leur présence historique n’est pas une preuve de leur indépendance du correctif
actuel. Travail restant lié à ce même ticket, sans doublon :

1. Rejouer ces quatre cas isolément sur `0511dece7` ; comparer aux symptômes du banc.
2. Si un rouge persiste, rejouer sur la base sans correctif, avec mêmes conditions.
3. Mesurer séparément le coût de plusieurs enregistrements undo, en conservant les
   conditions et les preuves.
4. Départager, puis seulement demander un tour de push et coordonner la lib.

Le ticket reste ouvert. La fenêtre de push de l’architecture est respectée :
aucun tour C++ accordé, aucun code du lot sur master.

### Rejeux isolés terminés — trois rouges à comparer à la base

`annulation/departage.log` et `isole-*.{log,xml}`, à luciepc, même binaire :
- `DdlEveryFifteenSeconds` : vert, code 0, 243,298 s.
- `DdlEveryMinute` : rouge, 376,830 s, 14 lecteurs refusés pendant la création
  d’index ; puis exception de liaison « Cannot find property b for d ».
- `SetInBatchesOf64` : rouge, 8,330 ms/ligne à 15 000 contre 2,137 à 1 500
  (borne 6,274).
- `SetInBatchesOf64WithRelation` : rouge, 8,509 contre 2,421 (borne 6,842).

Le lanceur 2956994 est terminé. La disparition du défaut d’annulation dans le
témoin réduit ne suffit pas à expliquer ces rouges. Suite déjà portée par ce
ticket : comparer les trois cas persistants au parent sans correctif `65486c02d`
(moteur de `6e7f11680`), sous les mêmes conditions, avec sauvegarde des binaires
du correctif avant rebâti. Mesurer les cas de plusieurs COPY annulés des deux
côtés à partir des temps gtest, en distinguant mesure et garantie fonctionnelle.
Aucun nouveau défaut attribué à ce stade, pas de changement des listes de rouges.

## Reprise du 12 novembre — comparaison terminée, verdict encore partiel

Porteur : cœur C++. Le lanceur `3805918` est terminé ; aucun processus à attendre
et aucun rebuild ni rejeu lancé lors de cette reprise. Preuves à luciepc sous
`~/.cache/rag3db-moteur-notes/annulation/` : `base.log`,
`base-TwentyWriters.DdlEveryMinute.{log,xml}`, `base-SetInBatchesOf64.{log,xml}`,
`base-SetInBatchesOf64WithRelation.{log,xml}`, `base-plusieurs-undo.{log,xml}`.

Source contrôlée : HEAD distant `6e7f116806034c7ac6eb3bf2f6309d1c8aaf1b93`,
fichiers du parent sans correctif `65486c02d` ; diff vide pour `src`, `test`,
`extension/vector`, `extension/geo`. Le bâti entier de la base finit en 141 étapes.
Ne pas attribuer les binaires courants au correctif : ce sont désormais ceux de
la base. Les binaires sauvegardés du correctif sont dans `binaires-0511dece7/`
avec SHA256 ; une extension chargée dynamiquement exige aussi son bâti assorti.

### Deux rouges de coût présents des deux côtés

| Cas | Correctif : 15 000 / 1 500 lignes (ms/ligne) | Base (ms/ligne) | Verdict |
|---|---:|---:|---|
| SetInBatchesOf64 | 8,330 / 2,137 | 10,469 / 2,378 | même contrôle rouge des deux côtés |
| SetInBatchesOf64WithRelation | 8,509 / 2,421 | 11,185 / 3,059 | même contrôle rouge des deux côtés |

Ces observations établissent que le correctif n’est pas nécessaire pour reproduire
ces deux rouges. Elles ne mesurent pas une accélération : exécutions uniques et
charge non appariée, pas de borne assouplie ni de liste modifiée.

### DdlEveryMinute : verdict différentiel inconnu

La base est rouge (1 009,647 s), sur l’attente de deux écrivains pendant
`3 CREATE REL TABLE` (785/769 ms, bornes d’environ 177/168 ms). Elle n’a ni refus
« buffer pool is full », ni exception « Cannot find property b », ni annulation
inachevée dans ce journal.

Le correctif est rouge seul (376,830 s) : le CREATE_VECTOR_INDEX et le ADD b sont
refusés faute de pool ; le test continue malgré ce dernier refus et son SET b lève
l’exception de liaison. Le journal contient 1 137 occurrences textuelles de
« buffer pool is full » (ce n’est pas un compte de transactions distinctes),
et aucune de la garde d’annulation inachevée.

Ces symptômes différents interdisent de conclure « rouge identique donc indépendant ».
La pression dépend du scénario chronométré et des écritures effectivement réalisées.
Reste à comparer ces conditions et isoler le déclencheur sur une charge bornée avant
tout rejeu supplémentaire. Cette analyse reste portée par ce ticket ; aucun
nouveau lot ou correctif de DDL autorisé par ce seul constat.

### Plusieurs enregistrements undo

Huit cas `RolledBackCopiesTest.*` et `RolledBackBigCopyTest.*` verts sur la base
(1,438 s) comme dans la suite entière du correctif. Trois COPY annulés puis point
de reprise : base 0,409 s, correctif 0,470 s ; un COPY de 100 000 lignes : base
0,305 s, correctif 0,298 s. Temps gtest incluant création/reprise de base et toute
la recette, pas le coût isolé du retrait de clés ; charge et ordre non contrôlés.
Les garanties fonctionnelles sont couvertes, la mesure du coût isolé reste ouverte.

### Prochain pas et limite du verdict

- Examiner les volumes, durées et états du pool déjà journalisés aux fenêtres DDL
  des deux exécutions ; poser une comparaison bornée seulement après ce diagnostic.
- Définir la mesure isolée du retrait par plage si le risque de coût reste matériel.
- Conserver le témoin long STRING rouge sans correctif et vert avec, les ciblés
  et la suite de transactions verts, mais aucun verdict global ni tour de push
  tant que DdlEveryMinute n’est pas départagé. Ticket ouvert, pas de lib publiée.

### Lecture des fenêtres existantes (12 novembre)

Les journaux établissent une divergence de pression avant le CREATE_VECTOR_INDEX,
pas uniquement une différence de résultat final : pool configuré 2 048 Mio des
deux côtés ; pic base 1 341 Mio, pic correctif 2 052 Mio (les réservations en cours
peuvent dépasser momentanément la limite). Avant `3 CREATE REL TABLE`, le pic de
la fenêtre précédente est 1 176 Mio sur la base contre 2 051 sur le correctif.
Avant `4 CREATE_VECTOR_INDEX` : 1 177 contre 2 052 Mio.

Le CREATE_VECTOR_INDEX de la base réussit en 752 409 ms ; celui du correctif est
refusé après 172 ms. La base atteint les comptes après réouverture Doc 59 927,
Link 447 312, Note 59 683, tous égaux aux comptes attendus. Le correctif s’arrête
sur le SET b avant ce contrôle : aucun compte final comparable à inventer.
Les fenêtres hors DDL comptent 295 723 transactions doc sur la base contre
245 379 sur le correctif, avec 46 contre 57 319 erreurs : ces exécutions ne
portent pas la même charge réalisée. Leurs totaux ne constituent pas une
comparaison appariée du coût du retrait par plage.

Le nouveau retrait parcourt toutes les partitions locales pour chaque
enregistrement d’insertion annulé ; l’ancien relisait les groupes sélectionnés
et cherchait les clés. Le risque de coût sous grand index local mérite donc une
mesure bornée nombre d’entrées × nombre d’enregistrements, distincte des temps
gtest de COPY complets. C’est une hypothèse de coût à tester, pas la cause prouvée
de la divergence de pression. Aucune nouvelle compilation ni exécution de rouge
pour cette lecture ; la comparaison contrôlée reste à préparer dans ce ticket.

### Protocole borné préparé, aucune mesure déclenchée

Voir [le protocole index × plages](../12-11-2026/03-protocole-borne-index-et-annulations.md).
Porteur cœur C++, même lot. Première cellule prévue N=65 536/U=64, R=4 096
constant ; matrice plafonnée à 12 cellules, 72 processus, 15 minutes, un seul
`poste mesure`, pool 512 Mio/RSS 1 Gio. Les modes comparent des entrées identiques,
le retrait par clé étant explicitement une référence basse sans lecture de colonnes.

Les binaires existants ne paramètrent pas N × U : aucune nouvelle mesure présentée
comme réemploi fictif. Prochaine action avant créneau : préparer et relire le
pilote externe minimal, confirmer ses dépendances identiques et la possibilité
de lier l’archive de base sans rebuild moteur. Le protocole distingue plages et
INSERT_INFO ; une mesure d’annulation complète exigerait un second pilote et
un moteur du correctif assorti, à proposer seulement après la première preuve.
Sources/logs terminés conservés ; aucune lib distribuée modifiée.

### Pilote et commandes à relire préparés, sans charge lancée

Voir les trois fichiers `../12-11-2026/outils/cx-mesure-retrait.cpp`,
`cx-commandes-lien-revue.json`, `cx-premiere-cellule-revue.json`.
Première cellule **seulement** : N=65 536, U=64, R=4 096, trois paires alternées.
Les 15 minutes sont un plafond **global**, pas par cellule ; la matrice complète
reste conditionnée à l’utilité du pilote. Le pilote compare des mécanismes sur
index existant, pas les moteurs baseline/fix exacts ni leurs rollbacks.

Relecture de provenance : seule archive disponible = base, SHA256
`40dca10ed39bca8c45769f086e3bfd95fe0f6502876fc12482b33c44ddd662cc`.
La disposition identique d’une classe ne démontre pas la conformité ODR avec son
en-tête modifié. La surcouche sur archive de base proposée précédemment est
retirée : **ne pas compiler/lier** le pilote ainsi. Archive et en-têtes assortis
à `0511dece7` requis, provenance non disponible à ce point.

Les commandes préparées ne compilent que le pilote ; elles ne reconstruisent
aucun moteur copié, et restent `ready_to_compile=false`. Aucun compilateur ou
mesure exécuté. Reste : revue source et provenance, disponibilité d’une archive
assortie à traiter explicitement avec l’orchestration, puis lanceur qui impose
budget global/RSS/groupe et créneau ; pas de mesure autorisée à ce stade.

### Inventaire en lecture seule achevé — archive du correctif non retrouvée

Voir [inventaire et proposition de build privé](../12-11-2026/04-inventaire-et-proposition-archive-privee.md).
Aucune archive ni aucun objet sauvegardé dans les notes de ce lot ; trois
exécutables seulement. Les journaux prouvent la création de l’archive du correctif
puis son remplacement à 22:04 par la base. Produits courants identifiés par
SHA256 et diff source de base ; aucune provenance d’archive `0511dece7` à établir
à partir d’eux. Périmètre de recherche explicite, pas d’absence proclamée sur
les sauvegardes d’autres sessions.

Proposition avant toute demande de créneau : build statique moteur privé sur
source propre `0511dece7`, sans tests/extension/install, cible `rag3db`, puis
manifestes et archive/en-têtes assortis avant pilote. Coût propre non mesuré,
ordre de grandeur de référence 741 objets moteur + 251 tiers ; limites proposées
30 min globales, un lourd `-j8`, build 2 Gio/mémoire groupe 8 Gio, à valider par
plan sec et espace disponible. Ce serait explicitement un build moteur, pas une
compilation de pilote déguisée. Commandes rédigées, aucun worktree/configuration/
build créé ou exécuté. Mesure et lien restent non prêts, différentiel ouvert.

### Build statique privé autorisé sous conditions — 12 novembre

Orchestration autorise préparation et build privé `0511dece7` seulement après
coordination arbre 1b/recherche CI, worktree/build neufs et propres, comparaison
compilateur/définitions, plan sec compatible, sans téléchargement. Un seul lourd,
cible rag3db/-j8, 30 min globales configuration+build, scratch build 2 Gio et
mémoire groupe 8 Gio ; lanceur borné actif avant build. Aucun test, pilote, mesure,
magasin, extension ou lib distribuée. Si les limites ne sont pas fiables :
configuration/plan sec seulement, sans élargissement.

Préflight en lecture seule : luciepc libre (0/4 lourds), cache sur btrfs ;
systemd-run, bwrap et unshare présents, pas de quota/fuse2fs trouvés par command -v.
La disponibilité des outils ne prouve pas leur capacité à imposer les limites.
À vérifier avant le build, et à coordonner dans ce même ticket.

Limites vérifiées par sondes légères : cgroup mémoire 8 Gio/swap 0 ; tmpfs bwrap
8 Mio effectivement borné (ENOSPC au dépassement), namespaces réseau/mount/user
disponibles. Build proposé sur tmpfs privé de cache 2 Gio inclus dans les 8 Gio,
pas dans /tmp. Lanceur préparé sous `12-11-2026/outils/cx-build-archive-privee.py`,
gate après comparaison source/commandes/compilateur et plan sec, unité avec
1740 s + arrêt 5 s pour rester sous 30 min. Aucun moteur compilé à ce point.
Coordination arbre : ciblé terminé, pas de réservation, accepte partage ;
coordination recherche CI encore attendue avant lancement moteur.

Coordination levée par orchestration après confirmation recherche sans build ni
réservation et poste libre. Arbre informé ; lancement permis du préflight puis
du build conditionnel déjà autorisé. Wrapper préparé sous
`12-11-2026/outils/cx-lancer-archive-privee.sh` : poste lourd avant l’unité,
1740 s + 5 s d’arrêt, mémoire 8 Gio/swap 0, tmpfs privé 2 Gio, réseau isolé,
source/référence en lecture seule, attente d’une gate explicite après plan sec.
Aucun pilote, mesure, lib partagée ou magasin dans ce lancement.

### Préflight privé exécuté et refusé — créneau libéré

Unité `cx-coeur-archive-0511-12nov-01.service`, lanceur PID 801185 : terminée
code 2/Result exit-code, MainPID 0, aucun lourd propre encore tenu. Contrôle
poste libre 0/4 après arrêt. Le créneau suivant recherche FTS peut commencer
selon son autorisation C7 ; aucune relance C++ de ce côté.

Source worktree neuf `~/git_workspaces/cx-coeur-archive-0511` sur
`0511dece74f94db6937fb03e6ec45f0c58828c5a`, propre avant/après.
Preuves à luciepc : `annulation/cx-archive-privee-12nov-01/` contient
`unite.log`, `configure.log`, `plan-sec.log`, `revue.json`,
`resume-refus.json`, `limites-resultat.txt`, compile_commands et CMakeCache.
Runtime 1,826 s, CPU 1,766 s, mémoire pic 85 774 336 octets ; limites
cgroup mémoire 8 Gio/swap 0, runtime 29 min + arrêt 5 s, tmpfs privé 2 Gio.
Aucune gate écrite, aucun `build.log` ni manifeste d’archive. Pas de compilation
d’objet moteur (configuration seulement, dont détection CMake du compilateur),
pas de pilote, test, mesure, lib partagée ou magasin. Le tmpfs de build a disparu
avec le namespace ; worktree et preuves conservés, sans retrait de parent.

Diagnostic réduit : 739 unités source identiques dans les deux configurations,
GNU 16.2.1/64bit/ELF identiques. Les 741 désignaient les objets physiques anciens,
pas une différence de deux unités entre ces plans. Le seul écart sur les 739
commandes est `-I<SOURCE>/third_party/spdlog`, présent dans la référence et absent
du privé. CMakeLists.txt:409–413 ajoute ce chemin si BUILD_TESTS,
BUILD_EXTENSION_TESTS ou BUILD_BENCHMARK sont actifs ; la proposition privée
désactive les tests. Le comparateur strict refuse donc ce plan, sans normaliser
ou ignorer automatiquement l’écart.

`BUILD_EXTENSIONS` vide reprend vector;geo à la configuration selon le projet ;
le plan sec de la cible statique ne contient aucune compilation d’extension ni
lib partagée. FETCHCONTENT_FULLY_DISCONNECTED est annoncé inutilisé (chemin
backtraces désactivé), mais le namespace réseau était fermé. Aucun téléchargement.
SHA256 plan sec :
`e9ede0e30f2a9dbc7103e76e059c906771a87a5cdf55652563619961580ea7b4`.

Reste : faire relire l’écart de configuration/inclusion et proposer une nouvelle
configuration assortie sans élargir la garde. Aucun rebuild/second préflight
automatique : priorité au préfixe recherche désormais autorisé. Archive
assortie toujours absente, pilote/mesure toujours non prêts, ticket ouvert.

### Relecture de configuration préparée pendant le slot recherche, sans relance

Voir [comparaison spdlog et option d’alignement](../12-11-2026/05-relecture-ecart-inclusion-spdlog.md).
0 différence de macros sur les mêmes 739 unités ; GNU16.2.1/64bit/ELF identiques.
Lecture `ninja -t deps` de référence : 739 objets VALID, 267 507 lignes de
dépendances, 0 en-tête sous spdlog. Aucun compilateur appelé pour cette lecture.
Le chemin est ajouté par la configuration des tests ou du benchmark ; cela
explique un refus de configuration, pas une régression démontrée du correctif.

Proposition de revue : option dédiée RAG3DB_INCLUDE_SPDLOG_HEADERS ajoutant le
chemin au même endroit, tests/benchmarks OFF, comparateur toujours strict. Le
diff de proposition reste dans les docs, non appliqué ; il modifierait CMake
et demanderait donc une identité de source/autorisation explicitement revue,
incompatible avec l’affirmation « 0511 clean inchangé ». Cette contrainte est
signalée, pas contournée. Aucun mélange d’objets ni nouvelle configuration/build
pendant recherche FTS ; pas de relance automatique, revue attendue.

### Décision de revue de l’écart — préflight révisé préparé

Option CMake dédiée refusée, patch documentaire marqué rejeté/non appliqué.
Source 0511 propre maintenue. Revue autorise seulement l’absence unique de
`-I<SOURCE>/third_party/spdlog`, sans prétendre des commandes identiques ou une
archive historique retrouvée. Identité privée explicite :
`cx-0511-tests-off-without-unused-spdlog-path-v1`. Toute autre différence reste
refusée, mêmes 739 unités/compilateur/ABI requis, gate après plan sec maintenue.

Audit sources privées + dépendances de référence : 3 955 fichiers, aucun absent,
aucune mention spdlog ; 21 lignes has_include relues avec contexte, aucune ne
cherche spdlog. Preuve distante `audit-conditionnels-spdlog.json`, détail dans
`05-relecture-ecart-inclusion-spdlog.md`.

Lanceur révisé pour vérifier aussi les dépendances privées après build : 739
objets VALID et aucun chemin/mention spdlog avant manifeste. Règle de l’exception
vérifiée en Python pur sur les 739 commandes réelles et 12 contre-exemples, plus
une macro parasite sur commande réelle ; aucun moteur/lanceur/pilote exécuté.
Préflight prêt à revue, pas de nouvelle configuration/build, même après le retour
recherche FTS vert ; aucune relance automatique ni lib modifiée.

### Nouveau préflight privé 02 — ATTENTE_REVUE, sans compilation moteur

Autorisation orchestration après revue complète 69ec08161, slot FTS fini/nettoyé.
Unité neuve `cx-coeur-archive-0511-12nov-02.service`, wrapper PID 927681,
MainPID 927714 ; résultats neufs à luciepc
`~/.cache/rag3db-moteur-notes/annulation/cx-archive-privee-12nov-02/`.
Source complète `0511dece74f94db6937fb03e6ec45f0c58828c5a` propre.
Revue fraîche `revue.json` et `resume-revue.json` : ATTENTE_REVUE, mêmes 739
unités, 739 omissions du seul include exact accepté, 0 différence imprévue,
compilateur égal, 0 entrée interdite dans le plan. Audit référence : 739 objets
VALID, aucune dépendance/mention spdlog, aucun fichier manquant.

SHA256 plan sec :
`36fe29eb885914563636036657b91eb8ddafb36c31e9f89343e5268c31afb054`.
SHA256 lanceur :
`5a53c32ed5fd24ad3eeae298f54e0c151741a394563cf7f31aabf5ec3c8a5ee6`.
Limites lues : mémoire 8 Gio/swap 0, scratch tmpfs privé 2 Gio inclus dans ces
8 Gio, RuntimeMax 29 min + arrêt 5 s. Échéance runtime calculée
2026-11-12T11:45:19+01:00 ; l’horloge globale inclut l’attente de revue.
Aucune gate ni build.log lors du partage : C7 doit relire puis écrire/autoriser
la gate, aucune compilation moteur avant. Configuration privée distincte
`cx-0511-tests-off-without-unused-spdlog-path-v1`, pas archive historique.

Gate fraîche C7 observée le 12 novembre à 11:17:56, SHA du plan et commit
identiques au préflight. Aucun contournement ni gate ancienne ; le build
statique privé a démarré sous l’unité 02 existante. Premier point lu 371/989,
pic mémoire 3 233 591 296 octets, deadline runtime inchangée. Le verdict final
reste conditionné à fin du build, audit des dépendances privées et manifeste.

### Build privé 02 terminé — archive prête à revue, aucune mesure

Unité terminée naturellement, code 0, MainPID 0, wrapper927681/PID927714 absents ;
poste relu libre 0/4. Runtime service 226,518 s ; temps configuration/build/
sauvegarde du manifeste 226,467 s, CPU service 986,670 s cumulées, pic mémoire
arrondi du journal 3,5 Gio, swap 0. Aucun dépassement/élargissement des limites.
Les propriétés d’une unité transitoire collectée après arrêt peuvent afficher
les défauts : les limites effectives pendant l’exécution sont celles lues au
préflight, dans revue.json/cgroup et vérifiées par C7 avant la gate.

Archive : luciepc
`~/.cache/rag3db-moteur-notes/annulation/cx-archive-privee-12nov-02/archive-assortie-0511dece7/librag3db.a`
65 588 302 octets, SHA256
`3030fe674e33e2ea0b5b01ed7bdf1a919e538c02e5dfc0b68fff7b50e1d06758`.
Manifeste : `cx-archive-privee-12nov-02/manifest.json`, SHA256
`0049111df769914dbeea66fd0d2d07c6add58a670eb80e797589604d48c80df7`.

Build statique 989 étapes, source privée toujours propre sur
`0511dece74f94db6937fb03e6ec45f0c58828c5a`. Identifiant embarqué :
`rag3db-build-0511dece74f94db6937fb03e6ec45f0c58828c5a-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
La configuration reste nommée séparément
`cx-0511-tests-off-without-unused-spdlog-path-v1`, historical_archive=false,
literal_commands_equal=false ; seule omission d’include revue, aucune autre
différence. Ce n’est pas l’archive historique retrouvée.

Audit **privé avant manifeste** : 739 objets VALID, aucun fichier manquant,
aucune dépendance ni mention spdlog. Le paquet sauvé contient 810 en-têtes
source, 2 générés, 17 archives de dépendances et l’archive moteur : 830 fichiers,
77 984 861 octets. Toutes les tailles et SHA256 ont été recalculées ensuite,
aucun écart ; preuve `verification-empreintes.json`. Les sources du worktree,
logs, compile_commands, CMakeCache, gate et audits restent conservés hors du
tmpfs de build qui a disparu avec le namespace. Pas de nettoyage du parent.

Aucun test, pilote, benchmark, install, extension compilée ou lib partagée
produite par ce lancement ; aucun magasin/service/lib distribuée modifié.
Reste : revue par C7 du manifeste et des commandes de lien assorties avant
toute compilation de pilote/mesure. Le différentiel DdlEveryMinute reste ouvert.

### Recette du pilote remappée au paquet privé accepté, sans exécution

C7 accepte manifeste004911/archive3030 en entrée de préparation, pas de mesure.
Recette `12-11-2026/outils/cx-commandes-lien-revue.json` actualisée : flags
effectifs privés (commande index, compile_commands SHA9a00…), plus aucun
include/flag de tests ou spdlog, aucune archive/header du build de base. Les
18 archives (moteur+17 dépendances) sont toutes dans le paquet02 avec SHA ;
headers moteur/générés attestés, tiers sur le worktree privé 0511 exact.

Détail et provenance dans `12-11-2026/06-revue-lien-pilote-paquet-prive.md`.
Vue include/common des deux générés prévue et empreintée, mais non créée ;
source pilote inchangé, SHA consigné. Aucun staging, compiler, lien, pilote ou
mesure exécuté. ready_to_compile et ready_to_run restent false. Priorité ciblé
CLI arbre ; C7 doit relire ces argv avant d’autoriser une TU puis la première
cellule de trois paires. Aucun moteur recopié/recompilé ni lib distribuée touchée.

### Autorisation conditionnelle pilote et première cellule — 12 novembre

C7 a relu e78/recetteb4d424 et autorise staging vérifié puis une TU/lien sous
poste lourd, 180 s globales/mémoire8Gio, contrôle ELF/provenance sans reconstruire
de moteur. Si ces contrôles passent seulement : première cellule64k×64/R4096,
3paires alternées sous poste mesure exclusif, 900s globales/60s enfant/mémoire1Gio
pool512Mio. Premier échec stop sans retry ; pas de matrice/DDL/attribution moteur.
Arbre finit ses ciblés, poste lu libre avant préparation, coordination prévue.

Contrôleur préparé `12-11-2026/outils/cx-controler-pilote.py` et wrapper borné :
unités distinctes MemoryMax8G/1G, swap0, RuntimeMax170/890s+arrêt2s (sous plafonds),
limites cgroup réelles vérifiées avant action, namespace réseau fermé et entrées
en lecture seule. Stage 830fichiers vérifiés, copie des2générés, compile/lien
uniques puis readelf sans exécuter le pilote. Mesure ultérieure : timeout enfant
conservateur57s+2s, groupe propre tué au timeout et arrêt sans retry. Validation
identité des entrées/survivants, des64plages et des6volumes avant résultat final.
Aucun lancement encore à ce point de préparation.

### Premier arrêt pilote : compilation/lien passent, contrôle ELF refusé

Unité `cx-coeur-pilote-compile-12nov-01`, wrapper1043954, terminée code2,
MainPID0 et PID absent ; poste relu0/4, aucun lourd propre conservé.
Staging : 830fichiers du paquet vérifiés, 2générés copiés et vérifiés ;
source pilote SHAfdc3e90f… et recetteb4d424… inchangés. UNE TU compilée
code0 puis lien code0 ; aucun moteur reconstruit.

Premier refus dans le contrôleur ELF : DT_NEEDED contient les quatre librairies
prévues et `ld-linux-x86-64.so.2`, absent de sa liste. ELF sans RPATH/RUNPATH.
Le contrôleur s’est arrêté comme demandé, sans retry : aucune exécution du
pilote, aucun poste mesure ni paire lancée, aucun résultat de mesure inventé.
Preuves luciepc `annulation/cx-pilote-retrait-0511dece7/` :
`unite-compile.log`, `compile/link-result.json`, `compile/link.stdout/stderr`,
`elf-header.stdout`, `elf-dynamic.stdout`, `premier-echec.json`,
`refus-elf-provenance.json`, `staging-verifie.json`.

Binaire produit mais NON EXÉCUTÉ : 25 610 008 octets, SHA256
`0c868053afea43df3cb2eb5fe8e77f597701b8ab4eb2a0c403ca854ef64cf704`.
Objet pilote SHA256
`f964fbacbe8c16f3a72c7325d18eb318041613cde73587101a9c90fb703dd0ef`.
Le manifeste de compilation finale n’a pas été émis car le contrôle ELF a
refusé ; ne pas le remplacer par une attestation réussie. Relecture de la
provenance du chargeur à faire avant toute nouvelle autorisation.

Diagnostic lecture seule complété dans
[07-premier-arret-controle-elf](../12-11-2026/07-premier-arret-controle-elf.md).
Le chargeur `/lib64/ld-linux-x86-64.so.2` se résout en `/usr/lib/ld-linux-x86-64.so.2`,
appartient à glibc2.44+r50+g1848099f063e-3 (base pacman), SHA256
`2a950d46575efa0210d22440988f5a6b4507ebddc6c86d9a42e6e7ef6f01463e`.
Le binaire de tests de référence a le même NEEDED. La liste du contrôleur
omettait donc un composant système ; elle n’a pas été modifiée/rejouée.
Compilation/lien effectifs en 1TU passent, mais attestation finale ELF refusée
à ce point ; pas de résultat de cellule ni d’égalité entrées/survivants à annoncer.
Reste revue C7 du chargeur et d’une reprise explicite sur le binaire existant,
sans nouvelle compilation ni relink automatique. Aucun lourd réservé.

### Reprise autorisée après revue du chargeur, sans recompilation

C7 a relu la provenance et accepte exactement le cinquième NEEDED système
ld-linux-x86-64.so.2, sans lib projet/RPATH ni exception générale. Requalifier
en lecture le même binaire0c868053, garder premier refus/préparation ; aucune
recompilation ou relink. Première cellule de3paires réautorisée seulement après
fin du ciblé CLI arbre, sous poste mesure exclusif/global900s/60s enfant/1Gio/
pool512Mio. Vérifier binaire/source/limites, premier échec mesure stop.
Poste lu libre0/4 au démarrage de la relecture ; confirmation de fin arbre
demandée avant mesure, aucun lancement de mesure sur ce seul constat.

Requalification metadata terminée : même binaire0c868053/sourcefdc3e90f, cinq
NEEDED exacts, interpréteur/chargeur SHA accepté, aucun RPATH ; ELF jamais
exécuté lors de cette étape. `elf-requalification.json` et reçu
`compile-manifest.json` explicitent reprise en lecture, sans rebuild ; premier
refus et contrôleur initial SHA119db4c8… conservés inchangés.
Arbre confirme FIN2684/11:49:12 et aucun Cargo/lourd réservé ; C7 demande
l’acquisition immédiate de la porte existante poste mesure, sans autre attente.
Début de la première tentative de mesure annoncée : unité neuve bornée1G/
890s+arrêt2s après acquisition, contrôleur initial forcé sur phase measure
uniquement, exact5 vérifié par enveloppe, premier échec mesure distinct pour
ne pas écraser le refus de préparation. Pas de compilation, matrice ni DDL.

### Première cellule terminée code0 — poste rendu, pas de matrice

Unité cx-coeur-pilote-mesure-12nov-01 terminée code0, MainPID0, poste relu0/4.
Runtime service0,652s, CPU0,412s, pic49,6M/swap0 ; contrôleur global0,558s.
Limite mémoire réelle1 073 741 824 octets/swap0 lue, unité890s+2 après
acquisition poste mesure exclusive, enfants57s+2 sous60s. Six processus
uniques, aucun retry, 384plages brutes, bin/source inchangés.

Toutes les entrées et tous les survivants ont les mêmes empreintes dans les
3paires, validés/réinsérés par le pilote. Médianes retrait : clés préconnues
0.296892ms, plages 26.442622ms, ratio 89.06. Ce sont les mécanismes dans
UNE cellule N65 536/U64/R4096, pas les moteurs complets baseline/fix ni une
preuve de croissance selon N/U ou de cause de la pression DDL.
Les mêmes 7 647 744 octets de pool sont observés aux frontières de tous les
appels ; pas une preuve d’absence d’allocation transitoire.

Preuves distantes `annulation/cx-pilote-retrait-0511dece7/` :
`cellule-resultats.json`, `cellule-1..6.stdout`, `cellule-1..6-result.json`,
`measure-limites.json`, `unite-mesure.log`. Export et analyse à compléter ;
source/binaire/provenance et refus initial conservés. Aucun nouveau slot réservé.

Résultats et bruts de la première cellule exportés dans
`12-11-2026/08-resultat-premiere-cellule-retrait.md` et
`12-11-2026/preuves/cellule-64k-64/` : 6sorties natives, argv/codes/empreintes,
bornes, unité et premier refus conservé. Médianes0,296892/26,442622ms ;
interprétation limitée aux mécanismes et à cette cellule, pas au moteur global.

### Quatre points supplémentaires autorisés, périmètre fermé

Après revue indépendante de la première cellule, C7 autorise seulement :
(16384,64), (262144,64), (65536,8), (65536,256), R4096 fixe,
3paires alternées chacun, même source/binaire/limites, maximum24processus.
Une acquisition poste mesure, global900s après acquisition, enfant60s,
mémoire1G/pool512M ; premier échec stop. Ni matrice complète ni DDL/codeproduit.
Arbre confirme fin de ses8CLI et aucune réservation. Après ces4points : STOP
mesures et analyse des latences/dispersion/validations par cellule.

### Quatre points terminés code0 — STOP mesures appliqué

Unité cx-coeur-quatre-points-12nov-01 inactive/MainPID0, code0, poste relu0/4.
24processus uniques, 4cellules×3paires, aucune relance ; contrôleur2,831s,
service2,874s/CPU2,395s/pic153,5M/swap0. Source et binaire montés RO,
empreintes début/fin identiques, premier refus préservé. Limite réelle1G
vérifiée, borne890s+2 après acquisition poste mesure, enfants57s+2.

Médianes brutes de retrait (pas rollback moteur) :
- N16384/U64 : clés0.282405ms, plages6.920786ms
- N262144/U64 : clés0.324703ms, plages105.293110ms
- N65536/U8 : clés0.294349ms, plages3.848231ms
- N65536/U256 : clés0.300968ms, plages102.217416ms

Validation par cellule : mêmes entrées/survivants/volumes et réinsertion
vérifiée dans les6processus ; à N65536 signatures égales à l’ancre U64.
Preuves `annulation/cx-pilote-retrait-0511dece7/quatre-points-12nov-01/`
resultats.json et point-*-process-*.stdout ; bruts/dispersion à exporter.
Mesures arrêtées après ces4points, aucun job/slot réservé ; analyse seulement.

Bruts/dispersion des4points exportés dans
`12-11-2026/09-resultats-quatre-points-et-analyse.md` et
`12-11-2026/preuves/quatre-points/` :24sorties,2 352plages, codes/argv/SHA,
signatures etMAD/min/max. Avec ancre précédente, N×4 donne tempsplages×3,82
puis×3,98 ; U×8 donne×6,87 puisU×4 donne×3,87, àR4096constant. Compatible
avec le coût procheN×U du mécanisme, pas rollbackcomplet ni causeDDL.

Revue C7 des24validations reçue, aucune mesure supplémentaire. Proposition
lecture seule `12-11-2026/10-proposition-regroupement-undo.md` : regrouper
uniquement des INSERT_INFO adjacents/samehandler/groupe dans un tampon,
opt-in sémantique seulement après équivalence de tous les effets ; aucun
registre de plages alloué au rollback et commit inchangé. Limites/barrières,
réservations partielles, survivants et index secondaires à prouver. Alternative
ensemble de plages préparé à l’écriture seulement si la fragmentation impose
un traitement plus large. Note : reverseIterate alloue déjà un vector système,
donc aucune promesse de zéro-allocation globale aujourd’hui ; le prochain
regroupement n’ajouterait rien et le retrait reste sans demande au pool.
Aucun code moteur modifié, build/mesure/test natif nouveau exécuté.

## Lecture du scénario après revue du plan 10

[Adjacence des undo du scénario vingt écrivains](../12-11-2026/11-adjacence-undo-du-scenario-vingt-ecrivains.md) :
chaque transaction ordinaire crée au plus un Doc ou une Note ; aucune série de
plages de cette table à fusionner. Les relations ont d'autres handlers et les
journaux conservés ne donnent pas leur séquence typée par tampon. A non retenu
comme prochain correctif du scénario sur le seul gain micro N×U. Lecture seule,
aucun code moteur, build ou nouveau run. Ticket toujours ouvert.

Revue croisée intégrée au plan 10 : opt-in NodeTable non démontré (contrat des
index secondaires, ordre des blocs et préfixe des effets en cas d'exception,
réservations partielles et bornes). Aucun opt-in recommandé ni implémenté.

Complément au plan 10 : l'identité du préfixe d'effets après exception n'est pas
un contrat démontré. Une quarantaine avec récupération sûre pourrait suffire,
mais le refus des nouveaux BEGIN/checkpoints ne démontre pas la sûreté pour les
transactions déjà ouvertes ; les garanties de versions des lecteurs restent
nécessaires. Lecture seule, sans nouveau défaut observé ni changement moteur.

Limite précisée après revue : la garde rollbackFailed du checkpoint public n'est
pas rappelée dans checkpointNoLock, que commit peut joindre directement ou par
writeRequestedCheckpointIfLastNoLock pour une transaction déjà ouverte. Graphe
de contrôle relu, aucun incident reproduit ; ne pas présenter la quarantaine
comme globale ou comme preuve suffisante des nouveaux préfixes d'effets.

## Recette de diagnostic du rouge restant, sans exécution

[Recette DdlEveryMinute](../12-11-2026/12-recette-diagnostic-ddl-minute.md) :
pression déjà à 2051 Mio avant le troisième DDL, donc antérieure au CREATE_VECTOR_INDEX.
Le microbanc ne départage pas temps PK, débit/allocations antérieurs et autre
échec d'annulation. Proposition de chronologie documentaire puis capture privée
bornée à relire ; aucun nouveau run, build ou instrument moteur effectué.

## Phases documentaires 1–4 réalisées

[Chronologies conservées et compteur exact](../12-11-2026/13-chronologies-ddl-minute-conservees.md),
logs/XML/scripts intégraux exportés avec empreintes. Pics échantillonnés toutes
les 100 ms sur fenêtres semi-ouvertes ; zéro peut signifier aucun échantillon.
2051 Mio appartient à [120009,180000) sur l'isolé, pas à un instant précis avant
DDL3. Aucun message d'annulation inachevée dans les deux logs ; 17 refus de groupes
de frames restent sans pile. Discriminant proposé : première saturation avec
phase, temps PK cumulé, transactions retenues et volumes dans la même fenêtre.
Aucune capture, instrumentation, compilation ou réservation effectuée.

## Diff privé de diagnostic préparé, sans exécution

[Capture bornée proposée](../12-11-2026/14-diff-prive-capture-premiere-saturation.md) :
source 01185695f sur cx-coeur-diagnostic-saturation, worktree distinct depuis0511,
diff exact et recette build/runtime assortie publiés pour revue. Première capture
pool/groupes de frames, phase scalaire rollback/PK/site d'allocation, compteurs PK,
transactions publiées et volumes transférés/itérations. Absences explicites : âge
mural, propriétaire mémoire, volumes durables. Aucun build/run/réservation et aucune
lib distribuée modifiée. Sortie seulement après joins : timeout sans capture finale
reste inconclusif, sans relance. Correction de sûreté et coût toujours distincts.

### Delta avant revue du build

[Publication et vidage](../12-11-2026/15-delta-publication-et-vidage-capture.md),
source6bfba5b4c poussée : armed lit acquire ; fichier privé neuf ouvert avant acteurs,
vidage first_saved/samples depuis sampler horsrollback, bilan final séparé. Timeout
conserve une preuve partielle, jamais capture globale réussie sans status valide.
pk_completed_ns exclut les appels encore actifs ; pk_active>0 interdit d'écarter
le coût PK sur ce cumul seul. Recette/diff actualisés ; aucun CMake/build/run/poste.
