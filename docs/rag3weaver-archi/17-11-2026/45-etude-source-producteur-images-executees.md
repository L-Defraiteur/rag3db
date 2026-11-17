# Images exécutées : barrière disponible, couverture complète non établie

Étude SOURCE SEULE attribuée C7 862aa/8b1bdcb0e5, après lecture du
[ticket commun](../../orchestration/tickets/2026-11-17-les-images-executees-manquent-d-un-producteur-de-preuves-commun.md).
Contrat consommateur TTY961c873bb9d74408aa892d7d6799737047882650/P01–P20,
relues comme spécifications documentaires, pas comme producteur livré.
Aucun code, prototype, test, traceur, proc d'une session, signal, build, SDK,
cache/target/service ou réservation de chemin n'a été exécuté/modifié ici.
Lectures : sources Git et documentation Linux primaire publique uniquement.

**Proposition minimale : un runner commun de confiance pour l'image d'ENTRÉE
seulement, conditionnel et non livré.** Il lierait l'objet ELF choisi, sa copie
stable et son hash au transfert d'exécution réussi d'une racine déclarée, puis
au wait de ce processus. Cette garantie constructive limitée ne couvre pas ses
futurs exec ou descendants ; elle ne satisfait donc pas automatiquement P17–P20.
C7 doit arbitrer explicitement cette portée et la surface transitive avant code.

L'autre candidat, collecteur ptrace événementiel persistant, offre une barrière
sur exec réussi mais sa couverture exhaustive n'est pas établie :
CLONE_UNTRACED et détachement interdisent les raccourcis décrits ci-dessous.
**STOP pour l'admission et l'implantation automatique des deux candidats.**
Ce STOP porte sur les garanties non démontrées, pas sur une impossibilité
universelle de Linux. TTY reste consommateur et CP propriétaire de la seule
publication ; aucun producteur physique n'est attribué par cette étude.

## 1. Ce qui existe réellement dans le parcours

Références projet : APPf1e8a8d502614d10f7cdb40eaea330f08b5d3f37,
canonSHA2150fcd847b1df7de04908d357a0c78abc05e50ce599e2793ff08ab48108825d,
CommonSHA6439fdd8b52c85fa4e74af0def2c667ef0bedae9b814cbbfd48e49946c4f74ea.
APP désigne le commit projet ; canonSHA et CommonSHA identifient des fichiers,
pas des commits Linux ou des captures.

| Source relue | Ce qu'elle fournit | Ce qu'elle ne fournit pas |
|---|---|---|
|tools/poste/passe_epinglee.sh,75–130|Appels e2e/filets, F8, touchsrc/lib.rs, Cargo lib/NAV et journaux déclarés|Ni choix de copie physique figée pour chaque enfant ni image au vrai exec|
|run_e2e.sh,jouer_les_suites,924–1058|CargoJSON e2e→chemin bin, puis exécutions directes filets/suites ; libs par Cargo ; logs par racine|Le JSON temporaire est supprimé avant les tests ; un wrapper PATHcargo ne couvre pas les exec directs|
|scripts/confiner_validation.py,194–253/270–370|Subreaper, enfantPopen/dbus, PID/start/cgroup et relevé25ms, stop/reap/END|Pas de protocole de génération d'image ; ni TracerPid0 obligatoire ni preuve de chaque exec bref|
|Archi229b,runner/observe/qualify|Catalogues/copiestargets, observations cohérentes échantillonnées, hashes finaux|Pas exhaustivité ni preuve contre imageB puis cheminA restauré entre observations|
|CP C9constant/c2d intermédiaire|Relais de fichiers/ELFmap/ACK et garde de phase ; Window en préparation|Une copie acquittée ne devient pas un fait d'exécution ; aucun producteur physique générique|

Le constructeur de commande peut attester sa propre intention, ses argv et le
rôle déclaré ; cela ne doit pas être intitulé «image observée». Le vrai runner
actuel ne contrôle pas seul tous les exec transitifs de Cargo, des suites et
de leurs descendants. Les neuf positions racines TTY restent une projection,
pas un nombre de processus, d'images ou d'auxiliaires complet.

## 2. Objet choisi et image constatée sont deux preuves différentes

