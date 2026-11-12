# Paralléliser les suites autorisées sans chevauchement

- **État** : ouvert, répartition autorisée par Lucie le 12 novembre ; prises en charge et preuves à suivre.
- **Famille** : produit
- **Gravité** : coût ; préparation et revues sérialisées alors que des suites indépendantes sont possibles.
- **Atteignable en service** : non, coordination du développement.
- **Touche rag3weaver** : oui, répartition IDE, mémoire et migration des vues.
- **Pour** : orchestration ; ED, archi, mémoire et arbre-principal pour leurs lots.
- **Signalé par** : Lucie, « oui stp pour mieux paralléliser, tant que vous vous emmêlez pas les pinceaux pour bosser ».

## Constat et décision

Échantillon de deux secondes sur luciepc : 24 processeurs logiques, CPU10,3%,
attente disque0,1%, mémoire disponible70,6Gio, poste lourd libre0/4. Le build C++
privé01 venait de s'arrêter sur une qualification manquante, diagnostiquée puis
corrigée ; build02 terminé en154,588s selon son propriétaire. Ces observations
ponctuelles ne décrivent pas l'occupation de toute la journée.

Ouvrir les suites cohérentes ci-dessous en parallèle. Chaque propriétaire complète
son ticket avant de coder, travaille depuis master dans un worktree cx distinct,
publie son périmètre et les fichiers possédés. Les corrections de montage précisément
diagnostiquées peuvent recevoir une autorisation conditionnelle d'exécution ; pas
d'aller-retour supplémentaire si les conditions convenues sont satisfaites.

| Propriétaire | Suite et ticket canonique | Frontière |
|---|---|---|
| ED | [Accord de commande hors domaine](../../everything-declarative/tickets/2026-11-11-un-accord-de-commande-hors-domaine-redemande-le-meme-accord.md) | Corriger la reprise de la ligne explicitement accordée, avec sa portée vérifiée et des témoins de refus des variantes. Possède Rust et tests backend de cette correction. |
| Archi | Même ticket d'accord | Possède le témoin navigateur du clic réel, de la reprise et de wait_output ; coordonne le contrat avec ED, ne modifie pas son code Rust. |
| Mémoire | [M8](../../memoire/tickets/2026-11-07-une-memoire-ne-sait-pas-que-ses-gabarits-sont-en-retard.md) | Première tranche de code autorisée : plan à blanc sur contenus fournis, fusion pure et refus structurés selon la revue canonique. Aucun apply, adoption persistée, montée réelle ou modification de politique vivant/épinglé. Réutiliser les validateurs ; signaler avant modification une primitive partagée nécessaire. |
| Arbre-principal | [Migration de l'ancre](../../arbre-principal/tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md) | Préparer maintenant la prochaine tranche de transfert générique chunks/vecteurs BM25+dense sur fixtures synthétiques : API, invariants, inventaire et témoins. Pas encore de transfert exécuté, publication, apply ou migration réelle ; revue du périmètre avant code de cette tranche. |

Le diagnostic C++ continue sous son ticket moteur existant et son paquet privé.
Il ne modifie aucune bibliothèque distribuée. Une interruption système ou un refus
dans une session ne se contourne pas en transférant l'action à une autre.

## Ressources et intégration

Préparation et code indépendants avancent ensemble. Les compilations utilisent
poste lourd, un target privé par lot, -j8 et32Gio au plus par lot Rust ; respecter
la capacité4 et la pression réelle de mémoire/disque. Une mesure sous poste mesure
reste exclusive et prioritaire ; son budget démarre après acquisition. Pendant
ce créneau, les autres sessions continuent leurs lectures, code et revues.
Pas de GPU, service, modèle ou base réelle nouveaux pour remplir la machine.

Un fichier commun a un propriétaire annoncé ; si deux lots en ont besoin, définir
le petit contrat commun et son ordre d'intégration avant de le modifier à deux.
Les lots restent séparément relisibles et testables ; l'orchestration tient la file
des passes et pushes. Un rouge exige diagnostic, aucun rejeu inchangé pour occuper
le CPU. Les lots terminés attendent normalement, sans travail artificiel.

## Pour fermer

Consigner les prises en charge, frontières de fichiers et dépendances ; confirmer
la reprise effective des suites possibles, ou leur attente motivée. Les tickets
fonctionnels restent ouverts jusqu'à leurs propres critères de fin ; ce ticket
ne prouve aucune fonctionnalité livrée.
