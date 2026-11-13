# Passation d’orchestration — 13 novembre 2026

## Instruction de fin de session

Lucie demande de préserver l’usage restant : toutes les sessions passent leur
travail, poussent leur WIP possédé même inachevé et cessent de demander de nouveaux
créneaux de calcul. La reprise du week-end se fait depuis ces documents et les
branches poussées. Un ancien rappel périodique ne lève pas cette priorité.
**Aucun ancien GO, reçu ou délai, même techniquement encore futur au moment de
l’arrêt, ne se réactive.** Ne pas relancer de test rouge inchangé.

Le rôle de coordination peut être repris par la nouvelle session : il n’est
pas nécessaire de contacter une ancienne instance pour les actions déjà autorisées.
Les nouvelles limites de qualification se fixent sur l’état présent et une
recette relue ; ce n’est pas une nouvelle demande de permission à Lucie pour
chaque commande. Les décisions explicites de release restent distinctes.

Point d’entrée : [index des quatorze passations](00-lire-pour-reprendre.md).
Les treize rapports des autres sessions sont copiés depuis leurs commits exacts ;
les 13 têtes distantes ont été vérifiées. Les déclarations de WIP propre et poussé
sont celles de chaque propriétaire, avec chemins et limites dans son rapport.
Les gros caches/preuves/ELF sont conservés hors Git et ne sont pas des sources oubliées.

## Cap du projet et ce qui est effectivement livré

Atelier déclaratif pour explorer, comprendre et transformer code/documents/données.
Les entités, relations, traitements, outils et vues doivent composer les contrats
existants ; ne pas fabriquer un orchestrateur multiagents implicite. Le chat IDE
doit avoir un parcours de code réel et des outils accessibles ; présence d’une
fonction n’est pas preuve d’accessibilité. Mémoire automatique M9 conçue, pas à
annoncer livrée. Les décisions et connaissances gardent leur provenance.

Les vues visées : contenu H partagé sans confondre les occurrences, positions,
relations ou visibilité ; génération de publication P distincte d’identité de
contenu. Le courant du worktree sera automatique ; la provenance est par occurrence.
Ancien état/checkpoint explicite, matérialisation seulement si nécessaire ; ne
pas imposer l’ingestion de tout l’historique. Ce contrat n’est pas une fonctionnalité
déjà raccordée au stockage.

Livraisons importantes sur master rag3db :

- Raccord physique d’index explicitement neufs : `b0a5177c00bfaf3b37ae21ea0c630c515a19e224`,
  bilan/nettoyage `ba0317ed146e71f59bb8cbbf383822e100cb73e8`. Journal durable,
  publication/reprise, reconstruction lexicale et lecture snapshot qualifiés dans
  leur périmètre ; pas activation d’une base legacy ni qualification dense/sparse.
- Résolveur pur de couches 2a : `d17a230b14f60fbac59aa1e4ac10510ba5f3c7e2`.
  Parents P exacts, masques/overrides/provenance/occurrences sans clone d’index.
  Ce résolveur ne constitue pas le chargeur physique durable ni une vue interrogeable.
- Garde des targets/entrées Cargo communes : `9feb7cdbadef6a60568dd4fc447bafddf7986942`.
  Source qualifiée `9ec421069320b2f0cca4099fa27f83059e944de9`, pilote `65ad0810ca3f4f1496539d5e2945e5c959b94a07`.
  F8 8/8, lib SDK29 2308/0/66, NAV40, 223 gabarits par ELF, deux corps HTTP
  vérifiés par un seul juge. Preuves finales `cdcc032dfde186f9b33fb8d9804ca50fcf2c1a4b`,
  558 fichiers/manifeste `cece3588719044bf461eb6f7f322eb8f9b0db524e2b63e979eb07f782522cb3d`.
  Toute entrée Cargo brute extérieure aux scripts peut contourner leur politique :
  ne pas annoncer une garantie universelle. Nettoyage Recherche fini ; trois refs
  historiques classées archives conservées, pas de suppression forcée.

Le dernier master lu avant collecte était `3a5080d93bdaeba05fd333b25bdd0565b49ad18b`
(documents Lucivy ; produit garde toujours présent). Le commit de cet index
fait foi pour l’état documentaire final, sans fusion des WIP produits.

## Couches 2b-A : sauvegarde prête, qualification union inachevée

