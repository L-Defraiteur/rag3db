# Gérer l'activation et la résidence des modèles selon les usages

- **État** : ouvert ; première tranche locale/dense qualifiée, combinaison avec master en cours sur cx-activation-modeles-3 ; suite distante/éviction ouverte.
- **Famille** : produit
- **Gravité** : coût et disponibilité ; éviter l'accumulation de modèles résidents et permettre leur reprise à la demande.
- **Atteignable en service** : non pour cette gestion automatique ; lancement sondé et chauffe initiale existent séparément.
- **Touche rag3weaver** : oui, ModelSource, Serveur, fournisseurs de modèles et déclarations de ressources.
- **Pour** : embarquement et ED pour les contrats communs ; optimiseur pour l'activation distante et les unités.
- **Signalé par** : Lucie, demande de résidence pertinente et de warming ; audit des services résidents.
- **Ouvert le** : 12 novembre 2026

## Conception

[Vision](../../orchestration/visions/2026-11-12-residence-des-modeles-et-prechauffage.md)
et [audit](../../tickets/closed/2026-11-12-revoir-les-services-d-embarquement-residents.md).
Composer ModelSource, Serveur/Attache et les unités existantes. Qualifier
résidence, activation partagée par modèle/profil/périphérique, baux d'usage,
éviction sans requête en cours, préchauffage anticipé et états observables.
La forme service actuelle joint un fournisseur ; elle ne l'active pas à distance.
Le groupe BGE/relecteur/OCR reste couplé au processus tant qu'une granularité
plus fine n'est pas implémentée et prouvée.

## Fin attendue

Première tranche revue puis réellement accessible aux appelants du modèle,
avec identités et espaces préservés, activation unique sous concurrence,
annulation isolée, budget et reprise à froid qualifiés aux deux postes.
Décrire chauffe partielle et erreurs par capacité sans les transformer en
réussite vide. Aucune nouvelle boucle d'agent nécessaire pour ce contrat de
ressources. La note seule et les arrêts manuels de l'audit ne ferment pas ce ticket.

## Première tranche nocturne autorisée — 12 novembre

Embarquement prend la première capacité commune réellement appelée : identité
modèle/profil, acquisition unique sous concurrence, état de chauffe/erreur et
annulation isolée. Composer ModelSource/Serveur/Attache avant un autre mécanisme ;
les politiques TTL/maintien/éviction sont des entrées déclarées, pas de nouveaux
défauts imposés aux backends. Aucun changement des services ou paramètres
d'embedding existants dans cette tranche.

Banc prépare indépendamment la recette puis les mesures de chauffe/reprise,
concurrence et annulation pertinentes sur modèle privé déjà disponible ; il
n'édite pas le runtime d'Embarquement. Fixtures CPU d'abord, source/recette et
placement avant toute mesure réelle. Card2 est autorisée normalement ; card0
seulement dans la fenêtre explicitée dans
[la répartition](../../orchestration/12-11-2026/02-lots-independants-pour-la-nuit.md)
et après entrée vérifiée d'Optimiseur. Ne pas répéter le banc Qwen clos ni
charger un modèle sans question de qualification. Le ticket reste ouvert si
seul un prototype ou une chauffe partielle est prouvé.

## Tranche de nuit : recette et limites

Worktree `cx-activation-modeles`, porteur embarquement. ED conserve hôte/agent,
banc prépare la mesure et mémoire relit le contrat. Aucun service installé
n'est modifié. Premier témoin CPU publié dans
`extension/rag3weaver/tests/activation/serveur.rs` : deux demandeurs concurrents
partent du même fournisseur froid ; une seule création de processus est attendue.
Le processus Python est factice, borné par le lanceur et arrêté par son PID propre.

Recette légère avant lourd : `rustc --edition=2021 --test
extension/rag3weaver/tests/activation/serveur.rs -o /tmp/cx-activation-serveur`,
puis `timeout 20s /tmp/cx-activation-serveur
--exact deux_demandeurs_ne_lancent_qu_un_processus --nocapture`.
Le premier rouge est attendu sur le nombre de lancements, pas un défaut GPU.

Contrat prévu : acquisition locale partagée au-dessus de Serveur/Attache,
verrou interprocessus sur l'adresse, budget de chargement distinct du délai
d'attente d'un demandeur. Annuler une attente ne tue pas une activation
partagée. ModelSource déclare explicitement l'activation, sans changer les
fournisseurs existants. Première capacité dense accessible par connect_embedder.
Identité modèle/dimension vérifiée avant remise du client ; chauffe fournisseur
observée, inconnue si non publiée. La politique initiale conserve le service,
sans TTL, éviction ou activation distante implicites. Les manques restent ouverts.

### Rouge réellement observé

Sur 5754a75ef, le 12 novembre : compilation rustc du témoin puis un test
joué, zéro ignoré. Deux créations au lieu d'une ; un demandeur reçoit
`Mort(code 1)` et l'autre réussit. La fixture a été arrêtée et nettoyée.

### Première source à relire

