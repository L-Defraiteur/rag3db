# Gérer l'activation et la résidence des modèles selon les usages

- **État** : ouvert ; première tranche en cours sur cx-activation-modeles, base a783013f1.
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
