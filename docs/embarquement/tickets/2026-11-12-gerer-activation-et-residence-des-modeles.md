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