`serveur/activation.rs` rassemble les demandes du processus et sérialise
entre processus via un verrou fichier système sur l'adresse locale fixe.
Les participants doivent partager le répertoire de coordination. Le worker
possède l'activation bornée, pas un demandeur. Phase disponible ne prétend pas
que les capacités sont chauffées : la santé porte séparément le bilan de chauffe.

`ModelSource.activation` déclare programme, arguments, environnement, profil,
périphérique, répertoires coordination/journal, budgets activation/attente et
résidence `maintenir`. Profil/périphérique déclarés ne constituent pas une preuve
de placement physique. Le connecteur dense existant passe par cette acquisition ;
sa variante annulable isole l'attente et son client conserve un bail. Les autres
capacités refusent explicitement une activation non encore supportée. Activation
distante, éviction et comptage global des usages restent hors de cette tranche.

Recette suivante CPU : même compilation rustc puis filtre `demandeur` (deux
témoins : partage et annulation). Ensuite vérification crate avec feature daemon
et test du vrai connecteur via fournisseur factice, sous poste lourd à luciepc
pour toute compilation Cargo. Aucun service installé ni modèle GPU modifié.

### Revue et corrections avant qualification

Revue mémoire sur 67f3a3db0 : le chemin géré omettait `timeout_secs` du
client dense ; corrigé avec témoin `client.timeout() == 1s`. Les sondes
héritées réinitialisaient leur timeout à chaque lecture : une réponse lente
pouvait dépasser le budget du worker. Échéance absolue propagée aux sondes
connexion/écriture/lecture et au premier lancement ; témoin TCP CPU distillant
des octets, budget 150 ms et résultat attendu sous une seconde.

Fixture bancaire CPU intégrée depuis 3963f6829 (dffb2724b). Témoins du
connecteur réel dans `src/model_source/activation.rs` : déclaration serde,
activation dense, identité/dimension, appel /embed, délai préservé, santé legacy
chauffe inconnue, annulation avant départ et dimension étrangère refusée.
`daemon::embeddings::tests::chauffe_partielle_garde_erreurs_et_formes` exerce
le vrai préchauffage avec une forme refusée, sans modèle GPU.

Qualification suivante : rustc standalone (partage, annulation, sonde lente),
puis à luciepc sous poste lourd `cargo test --lib --features daemon
model_source::activation::tests` et filtre `chauffe_partielle`. Budget annoncé
20 minutes après acquisition pour cette première compilation ciblée, arrêt au
premier rouge inattendu. Source et recette sont poussées avant lancement.

Sur 235cde8ec : 15 tests standalone réellement joués, 14 passent, un rouge
sur le diagnostic de sonde lente. La durée est bornée mais la réponse partielle
à l'échéance est classée `Occupe` au lieu d'expiration du budget. Aucun Cargo
lancé après ce rouge. Correction : vérifier l'échéance avant d'interpréter la
réponse initiale et avant de rendre une attache après démarrage.

Sur ef11f0e16 : 15/15 tests standalone verts, zéro ignoré (0,40 s).
Premier Cargo à luciepc arrêté avant compilation : dépendance chemin codeparsers
sans Cargo.toml, sous-module du worktree neuf pas encore initialisé. Aucun
rouge runtime caché. Prochain lancement après `git submodule update --init
extension/rag3weaver/codeparsers` ; même budget privé, pas de rejeu inchangé.
Témoin autonome ajouté pour deux processus séparés synchronisés par une gate,
chacun appelle réellement l'acquisition commune ; un seul départ attendu.

Cargo après initialisation codeparsers : arrêté sur E0433 mail.rs:120, accès
à daemon_lifecycle exclu sans rag3db-native. Aucun témoin joué. Défaut séparé
consigné chez ED : `2026-11-12-la-boite-ne-compile-pas-sous-daemon-seul.md`.
Prochaine recette : features `rag3db-native,daemon`, export de la lib épinglée
par `scripts/lib_du_magasin.sh` puis mêmes filtres CPU, sans accès GPU.

Durcissement supplémentaire : santé du fournisseur courant atteste la recette
(profil, périphérique déclaré, empreinte). Un fichier administratif persistant
ne certifierait pas l'instance après reprise du port par autrui. Refuser un
fournisseur non attesté/différent, même si modèle et dimension sont identiques.
La chauffe absente reste inconnue ; ceci ne prouve toujours pas le placement GPU.

Troisième tentative Cargo arrêtée avant tests : guards des modules chat et
livraison exigent code en plus de natif/daemon. Même ticket de features ED
complété, sans retoucher ses fichiers. Jeu suivant : `rag3db-native,code,daemon`.

La sonde finale d'identité dense reçoit le restant du budget du demandeur ;
le délai des appels suivants est restauré depuis timeout_secs ou son défaut,
pas hérité de ce restant. Tests ajoutés sur délai par défaut et adoption d'un
profil étranger depuis un processus distinct (modèle/dimension identiques,
refus attendu sans tuer le fournisseur initial ni relancer un constructeur).