[execveat](https://man7.org/linux/man-pages/man2/execveat.2.html) permet de viser
un fichier ouvert par descripteur. Ce serait un lien constructif à l'objet
choisi pour les commandes maîtrisées par un runner ; cela ne constate pas à
lui seul son succès ni les exec ultérieurs arbitraires du canon. Passer par un
FD puis annoncer l'intention comme observation ne ferme pas cette lacune.

[fexecve](https://man7.org/linux/man-pages/man3/fexecve.3.html) n'immobilise pas
magiquement les bytes d'un fichier ouvert ; conservation/copie et stabilité
restent à prouver. Remplacer silencieusement les exec du canon par un wrapper,
modifier Cargo/PATH/LD ou ses entrées transitives changerait le parcours à faire
approuver. Aucune redirection de ce type n'est proposée comme déjà compatible.

Une observation au stop d'exec réussi pourrait ouvrir l'exécutable du processus,
plutôt que rouvrir son ancien chemin T. Le manuel
[proc_pid_exe](https://man7.org/linux/man-pages/man5/proc_pid_exe.5.html)
documente l'ouverture de l'exécutable, y compris après unlink, avec contrôles
d'accès et cas d'indisponibilité. Le nom affiché n'est pas une identité de contenu.
Lecture refusée/disparition avant copie resteraient un refus, pas une copie A
retrouvée ailleurs après disparition de B.

Pour les ELF ordinaires du modèle Linux lu, begin_new_exec installe le fichier
exécutable du nouveau mm ; son rôle et l'exclusion d'écriture sont visibles dans
[fs/exec.c v6.18](https://github.com/torvalds/linux/blob/v6.18/fs/exec.c#L1051-L1089)
et [set_mm_exe_file](https://github.com/torvalds/linux/blob/v6.18/kernel/fork.c#L1111-L1147).
Cela motive une copie depuis ce fichier pendant la barrière, hashée et conservée
avant continuation. Ce n'est pas une preuve des pages RAM/JIT ni des libsSDK,
et ce modèle ne certifie pas un stockage distant/mutable non qualifié.
Linuxv6.18 est ici une référence source explicite, PAS le noyau de la machine
relevé : aucun uname/sysctl/probe de cette étude ne l'établit.

## 3. Proposition minimale : runner commun, racines explicitement déclarées

Le raccord proposé serait unique pour e2e, lib, NAV et TTY : Common conserverait
l'autorité et les bornes ; le runner de confiance choisirait l'objet préautorisé
et publierait ses faits par CP. Aucun nouveau reçu, API ou chemin n'est créé ici.

| Étape conditionnelle | Preuve exigée et limite |
|---|---|
|Sélection|Source/profil/catalogue/argv préautorisés, objet ouvert et copie hashée liés ; aucune sélection depuis l'actual pour sauver B|
|Stabilité|Même objet et mêmes bytes entre hash et exec ; un FD seul ferme la substitution de chemin, pas la modification du contenu|
|Handoff|Succès d'entrée à établir au vrai raccord ; invoquer fexecve ne prouve pas son succès. EOF d'un canal close-on-exec seul peut aussi résulter d'une mort et ne suffit pas|
|Observation éventuelle|Une barrière bornée sur le premier exec réussi pourrait confirmer l'entrée puis libérer avant le corps ; compatibilité et mécanisme NON QUALIFIÉS|
|Fin|Wait du vrai PID/start et RAW liés au lancement ; le statut final appartient au processus, pas automatiquement à son image initiale après remplacement|

Une conservation de bytes indépendante pourrait être étudiée, par exemple les
scellés documentés par [memfd_create](https://man7.org/linux/man-pages/man2/memfd_create.2.html)
et [F_GET_SEALS](https://man7.org/linux/man-pages/man2/F_GET_SEALS.2const.html).
Ce n'est PAS un choix arrêté ni une primitive qualifiée : politique d'exécution,
coût mémoire, nom de l'exécutable et recherche de ressources peuvent changer.
Le chargeur et [$ORIGIN](https://man7.org/linux/man-pages/man8/ld.so.8.html)
imposent une revue du contexte ; copier ailleurs n'est pas présumé transparent.
Aucun objet, FD ou scellé n'a été créé pour cette étude.

La garantie proposée serait «entrée de cette racine dans cet objet attesté»,
pas «chaque image de toute la passe». Après libération, A→B ou enfant bref
restent hors observation. Un contrat excluant ces transitions doit être justifié
et accepté ; une absence de preuve n'est pas une exclusion démontrée. P17–P20
restent donc des obligations à arbitrer, pas des juges réputés satisfaits.

### Surface transitive proposée, aucun changement effectué

Le canon2150 peut rester identique octet pour octet, mais son parcours changerait.
Dans run_e2e.sh relu, **quatre sites directs** appellent le binaire e2e : filets
ligne970, inventaire --list ligne989 et suites lignes998/1001. Les deux derniers
passent par confined, aujourd'hui simple appel "$@" ligne149 ; changer seulement
cette fonction laisserait les deux premiers hors raccord. Ces quatre sites,
plus le point commun, constituent la surface source proposée à examiner ; ce
n'est pas un nombre d'invocations ou de processus observés.

Pour les tests lib/NAV/TTY lancés par Cargo, la configuration officielle
[target runner](https://doc.rust-lang.org/cargo/reference/config.html#targettuplerunner)
prévoit un runner recevant l'exécutable pour cargo test/run/bench et une variable
CARGO_TARGET_<tuple>_RUNNER. C'est un raccord possible vers LE MÊME runner,
pas un shim PATH Cargo. Tuple hôte effectif, propagation par garde/recettes,
portée sur les commandes et identité des arguments restent à qualifier ; ce
mécanisme ne couvre ni les appels e2e directs ni les enfants des tests. Aucun
environnement/configuration/argument n'est installé ou modifié ici. La garde
refusant --config n'est pas contournée.

Si C7 exige le parcours transitif actuel inchangé, cette variante ne peut pas
revendiquer les lancements qu'elle ne contrôle pas : HOLD. S'il accepte cette
surface, elle exige une revue explicite canon/guard/Common et du consommateur,
pas une neutralité déduite du SHA2150 conservé.

### Comparaison avec le collecteur persistant

[ptrace](https://man7.org/linux/man-pages/man2/ptrace.2.html) fournit des arrêts
d'exec et de création par thread ; exec nonleader peut remapper le TID. Une
frontière sur exec réussi peut précéder userland. Détacher perd les faits futurs ;
le wait final ne décrit pas toutes les générations d'image. Un parent producteur
avec propriétaire unique des waits serait nécessaire pour ne pas confondre les
stops avec la terminaison dans Common. Ce raccord n'existe pas dans Common6439.

TTY précise P19 : aucun B promu sans SUCCESS observé, pas comptage exhaustif des
tentatives échouées. Sans observation de tentative, celle-ci reste UNKNOWN ;
l'absence de SUCCESS ne prouve pas un échec particulier. Ce minimum n'impose pas
à lui seul le traçage de tous les syscall. La fermeture des descendants reste
un problème distinct, exposé ci-dessous.

## 4. Trou de fermeture : TRACEEXEC + créations ne suffisent pas seuls

Le [manuel clone](https://man7.org/linux/man-pages/man2/clone.2.html)
explicite CLONE_UNTRACED. Le chemin
[kernel_clone v6.18](https://github.com/torvalds/linux/blob/v6.18/kernel/fork.c#L2440-L2457)
ne publie pas l'événement ptrace de création lorsque ce flag est demandé.
Il serait donc faux de déduire «tous les descendants couverts» des seuls
événements activés. Cette étude ne dit ni que les outils du lot utilisent ce
flag, ni qu'ils ne l'utilisent pas : aucune absence n'a été démontrée.

Pour fermer cette voie, il faudrait une exclusion établie dans le contrat des
créations OU une frontière observant/refusant les créations hors couverture
avant de les laisser échapper. Le mode syscall décrit par ptrace accroît les
arrêts au volume des appels, pas seulement des images. Un filtre seccomp sélectif
serait une modification de surface/sandbox, pas une optimisation autorisée ici.
Aucun de ces raccords n'existe dans les primitives relues. Un simple relevé
périodique des enfants ne rattrape pas de manière démontrée un enfant bref.
**STOP couverture exhaustive à ce stade**, sans présenter un modèle événementiel
partiel comme un producteur complet ou exclure toute autre solution future.

L'absence d'un trou connu ne suffirait pas encore : tables de threads/exec,
versions/ABI, événements horsordre, héritage, fin du collecteur et journal tronqué
resteraient des cas de fermeture à prouver. Les workers kernel et rôles exclus
de l'image userland devraient être explicitement bornés, pas oubliés du compte.

## 5. Détachement, isolation et effet sur le parcours

CP a confirmé à la lecture : Common6439 n'impose PAS TracerPid0. Nous ne créons
pas cette exigence pour justifier un STOP artificiel. En revanche, conserver
une sonde pendant les tests n'est pas neutre : signaux/stops/parentage/temps et
compatibilité des outils peuvent changer. La famille étudiée ne démontre pas
simultanément suivi de tous futurs exec/descendants ET libération avant les tests.
Un détachement ne doit pas être utilisé pour fabriquer un succès de couverture.

Les droits du montage sont inconnus ici. [Yama](https://docs.kernel.org/admin-guide/LSM/Yama.html)
peut refuser même TRACEME ; [Landlock](https://docs.kernel.org/userspace-api/landlock.html#ptrace-restrictions)
impose sa relation de domaines. MêmeUID/parent n'est pas un GO. Aucune politique,
capacité, sysctl, espace de noms ou sandbox n'est assoupli ou inspecté sur hôte.
Les effets documentés de [execve](https://man7.org/linux/man-pages/man2/execve.2.html)
sur credentials en présence d'un traceur interdisent d'affirmer la neutralité
pour une image privilégiée. Si un rôle exige un tel comportement, STOP ou
nouvelle décision de périmètre, pas suppression d'un garde pour faire passer.

Les sources du navigateur du dépôt contiennent déjà --no-sandbox ; cela ne
prouve ni que tout auxiliaire est compatible, ni qu'une option puisse être
ajoutée maintenant. Les autres mécanismes n'offrent pas un raccourci attesté :
[fanotify non privilégié](https://man7.org/linux/man-pages/man2/fanotify_init.2.html)
est restreint, et un événement d'ouverture n'est pas un succès d'exec ; un
[ringbuffer BPF](https://docs.kernel.org/bpf/ringbuf.html) peut manquer de place.
Aucun privilège ni observateur alternatif n'est introduit pour contourner le STOP.

## 6. Catalogue, occurrence et changements A/B

Le profil minimal proposé ne recataloguerait PAS opportunément dans le canon.
Il n'accepterait une image de test que liée à un catalogue/source/profil et ACK
DÉJÀ autorisés. B inconnu au stop d'exec : faits B conservés et refus avant
corps de test, pas un nouveau catalogue B choisi pour sauver la passe. Un B
préautorisé/publié pourrait avoir sa propre occurrence ; aucune cardinalité
ou étape ne serait ajoutée depuis l'actual. A→A peut réutiliser le contenu,
jamais l'ancien PID ni la génération précédente.

La copie de B ouverte depuis l'image devrait rester la preuve si le chemin
redevient A. Le post appartient à son occurrence AVANT rebuild suivant, pas
au chemin final de la passe. Pour les invocations directes e2e et les libs
lancées par Cargo, la couverture doit passer au même point d'exécution commun,
avec les mêmes filtres/argv/canon ; pas deux wrappers métier indépendants.
Les rôles auxiliaires/catalogues et images successives doivent être prévus
ou refusés ; quatre publications Arched8ELF ne donnent pas un compte d'images.

RAW/rc doivent être liés au vrai lancement et au sink déclaré, conservés avant
jugement. Un fichier partagé par plusieurs descendants n'atteste pas à lui seul
le PID écrivain de chaque octet ; ne pas transformer ce lien de runner en
observation syscall. Framing/ordre des résultats et inventaires du consommateur
restent nécessaires ; mélange ou attribution ambiguë doit refuser. Le wait du
processus après A→B ne devient pas le rc d'une suite supposée achevée sous A.

## 7. Le SDK reste une preuve distincte

L'arrêt d'exec précède aussi le travail userland de l'interpréteur ELF dynamique ;
il ne fournit pas une preuve que librag3db/les extensions attendues sont chargées
à l'instant métier. Une copie de l'ELF ne ferme pas leurs résolutions/chargements.
Le porteur SDK doit déclarer le point d'observation au PID/génération réels,
les champs autorisés et la portée sur les chargements ultérieurs. Pas d'export
d'environnement complet, de secrets ou de preuve23 empruntée à229b.

Le bracket env/maps229b est une préparation échantillonnée, pas cette garantie.
Un GDB additionnel ne peut pas simplement s'attacher à un tracee déjà détenu
par le collecteur ; partage de la frontière ou observation compatible restent
à établir. Ne pas affirmer qu'un seul événement d'exec remplace le profilSDK.

## 8. Bornes, coûts et premières causes

Aucune mesure ni valeur d'overhead n'est fournie. Les dépenses dépendent du
nombre d'images/threads/signaux et des bytes à hasher/copier. Une FD conservée
n'est pas une preuve universelle d'immuabilité entre deux générations ; un
cache par chemin/inode/mtime seul ne doit pas supprimer la preuve de B.
Déduplication de stockage seulement après égalité de contenu démontrée.

| Poste de coût/perturbation | État de l'étude |
|---|---|
|Stops exec/créations/signaux/sorties, ordonnanceur et waits|Inévitables dans cette famille ; coût UNKNOWN|
|Hash/copieur de la bonne image, gros debugELF, fsync et transfert|Temps/disque/mémoire non mesurés ; pas de prolongation d'END|
|Fermeture CLONE_UNTRACED/ABI et contrôle des créations|Non démontrés ; peut accroître fortement la surface observée|
|Coexistence Common/subreaper/cgroups/landlock/Chrome/SDK|Compatibilité NON QUALIFIÉE|
|Reap/quiet et publication après timeout ou mort du collecteur|À prouver sous la même réserve ; UNKNOWN ne devient pas vide|

Le producteur devrait rester dans le Common original. Queue/ARM/copie/ACK ne
repartent jamais à zéro ; chaque arrêt attendrait MIN(reste frais,cap). Le
collecteur lui-même doit finir/récolter sous la réserve, avec première erreur
immuable avant cleanup. Mort du collecteur/perte/refus d'accès : aucune occurrence
positive posthume ; Common conserve arrêt attribué/ownedquiet. Un watchdog ne
prouve pas que toute tâche bloquée a disparu : état inconnu reste refusé.
Aucun temps monotonic absolu n'est comparé entre hôtes ; CP publie seulement
les faits conservés, pas un pouvoir de rallonger lease/budget ni de lancer B.

## 9. Arbitrage nécessaire avant code

| Choix soumis à C7 | Conséquence explicite |
|---|---|
|Runner de confiance, image d'entrée des racines seulement|Surface transitive section3 à accepter ; P17–P20 à réviser/borner explicitement, pas de conformité20 déduite|
|Couverture de chaque exec/descendant maintenue|Le candidat ptrace event-only étudié est insuffisant ; fermeture supplémentaire non attribuée et coûts UNKNOWN, donc STOP|
|Canon ET entrées transitives strictement inchangés|Pas de raccord FD livré contrôlant ces racines ; HOLD sans inventer une observation|

Un éventuel contrat devra séparer faits constructifs, entrée confirmée,
générations observées et couverture inconnue ; conserver pertes et première
cause, source/catalogue/argv/phase, RAW et vrai wait. La copie/ACK CP ne devient
jamais le producteur d'un fait physique. SDK reste indépendant au PID réel.

Si un développement est ensuite autorisé, les témoins devront être prépubliés
au raccord réel : substitution chemin/contenu, erreur exec et mort avant handoff,
A→B, exec nonleader/enfant bref/CLONE_UNTRACED, fermeture avant le corps,
RAW ambigus, perte du producteur, END pendant hash et cleanup UNKNOWN. Ceci
n'est ni une recette exécutable, ni une demande de CPU, ni un budget réservé.

## 10. Résultat et ticket ouvert

La piste constructive minimale possède un raccord source concret à examiner ;
sa stabilité d'objet, son succès de handoff et sa compatibilité ne sont PAS
livrés ou qualifiés. Elle offre une portée plus faible que le contrat20 actuel.
Le collecteur persistant étudié ne démontre pas la fermeture exhaustive.
Aucun coût chiffré ni impossibilité universelle ne sont déduits.

Les retours source CP/TTY/M sur la première analyse sont conservés. TTY a relu
la variante entrée-seule : avis documentaire conditionnel favorable à cette
distinction, avec handoff réussi et PID runner/enfant séparés ; contrat961 reste
plus fort, aucun abandon implicite. La variante est aussi soumise à CP/M, sans
présumer leur accord.
Le ticket commun et le parent TTY restent ouverts. Arbitrage C7 sur garantie et
surface avant code/prototype/test ; aucun ancien GO, Q229b ou résultat CPU n'est
converti en admission physique. Aucun chemin runtime n'est réservé. Les sources
Linux publiques citées ne sont pas une qualification de l'hôte.
