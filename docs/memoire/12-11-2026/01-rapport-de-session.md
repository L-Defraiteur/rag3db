# Mémoire — reprise du 12 novembre 2026

## Ticket et revue reçue

Le [ticket M8](../tickets/2026-11-07-une-memoire-ne-sait-pas-que-ses-gabarits-sont-en-retard.md) reste ouvert et porté par mémoire. Les sept parties de la revue ED sont intégrées dans [la note canonique](01-relecture-m8-adoption-fusion-et-reprise.md), commit `aa741a4c3bbd48a3194a802cfab1c8612cdd8a57` ; la conception du 9 novembre y est reliée et corrigée. Ne pas créer une seconde synthèse concurrente. Le « précontrôle 1b » cité correspond au commit `85ef4921128b445e74766c1e628809b206957485`.

Lectures indépendantes à `da69b842c3` : livre et détection des copies, fusion et écritures de refresh, ouverture/fermeture de slot, lecteur de recouvrement, chargement et runner de migrations. Elles confirment les limites principales de la revue : la fusion actuelle ne prouve pas les conflits, close n’est pas l’exclusion de maintenance et le runner ne garantit pas une bascule multi-ressources. Ces lectures ne constituent pas des tests.

## Complément de preuve et action restante

La lecture de `backend.rs::PreparedBackend::load_avec` et de `builtin_templates::{resolve,root}` montre un effet à cadrer pour le futur plan à blanc : un manifeste avec `builtin:` peut provoquer l’extraction des gabarits dans le cache si celui-ci est absent. Ce chemin a aussi été relu à `7004350fb6b0117a9b675d65da2121709fd7a146`. Aucun chargement de backend n’a été exécuté pour le vérifier.

Le ticket M8 et la note canonique portent ce complément. Avant toute promesse de plan purement sans écriture, définir le périmètre des effets et le contrat de résolution, et couvrir un cache builtin vide par un témoin. Un cache déjà chaud ne doit pas dissimuler l’effet. Aucun choix d’implémentation n’est déclaré acquis.

## Où reprendre

Reprendre depuis le ticket et la note canonique : coexistence vivant/épinglé, inventaire et bases récupérables, fusion pure, validation avec frontière d’effets explicite. La première tranche proposée reste un plan à blanc sur bases fournies. L’adoption persistée puis l’application attendent les contrats communs de maintenance, instantané et récupération, avec preuve sur copie privée. Les appelants des outils restent des observations partielles, pas un inventaire exhaustif.

Aucun code, build, test, modèle, service ou base réelle n’a été lancé ni modifié par cette reprise documentaire. Les suites [archivage sur copie](../tickets/2026-11-11-terminer-l-archivage-a-blanc-des-documents-sur-copie.md), [M9](../tickets/2026-11-11-raccorder-l-archiviste-automatique-au-chat.md) et [classeur local](../tickets/2026-11-11-trancher-et-verifier-le-classeur-local-du-rappel.md) conservent leurs tickets et leurs conditions propres.

## Cadrage documentaire de la première tranche

La note canonique sépare désormais le cœur de plan sur contenus fournis/embarqués et l’éventuel adaptateur CLI de préparation des sources. Le cœur proposé n’a aucun effet de lecture implicite, d’extraction ou d’écriture ; une source requise manque, il refuse. Les sorties sont un candidat et des différences/refus, sans droit d’application. Le cas de cache vide et l’absence de repli sur le disque font partie des preuves à construire.

Le contrat est proposé, non implémenté. Les choix vivant/épinglé et les garanties de maintenance/récupération restent à traiter dans le même ticket M8. Aucun code, build, test ou base réelle n’a été engagé pour ce cadrage.

## Lot de code : situation après globale

La première tranche M8 a ensuite été autorisée et implémentée : API Rust de plan fermé sur contenus, fusion3voies, lecteur fermé raccordé au validateur commun ; aucun CLI/apply/persistant. Branche publiée cx-memoire-m8-plan-3, worktree portable /home/lucied/.cache/rag3weaver-build/cx-wt-memoire-m8-plan ; worktree luciepc /home/lucied/git_workspaces/cx-memoire-m8-plan, source globale97f8f7ad9, branche locale cx-memoire-m8-plan-globale-3. Targetofficiel unique luciepc cx-target-memoire-m8-plan.

[API, rouges/verts et preuves de globale](02-plan-m8-api-et-preuves.md) : ciblés36/0/0, globale features5/5 +minimalchecklib +lib2192/0/64 +nav39/0/0 +8e2e100/0/0. Driver brutROUGE de nettoyageTMPglobal, résultats testsVERT distincts ;43résidus possédés inventoriés puis archivés,4ELF/220assets attestés, processus quiescents et7879 libéré. Diagnostic soumis à orchestration, aucun tourmaster à ce point. TicketM8 reste ouvert et source97 reste la seule source de globale ; docs suivantes ne changent pas le code.

Le petit lot cfg80 est déjà intégré avec Arbre sur master3b2eab7ec. Worktrees cfg portable cx-wt-memoire-cfg-temoin-natif et luciepc cx-memoire-cfg-temoin-natif restent à nettoyer sous garde exacte65aa avec archives administratives externes. Targetcfg secondaire1,4Go créé par erreur, inactif/inventorié ; aucun Cargo nouveau dedans et aucun effacement aveugle. Le ticket ttyglobal reste ouvert pour ses2helpers minimaux préexistants.

Reprise : faire relire le diagnostic de nettoyage et les attestations ; avant master fetch/rebase sous branche neuve, examiner le Rust reçu et rejouer court hors verrou si nécessaire. Les worktrees/targets inachevés ne se retirent qu’après preuves gardées, quiescence et garde commune VERT ; aucune force. Aucune activation M9, aucun lancement des50documents, aucune montée de la mémoire réelle autorisés.