Porteur : arbre-principal. Lire son rapport99 pour les dix branches et worktrees
aux deux postes. Ticket ouvert :
`docs/arbre-principal/tickets/2026-11-13-appliquer-la-visibilite-des-couches-avant-les-requetes-des-vues.md`.
Le parent contenu/couches reste également ouvert.

A distingue admission CPU/fixtures, B chargeur durable + liaison
occurrence/offset/P/modèle + snapshot composé DB/FTS/sparse, C accès réel outils.
A ne certifie ni B ni C. Validité conservée jusqu’au rendu, invalidation irréversible
et échéance effective recontrôlée après la dernière lecture. Masque avant top-k,
projection du même ensemble, deux bouts des arêtes, multiplicité préservée.

Preuve A historique `33a64da41c0d68a4fd0653c71167fc8887e41f7d` : APP
`affa6d03b5a57dd99810b6853783dc1a267f76b9`, pilote `c75ad3ae4b437495fb4544045894a986ac23bebb`.
15 exacts + lib CPU 1662/0/6, soit 1662 fonctions distinctes/1677 invocations ;
315 fichiers, manifeste `59bf3e14da0adfc99f1788ff431a6ae4d707602e32f860bb51b654019682d6ba`.

Union initiale APP193d77/pilote0d5 : première F8 rouge en lib hors cfg(test),
8 champs/méthodes de fixture inutilisés sous -D warnings. Correction APP
`d9aa4be86c6f21665cf85c22e47134c87ab6aab8` ajoute 8 cfg(test) ciblés sans
allow global, faux usage ni changement des juges/invariants runtime.

Corrective d9/d7d00 : cache repris correctement ; F8 jeux1/2/3 verts puis
observation Cargo avec sept champs None. Profil non atomique /proc, rawlen absent :
**cause non attribuée**, pas une preuve d’absence de SDK ou de faute produit.
Preuves rouges `237614f8343a196f384d2b01361a8f83e7a0b490`,59 fichiers/manifeste
`fd4e8bb02f79559458341a178e4c7ae740bc992d24a7e95abf773465cc572aab`.
Pas d’ELF/juge union qualifié. Ancienne tentative e6e6 conservée séparément.

Couple courant sauvegardé : APPd9aa ci-dessus et PILOTE
`4fca0dd38378d5bb31269dab76713d6bc0060ebc`. Capture Cargo cohérente préparée :
pidfd, attribution PID/start/UID/cgroup avant signal, SIGSTOP de ses seuls
descendants, tous threads arrêtés/TID stables, snapshot avant assertions,
SIGCONT en finally via le même fd si arrêt initié. SDK7 présents stricts ;
CPU six champs SDK absents et LD présent selon politique de phase explicite.
Un processus devenu non-Cargo est non qualifiant, pas un Cargo validé par défaut.
Refus du module converti en RuntimeError pour ne pas être absorbé par les
handlers OSError du polling ; first-red immuable, cleanup jamais attestant.
Quatre avis statiques favorables,24 fonctions hermétiques/26 cas annoncés.
**Aucun GO ni exécution réelle de 4fca**, aucun P/Q/Sv3 créé.

Target PC conservé `~/.cache/rag3weaver-build/cx-target-contenu-derive`, marqueur
owner `cx-wt-couches-2b-union-cfg/extension/rag3weaver` (d9aa), pas affa/193.
Same-owner doit utiliser la garde commune réelle sans fausse réinvalidation.
Les anciennes autorités v1/v2 sont terminées. La fenêtre historique
918138.385510088→922938.385510088 est enregistrée pour lire les preuves seulement.
À la reprise, refaire revue/créneau/GO/budget neufs et adapter le pilote en
branche neuve avant exécution ; ne pas réutiliser ses constantes historiques.

## Lucivy : barrière correcte dans sa portée, coût à améliorer

Dépôt séparé `/home/lucied/git_workspaces/lucivy`. Dépendance rag3weaver courante
4.3.3 ; aucun tag/publication nouvelle autorisé. Ticket unique CI ouvert :
`docs/orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md`.

CI statique D17 : `native_legacy_foreign_schema_and_values_refuse_owned_open_without_writes`
refuse une réouverture après Drop car un writer reste vivant. Garde C++ writer
identique textuellement entre D17 et SDK29. Pas de corruption ni détenteur
historique définitivement attribué ; le vert local SDK29 ne répare pas la CI.

