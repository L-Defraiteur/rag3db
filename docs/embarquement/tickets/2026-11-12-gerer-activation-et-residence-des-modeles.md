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