Sur 8a0b29180, Cargo compile le jeu natif/code/daemon. Cinq tests du
connecteur réellement joués : un vert (annulation initiale), quatre rouges
(timeout 5 s). Cause isolée : fournisseur factice sérialise le JSON avec
espaces (`service: ...`) ; sonde existante `daemon::sonde` cherche le marqueur
compact `"service":"rag3weaver-embeddings"`, émis par le vrai fournisseur.
Correction de la fixture : JSON compact via separators, pas d'augmentation
de budget ni de relâchement de sonde. Processus factices arrêtés après erreurs ;
log conservé à luciepc `~/.cache/embarquement-notes/activation/cpu-d.log`.
Banc informé avant modification du bloc partagé. Rejouer après ce correctif.

Sur d5133197e : cinq tests du vrai connecteur verts, zéro ignoré ; un témoin
de chauffe partielle vert (12/16 formes réussies, quatre erreurs conservées).
Log `cpu-e.log`, unité finie inactive/Result=success. Ces preuves sont un
précontrôle : lancement par ancien poste lourd direct et target-spans encore
marqué de son ancien worktree. Elles ne constituent pas la passe conforme.

Correction opérationnelle avant qualification : recette suivie
`scripts/qualifier_activation_modeles.sh` (modes cpu/lib), entrée commune
confiner_validation, plafond effectif <=32 Gio/bus privé/arrêt des descendants.
Reprise explicite du target de session via garde_target avec
RAG3WEAVER_TARGET_REPRENDRE=1 : invalidation des empreintes locales de l'ancien
arbre, dépendances conservées et nouveau propriétaire vérifié. Rejouer CPU
depuis cette portée, puis passe large/check_features/lib/binaire/navigateur.
Budgets : CPU 1200 s, lib 7200 s après acquisition ; arrêt au premier rouge.
Aucun service existant modifié. La mesure privée du banc attend ce socle.

### CPU qualifié dans la portée commune

Sur 6ddf622ca, luciepc : validation-rw5c889b VERT, quiescence contrôlée.
16/16 tests standalone, 5/5 connecteur dense, 1/1 vraie chauffe CPU (formes
refusées et faux succès vide conservés en erreurs), zéro ignoré pour ces groupes.
La garde a réellement invalidé les empreintes locales de l'ancien arbre et
marque maintenant le worktree cx-activation-modeles. Log `cpu-f.log`, preuves
`~/.cache/rag3weaver-build/validations/validation-rw5c889b`.
Huit filets Python de la fixture verts sur le portable après intégration Banc.

Recette large publiée mode lib : cinq jeux check_features, lib entière sous
rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm ;
binaire embeddings neuf pour la mesure privée, puis témoins navigateur avec
paquets React du magasin. Budget global 7200 s après acquisition, Max32G,
aucun accès au bus personnel, premier rouge arrêté. Plafond target vérifié
entre les étapes, aucune suppression automatique pour faire passer une passe.
Les e2e du changement suivent séparément cette passe (protocole dense/spans).

### Propriété du cache et borne de la passe large

Le rapport de cette session du 11 novembre identifie target-spans (13 Go) et
rag3db-spans comme ses biens, tous lots atterris et aucun travail inachevé.
Avant invalidation, .arbre-du-target a été lu : rag3db-spans/extension/rag3weaver.
L'unité privée cpu-e était inactive/Result=success, relevé conservé dans le
compte rendu d'outillage. La garde a effacé seulement les empreintes locales
(codeparsers, rag3db, rag3weaver, rag3weaver-ir, rag3weaver-tty, tree-sitter-scss),
pas les dépendances ni les fichiers d'un autre lot ; cpu-f.log garde ce relevé.
La validation commune rw5c889b atteste la quiescence finale du nouveau lot.
Aucun inventaire exhaustif historique des PID n'a été conservé avant invalidation :
ne pas transformer cette propriété documentée du cache en preuve plus large.

Plafonds de phases après acquisition du poste : check_features 2400 s, lib
1800 s, fournisseur 1200 s, navigateur 900 s ; somme 6300 s, global 7200 s
avec 900 s réservées aux contrôles et à l'arrêt. Un seul superviseur et aucun
reset de budget. Les plafonds internes provoquent l'arrêt du script au premier
rouge ; le superviseur conserve le budget global et contrôle les descendants.

Justification de la borne, pas une mesure de cette passe : les logs historiques
de la session spans montrent des libs entières à 30,55/39,64/56,64 s et un groupe
navigateur à 129,19 s ; compilation ciblée historique 35-53 s sur cache chaud.
Le précontrôle actuel a reconstruit le graphe CPU/code sur plusieurs minutes ;
le jeu large Burn/OCR et les cinq configurations doivent encore recompiler
des dépendances et relier de gros binaires. Les plafonds sont des maxima
conservateurs, jamais une autorisation de relancer après expiration.

Pas de nouveau service ou modèle pendant cette passe lib. Variables d'adresses
d'embarquement retirées de son environnement pour conserver les unitaires
isolés ; paquets React explicitement épinglés au dossier de la passe. Les e2e
par services (7878/7879) et le privé 107m du banc seront coordonnés séparément,
avec leurs recettes et leur propre borne annoncées avant acquisition.