Bug indépendant établi : shutdown ack avant destruction de l’acteur. Témoin
CPU645b6e6 rouge attendu. Variante B `fbd3f0a4179917e3674fa52a56ffde027831a354`
met une barrière actor+mailbox/envois admis : fermeture d’admission, drainage
hors mutex, attente de tous les envois, reprise de la première panique après
nettoyage ; wrapper avant on_start, garde handler RAII worker/coop/pinned.
Source3b1661446c573ee794b918ec016c0cbff748e4ce ajoute seulement les juges core.
Qualification10 CPU puis lib entière179/0/0 + six core exacts verts, dont
close RAM succès/erreur avec ownership router/shards libéré au retour.
Preuves `613d9362995740bb50f45cccb5861c9e1b3fc85e`,32 SHA relus. Trois noms
non appariables ligne à ligne à cause de nocapture entrelacé ; résumé179 solide,
pas trois échecs et pas prétention179 lignes individuelles.

Mesure A663975adf11fc4a9d6cadf90e70ee68f6c9db075/BFBD :96 ABBA valides,
preuve `a568adb2941dba8cc5782391a4ffb5781c3415db`. Dégradation dans7/8 cas,
worker jusqu’à-40,1%, un cas borné4producteurs +80%. Pinned inclut coût de
diagnostics stderr/devnull. Ce n’est ni une ingestion ni le mutex isolé.
Décision C7 : retenir la fusion de B sous cette forme, conserver ces chiffres.

Alternative C `fa1d0273c468b997163f3cf9d5f56bae052f522a`, branche
`cx-lucivy-barriere-admission-atomique` : bit fermeture+compteur CAS unique,
garde empruntée sans clone Arc par send, mutex/Condvar seulement en fermeture.
Notifier chaque fin pendant fermeture permet le cas capacité1 avec3 envois admis.
Revue ED favorable statique ; tests supplémentaires publiés mais **NON JOUÉS**.
Préparer qualification complète C sur source exacte puis mesure A/C distincte,
pas de replay B inchangé ni d’attribution de résultats B à C.

Les variantes, les deux pipelines de qualification/mesure et preuves sont poussés.
Fusion ordinaire après revue+qualification relève de la coordination autorisée ;
tag v*, release crates/npm/PyPI réclament toujours décision explicite de Lucie.
Ne modifier ni registre Cargo, ni SDK épinglé, ni dépendance rag3weaver pour
maquiller le juge historique. Aucune nouvelle publication réalisée.

## Ressources, règles et suite opérationnelle

- Portable : `/home/lucied/git_workspaces/rag3db`, SSH `luciepc`, même racine distante.
  L’arbre de Lucie est ancien HEAD41497 et garde ses modifications préexistantes
  sous-module codeparsers + lucie-tmp. Ne pas checkout/restore/stage ces données.
- Worktrees et branches cx isolés ; postes et targets propres par lot. Session
  fonctionnelle au plus j8/32GiB, HIGHmax ; `poste lourd`. Mesure CPU exclusive
  avec `poste mesure`, alors600s après acquisition pour le microbanc. Les recettes
  figées de chaque lot règlent les vrais budgets, aucun ancien compteur réutilisé.
- SDK29 épinglé libSHA b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929,
  pool1GiB pour IndexNode. Pas de rebuild/downgrade/pose/activation de base implicite.
- Card2 utilisable si pertinent et coordonné ; autorisation nocturne card0 terminée
  à08h, pas de reconduction. Crédits cloud via Vertex ; Decisions API en attente
  d’accès Vertex, pas de campagne externe improvisée.
- Les autorités Arbrev1/v2 et MESURE Lucivy contrôlées sont terminées. Les
  propriétaires déclarent pas de calcul/verrou propre retenu. Services partagés
  inchangés ; leur présence ne prouve pas un job de développement actif.
- Nettoyage seulement après sauvegardes fidèles des administrations deux postes,
  quiescence et garde COURANTE SHA6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c
  avec rapport frais VERT lu avant chaque retrait. Jamais forcer un refus de garde,
  -D, suppression de remote, GC, sudo ou effacement de source inconnue. Les W/T
  de travaux incomplets sont laissés pour reprise, pas nettoyés par cette passation.
- Les FD devices ne prouvent pas du calcul GPU ; UNKNOWN n’est jamais une observation
  négative. Conserver tous rouges et limites des preuves, pas de faux vert de résumé.

Trois pas de reprise : lire l’index/rapport propriétaire et tickets ; comparer
source/remote/W/PID/verrous actuels sans lancer ; préparer le prochain couple
source/pilote avec budget neuf puis GO explicite avant calcul. Les restaurations
de base/index, poses de service et publications ont leurs propres autorisations.