Consommateur de qualification Banc intégré depuis 678c5e678 :
`examples/banc_residence.rs`, sans modification de Cargo.toml. Même phase de
bâti 1200 s que le fournisseur, mêmes features/target, aucune exécution GPU
automatique. Il observe d'abord demander/attendre puis raccorde le connecteur
public à chaud : distinguer ce coût de raccordement du coût de reprise à froid.
Le banc garde ses contrôleurs/recettes/placements et arrêts de groupes privés.

### Passe large sur 356d7d927 et artefacts privés

Validation-sal6yn1v VERT, unité inactive/Result=success, quiescence finale.
CHECK_FEATURES 5/5, lib entière 2249 passés/0 échec/65 ignorés structurels
(non qualifiés par cette exécution), 40/40 navigateur sans ignoré. Target 28 Gio.
Fournisseur et consommateur copiés en lecture seule hors target dans
`~/.cache/embarquement-notes/activation/bin-356d7d927/` (SOURCE, SHA256SUMS,
dependances-fournisseur.txt, dependances-consommateur.txt). Aucun not found
dans ldd ; pas de dépendance dynamique librag3db observée pour ces exécutables.
Fournisseur SHA256 1c41f167fc016ff4ddc33b667f3045388003969510f836512749c10e87a217f5 ;
consommateur c0ae3020530dfd717579fb15f01f302bacaf08da3202299cd58eabf672d1b2c9.
Banc prépare sa recette privée107m/card2, sans service installé modifié.

### Cas limite de durée avant intégration

La déclaration peut contenir un budget supérieur à la représentation Instant.
Le worker additionne cette durée sans checked_add : risque de panique du fil
au lieu d'une erreur de déclaration. Témoin CPU autonome ajouté, attendu rouge
sur la source actuelle : `budget_non_representable_est_refuse_avant_le_worker`.
Recette : rustc standalone puis ce filtre exact dans une portée commune 30 s,
aucun programme réel/GPU. Après rouge, refuser ce budget avant démarrage du fil.
La preuve normale de la passe précédente n'est pas étendue à ce cas limite.

Sur 2721123c5, témoin de budget non représentable : un test réellement
joué, zéro ignoré, ROUGE (l'API accepte la déclaration au lieu de la refuser).
Validation-w1ppfj1m et budget-rouge.log conservés, aucun fournisseur lancé.
Correctif : checked_add avant création du worker et échéance vérifiée avant
assurer ; même refus explicite pour la primitive Serveur brute. Rejouer la
recette CPU complète sous portée commune, sans augmenter son budget 1200 s.
Les artefacts Banc source356d restent figés ; ce delta ne change que le refus
des budgets invalides, pas les recettes valides de mesure.

### Reprise sur master et qualification finale

Rebase sur 9915fe6f4, huit commits amont : docs et fixtures sous docs seulement,
aucun Rust ajouté sous extension/rag3weaver. Diff après rebase relu ; nouvelle
branche cx-activation-modeles-2, worktree inchangé. Aucune branche distante
antérieure supprimée ou réécrite. Runtime 799c6b8b2 : validation-jqwupheq VERT,
17/17 standalone (budget non représentable maintenant refusé), 5/5 connecteur,
1/1 chauffe, zéro ignoré, quiescence contrôlée. Revue mémoire ciblée favorable
sur checked_add et absence de régression de la deadline.

Consumer Banc enrichi depuis 26522eb8c (77ca19867) : gate_usage facultative
pour attendre la preuve de placement avant le vrai appel embed ; aucun runtime
changé par cette addition. Reconstruction dans la même phase 1200 s, copies
neuves hors du répertoire immutable source356d.

Passe large finale rejouée avec les mêmes plafonds 2400/1800/1200/900 et
global7200, après la correction propre du budget et du consumer. La rebase docs
ne l'exigeait pas à elle seule. Aucune reprise d'une tentative expirée, et
aucune activation de service installé ni charge privée dans ce lancement.

### Plafond du target atteint sur la passe finale

Sur 345a402d0 : F5/5 et lib2249 passés/65 ignorés, fournisseur/consumer
construits, puis arrêt avant navigateur : 35163144192 octets >32 Gio.
Validation-4qfyognx ROUGE code1, restes=[], nettoye=true ; unité inactive.
Aucune suppression automatique ni augmentation du plafond.

Copies immutables conservées dans bin-345a402d0 hors target (même parent de
preuves que bin-356d7d927, qui reste intact) : fournisseur SHA256
10f44d35479f2a6462a6db5f2dfbad2714f524eff66a27589940e4e5ad44345a ;
consumer avec gate_usage c171231c0d158fef5bd06755ca7ee22c1cb00a386d1266461ce7883ef2a6fb73.
SOURCE/ldd/SHA256SUMS conservés ; aucune dépendance not found.

Remède borné au cache possédé : garde_target confirme l'arbre actuel, validation
atteste l'arrêt de ses descendants ; `cargo clean -p rag3weaver --target-dir
/home/lucied/.cache/rag3weaver-build/target-spans --dry-run --verbose` inventorie
9176 artefacts/28,0 Gio apparents, tous sous ce target. Conserver cette prévision
dans nettoyage-prevision.log, puis nettoyer ce seul paquet et mesurer le target.
Pas de suppression de modèle, de données, de worktree, de lib du magasin ni
d'artefact immutable du banc. Requalification seulement après correction de
cette cause, sans remise à zéro de la tentative précédente.

Nettoyage réalisé du seul paquet : 9162 artefacts/26,6 Gio apparents retirés ;
target mesuré 35163144192 → 10602258432 octets. Contrôle postérieur ciblé
exe/cwd/FD/cgroups : zéro référence au target, target-apres-nettoyage.json.
Copies provider/consumer345 rehashées conformes après action.

Limite de preuve conservée : instruction détaillée d'orchestration reçue après
le début de l'action. Avant mutation : propriété par garde, unité inactive et
validation avec restes=[] ; pas de scan FD exhaustif. La commande exécutée
avait le --target-dir absolu et le cwd du manifeste possédé ; --manifest-path
n'était pas explicite. Ne pas prétendre avoir effectué ces deux contrôles
supplémentaires avant l'action. Les prochains nettoyages les intégreront.

Requalification après correction de capacité, source runtime345 inchangée :
driver versionné scripts/requalifier_activation_apres_cache.sh, lancé sous
POSTE_MEM_MAX=32G et le vrai poste lourd. Il calcule le temps restant depuis
l'échéance monotone de validation-4qfyognx APRÈS acquisition ; aucune nouvelle
borne de7200. Refus si échéance expirée. Puis même recette phases
2400/1800/1200/900, chacune limitée aussi par le restant global, Max32G/j8 et
plafond target32. Pas de recyclage du navigateur non joué. Les modèles/services
restent inchangés et la recette ne lance aucun modèle privé de mesure.

### Requalification après cache et recette e2e

Validation-e21lc7w8 VERT : F5/5, lib2249 réussis/65 ignorés structurels,
40/40 navigateur sans ignoré, restes=[], unité inactive/Result=success.
6649 s accordées après acquisition ; échéance875850,661490103 inférieure à
l'originale875850,721087351. Target20941500416 octets, donc sous32 Gio.

E2E après la fenêtre privée du banc, pas de réservation GPU pendant compilation.
Recette versionnée `scripts/qualifier_activation_e2e.sh demon` : portée commune
mode mesure1800 s (construction du binaire e2e comprise, graphe large déjà
compilé), deux témoins fournisseur privé BGE1024 puis granite278/768 exécutés
SÉRIELLEMENT via RUST_TEST_THREADS=1. Les services 7878/7879 restent installés
et inchangés ; aucune instance107/7880 ni7882 n'est réveillée. GPU embedder
explicitement gpu:0/card2, chauffe activée, relecteur/OCR privés désactivés,
budget caractères4096/duty70, filtre ROCm éventuel limité aux privés.
Lire la capacité/VRAM fraîche avant entrée, attribuer les PID privés par la
portée et observer fdinfo PCI/mémoire/moteurs ; arrêter seulement ses propres
processus si le placement ne respecte pas le profil.

Puis `... spans` : portée commune lourd600 s, un témoin de protocole par le
BGE7879 existant (deux textes, deux appels), santé avant/après et coordination
avec les consommateurs réels. Aucun nouveau modèle dans cette seconde suite.
Lib épinglée29a, j8, target possédé/max32 ; arrêt au premier rouge, aucun
réessai inchangé ni remise à zéro de budget. Ces nouvelles phases n'étaient
pas jouées dans la passe lib et ne prolongent pas son échéance.

Préparation fixture du témoin fournisseur : source ancienne conservait deux
journaux sous des noms fixes dans TMPDIR. Journaux désormais uniques et
nettoyés seulement après l'arrêt explicite des enfants possédés et succès ;
sur panique ils restent pour l'archive rouge du lanceur. Aucun seuil/contrôle
fonctionnel n'est abaissé. La santé du nouveau BGE privé doit rapporter une
chauffe complète non vide et sans erreur, en plus des vecteurs réellement
calculés déjà contrôlés. Pas de témoin supplémentaire artificiel pour ce
nettoyage de fixture ; garde des résidus et e2e réel jugeront son résultat.

Banc signale sa mesure réelle verte (analyse à consolider par son porteur) :
validation-bvdguacm VERT/restes=[], 23:54:31→23:54:37, durée5,866447 s.
A annulé, B/C même PID1097632, reprise D PID1098406 ; chauffe complète16/16
(1226/1170 ms), vrais vecteurs384 non factices, PCI04 et activité gfx. Deux
privés arrêtés, port48329 absent et card2 revenue exactement à17212403712 octets.
La fenêtre est rendue ; aucun nouveau modèle partagé à arrêter. Les mesures
du banc viennent d'une vraie acquisition avant raccordement à chaud, pas d'un
chronométrage froid direct de connect_embedder.

Recette e2e complétée avant entrée : check_features des fixtures modifiées
plafonné300 s dans les1800 s existants du mode demon, pas de budget ajouté ;
santé7879 avant/après incluse au mode spans. Observation GPU des seuls privés
dans la portée, données manquantes inconnues ; réutiliser capture du banc au
lieu d'inventer une logique de placement dans le produit. MainPID des services
peut être relu hors portée privée, sans ouvrir le bus utilisateur dans la suite.

Avant e2e, capacité fraîche : card2 utilisé17212403712/34208743424 octets,
soit16996339712 octets libres. Un seul privé BGE ou278 à la fois, profil
Burn/WGPU gpu:0/Flex32, ROCR0, HIP/CUDA non posés. Les poids/tokeniseurs
BGE2,2Go/17Mo et2781,1Go/8,7Mo sont présents aux chemins de cache explicites ;
leurs empreintes seront conservées avant charge (cache de fichiers ainsi
réchauffé, pas une prétention de froid disque). Pas de téléchargement.

Driver d'échéance réutilisé aussi pour demon/spans : exige le mode réel du
poste (mesure/lourd respectivement), calcule après acquisition le minimum
1800/600 s et restant de l'échéance originale4qfyognx. Refus si expirée, pas
de nouveau7200. Même source e2e publiée avant entrée ; observation de placement
par capture du banc et données propres des PID, pas une variable GPU seule.
La demande d'usage bref7879 est coordonnée directement avec Archi avant spans.

### E2E fonctionnel et correction de capture

Sur fb8fc788a : validation-s69dn7tx VERT, check_features5/5 puis deux tests
fournisseur réel réussis/0 ignoré, une suite jouée sur une, TMPDIR propre ;
13,99 s pour les deux modèles privés sériels.
Capture opérateur incomplète : un premier observateur ad hoc a échoué sur
PermissionError `/proc/1247/exe` (processus étranger), puis l'observation
corrigée tardive n'a vu aucun PID privé. Les chiffres card0 globaux du journal
de charge ne sont PAS attribués à ces modèles ; aucun placement par PID
n'est inventé pour cette première exécution. Le rouge d'observabilité reste
conservé avec le vert fonctionnel.

Cause corrigée dans le driver avant nouveau test : observer_e2e_gpu.py est
préattaché et sa readiness attendue avant lancement, filtre le cgroup vérifié
AVANT lire exe/FD, traite OSError comme inconnu. Il réutilise capture du banc
SHA85a4d58312cd6629bea9725159208b3496c5ea096635202dc7b37675b8ff590f
et sa garde mémoire SHAa29173c8ad1696e1e1c70460356c29049249ee9861ec7a688f296769693125ef.
Seulement notre exécutable fournisseur dans notre portée, deux PID attendus ;
VRAM et gfx PCI04 doivent être observés, champs inconnus jamais publiés zéro,
activité PCI07 ou violation de garde arrête notre seul superviseur vérifié.
Les captures sont hors target dans un dossier neuf ; leur lecture/hash peut
perturber les temps, aucun chiffre de performance absolue n'est revendiqué.

Rejeu INSTRUMENTÉ après cette correction, pas une répétition inchangée :
conserver l'échéance de s69dn7tx (budget initial1800), pas nouvelle1800 ni7200.
Le driver calcule le restant après la vraie acquisition mesure ; précontrôle
à1108 s restantes, refus si expiré. Source publiée/syntaxe Python et shell
vérifiées avant entrée. Artefacts du banc345 et anciens journaux restent intacts.

Revue mémoire20a8 : la première version d'observateur cessait de lire un PID
après sa première capture qualifiée ; une activité ultérieure aurait été
manquée. Elle vérifiait aussi le starttime avant pidfd_open au lieu d'après.
Le job20a attendait encore le poste, aucun superviseur commun actif ni modèle
chargé : arrêt des trois seuls PID de l'unité possédée1263275/1263277/1263295
par pidfd avec identité vérifiée, unité ensuite inactive/MainPID0.

Correction avant nouvelle charge : capture/garde continuent jusqu'au signal
de fin, preuve initiale conservée, clé PID+starttime et dernière preuve mises
à jour ; identité du superviseur recontrôlée APRÈS ouverture du pidfd.
Aucun Rust/Cargo changé depuis fb8fc788a ; les F5 réellement verts de s69
restent valides, pas de nouveau rejeuF5 pour cette seule instrumentationPython.
Reste de la même échéance s69 calculé après acquisition, aucun renouvellement
silencieux si le poste tarde ou si l'échéance est expirée.

### Derniers résultats puis actualisation master

E2E instrumenté9c8173275 : validation-tbwulyl4 VERT/code0/restes[]/nettoyé,
budget331 s restant de s69, échéance872389,768482234 conservée. Deux tests
fournisseur passés/0 ignoré ; 80 captures complètes, aucun client PCI07
observé. PID1490216/start87209641 et1491015/start87210172 : résidence et
gfx PCI04 positifs, premières ET dernières captures gardées. Une course
de lecture /proc/1489937/exe disparu est nommée inconnue, pas publiée zéro.

Spans : validation-igji2soq VERT/code0/restes[]/nettoyé, budget600 s, un test
joué/0 ignoré et une suite sur une. Santé7879 avant/après identique ; PID
partagés966256/3415586 inchangés et VRAM card2 revenue à17212403712 octets.
Pas de place/service conservé. Target25818341376 octets sous32 Gio.

Rebase sur8f604328a : format C++ pur et Rust de navigation arrivés entretemps.
Tous ajouts amont conservés, diff amont12fichiers rag3weaver relu ; nouvelle
branche cx-activation-modeles-3, source456797a07, anciennes remotes préservées.
Rejeu requis pour nouveau Rust : phases distinctes1200(lib complète/F5/bin/NAV),
900(provider instrumenté) et300(spans) après vraie acquisition, chacune
limitée AUSSI au restant de la première échéance4qfyognx (2464 s au relevé).
Aucun nouveau7200. ACTIVATION_PHASE_PLAFOND ne peut que réduire le plafond
de phase, jamais augmenter ni renouveler son échéance.

Avant nouvelle compilation, risque d'accumulation du target connu : figer et
rehacher provider/consumer actuels hors cible, scanner exe/cwd/FD/cgroups
AVANT mutation, confirmer garde propriétaire/quiescence, dry-run puis seul
cargo clean -p rag3weaver avec manifest et target ABSOLUS, mesurer et
conserver les preuves. Rien du magasin29, des modèles/services ou d'autrui
n'est visé. Les anciens rouges et binaires345 restent conservés.

### Combinaison navigation puis pile Pub → ED → Embarquement

Sur f01acd980 (master8f604 + lot), validation-qdm6acnb VERT : F5/5,
lib2258 passés/65 ignorés, NAV40/40 sans ignoré. Plafond1200 s et échéance
4q conservés. Rejeu fournisseur combiné validation-uu18g72q VERT : deux
tests/0 ignoré et capture continue de deux privés avec résidence/gfx PCI04.
Les trois courses de lecture de processus quittant la portée restent nommées
inconnues dans le résumé. Spans combiné en cours sous300 s/min ancien4q.

Consigne d'intégration reçue : Pub puis ED puis ce lot. Préparer notre union
sur candidats Pub/ED publiés figés, sans attendre leurs atterrissages. Demander
SHA/tree/propriétés des fichiers communs ; préserver nos seules lignes de
constructeur dans backend.rs et les BAC d'ED. Une vraie union Rust impose sa
qualification et les quatre e2e_tx de liaison catalogue à coordonner avec Arbre.
Pas de nouveau mécanisme ; pas de rejeu GPU107/BGE/278 pour rebase seule si
constructeurs, consommateur, fournisseur et observateur sont inchangés et
preuves qualifiées. Contrôler leurs diffs/SHA et conserver les résultats déjà
obtenus, sans les étendre à du code modifié. Parent ticket toujours ouvert pour
les limites distante/éviction/baux distribués/crash.

### Union figée Pub/ED du 13 novembre

Spans combiné terminé : validation-twt84wn7 VERT. Empilage sans conflit sur
ED c9e6b6e39/tree cddac94f03a4fb4e73f6870f5bbd3550c12ffdec, lui-même sur
Pub31db55abb et navigation8f604. Branche neuve cx-activation-modeles-4,
première tête dfa0f590a ; anciennes branches conservées. Diff vide des
model_source, activation/serveur, fournisseur, consommateur et observateur
par rapport à branche3 : preuves GPU précédentes conservées, aucun rejeu GPU
pour cette seule intégration. Les deux modifications backend.rs restent
séparées : constructeur ModelSource et montage BAC d'ED.

Qualification d'union CPU distincte, autorisée par l'orchestration : recette
`../13-11-2026/qualifier-union.py`, enveloppe commune globale2400 s calculée
après acquisition réelle lourd, j8/memory.max32G/high0/bus privé ; target
possédé target-spans (23187992576 octets avant). F5, lib entière, NAV puis
les quatre suites catalogue neuves exactes3/1/18/3 ; une base PG de fixture
créée exclusivement si absente, supprimée en finally. Premier rouge arrêté.
L'ancienne échéance4q demeure historique et n'est pas renouvelée. Aucun
changement de service ou de modèle ; porteur embarquement, fin attendue :
preuves d'union vertes et atterrissage dans l'ordre Pub → ED → Embar.


Précontrôle de lancement union : systemd refuse MemoryHigh=0 comme propriété
brute ; aucun travail démarré. Remplacé par infinity (le paramètre commun high0
signifie sans limite intermédiaire). Premier lancement ensuite refusé AVANT
recette : « aucun plafond effectif <= 32 Gio », car poste crée sa propre portée
au défaut40G, sans hériter de celle du service parent. Aucun test ni fixture
créé, aucun budget de travail consommé. Deuxième unité fixe explicitement
POSTE_MEM_MAX=32G avant poste, paramètre commun HIGH0 conservé ; entrée de
recette réussie, échéance2400 s créée après acquisition réelle. Logs union.log
et union-2.log conservés, pas de vert inventé pour ces précontrôles.

### Rouge d'union — carte show_progress absente

Source84259404f, validation-hex7oup7 ROUGE/code1/restes[]/bus nettoyé.
F5/5 verts puis lib2272 passés/1 échec/66 ignorés structurels en47,53 s.
Échec `daemon_host::worktrees::tests::a_progress_card_reaches_the_page_and_never_the_model`,
worktrees.rs1934 : `ui[0].kind` vaut null au lieu de index_progress sur le
second tour outil show_progress. Son contenu comporte une enveloppe delivery,
metadata vide, résultat plein texte prêt4/4 et vecteurs0/3. La première carte
attach_worktree ne rougit pas. Aucun NAV/tx joué après ce rouge. Log exact
hors target : activation/union-pub-ed/lib.log lignes2673–2698.

Nouvelle action d'intégration dans ce même ticket : porteur attendu ED pour
le parcours carte/progression, embarquement pour la requalification finale.
Diagnostiquer cette absence sans affaiblir l'assertion ni la remplacer par
un rejeu chanceux ; conserver le rouge et utiliser le restant de l'échéance
hex7oup7 pour une reprise éventuelle. Aucune nouvelle charge de modèle demandée.

Reprise préparée, pas encore exécutée : `13-11-2026/reprendre-union.sh` lit
l'échéance absolue877122,611725539 de hex7oup7 APRÈS acquisition lourd, refuse
l'expiration et limite la nouvelle portée au restant seulement. Le suffixe
explicite impose de nouveaux journaux/fixture PG sans écraser le rouge initial.
ED confirme prendre le diagnostic source, aucun rejeu Cargo chanceux ni édition
concurrente de worktrees.rs par cette session ; c9 demeure figé jusqu'au routage.

Correctif ED08c379b11 consommé par cherry-pick c44dfde1a, seul testworktrees.rs
+63lignes, production inchangée. Preuve ED : compilationF3/lib29 30,75 s,
exact1/0/0 en3,70 s, portée commune VERT/restes[]/nettoyétrue ; preuves
I-luciepc/parcours-agent-ide-20261112/fixture-carte-index. Barrière réellement
atteinte, assertions initiales2cartes/flux/confidentialité conservées, état
terminé testé sans UI. Pas de nouveau mécanisme ni seuil affaibli.

Reprise de l'union complète préparée sur cette source : F5/lib/NAV/4tx CPU,
script reprendre-union.sh, suffixe-r2, reste de hex calculé après acquisition
et aucun nouveau2400. ED conserve son candidat produit c9 ; ce correctif test
fait partie de notre lot d'intégration. Pas de rejeu GPU puisque ses composants
protégés n'ont pas changé. Les journaux initiaux rouges ne sont pas écrasés.


### Régression de fixture corrigée et union qualifiée

Correction08c379b11/c44dfde1a : la fixture tient un vrai index à sa barrière
CPU pendant les deux reçus de carte, puis prouve le cas terminé sans UI. Les
assertions d'origine sur cartes/flux/confidentialité restent intégrales.
Régression d'intégration résolue sur138268398, preuve validation-5wmntc4y :
F5/5, lib2273 passés/0 échec/66 ignorés structurels non qualifiés, NAV40/0/0,
transactions ligne-à-ligne3/0/0, merge-copy1/0/0, arrêt-paquet18/0/0,
rollback-vecteurs3/0/0 ; quatre suites sur quatre, aucune sautée. Portée VERT,
code0, restes[], nettoyage true ; fixture PG supprimée, unité inactive/code0.
Échéance877122,116265722, inférieure à hex877122,611725539 ; budget1275 s
restant réellement acquis, aucun renouvellement2400. Target24407724032 octets.
Le rouge hex et ses logs restent conservés hors target.

Pub livrébc1a81500 puis ED43e4f185b ; diff produit vide entre Pub31 etbc,
et entre EDc9 et43. Parent code déjà qualifié : pas de nouveau rejeu pour
leurs bilans documentaires. Composants activation/constructor/provider/consumer/
observer inchangés depuis les deux passages physiques qualifiés ; pas de
nouvelle chargeGPU pour cette intégration. Prochain pas : tour master explicite,
contrôles et retrait gardé avec archives. Le ticket global reste ouvert pour
les suites distante/éviction/baux distribués/crash ; seul ce défaut de fixture
est résolu, aucune prétention de terminer ces politiques.


### Livraison de la première tranche locale

Master23ceed87b80c58fb0070bf69cab8fa610d133ab3,
tree3d8f853fa172e47a1fd41e27a167d950bf9e1032. Tour explicite de
l'orchestration, revue finale Mémoire favorable, dernier fetch/poste push/
if VERT et trois contrôles lus propres. Rebase documentaire initial abandonné
sans push (anciens bilans ED), puis --onto master c9 hors verrou sur branche6,
diff produit vide et livraison sans nouveau rejeu. Les suites distante,
éviction, baux distribués et reprise crash maintiennent ce ticket ouvert.
Sources138 archivées hors cible et preuves qualifiées/rouges/remotes conservés.
Retrait des seuls W/target propres après quiescence, archives fidèles et garde
actuelle fraîche ; reçus externes dans activation/archives-fin-{portable,luciepc}.
