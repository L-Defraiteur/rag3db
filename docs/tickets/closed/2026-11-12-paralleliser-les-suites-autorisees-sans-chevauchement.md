# Paralléliser les suites autorisées sans chevauchement

- **État** : fait, décision6e5f9458c et quatre prises en charge confirmées le 12 novembre ; répartition et frontières établies, tickets fonctionnels toujours ouverts.
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

## Prises en charge et frontières confirmées

- ED : WT/branche `cx-ed-accord-hors-domaine`, base6e5f9458c ; commande.rs,
  dataflow/run_nodes.rs et tests Rust. Rouge6cbbb22a9 puis correctif7e95d5b7a
  rapportés ; leurs résultats ne sont pas certifiés par cette clôture.
- Archi : WT `cx-wt-accord-exterieur`, branche `cx-archi-accord-exterieur-1`,
  base6e5f9458c ; contrat59a55ced3, témoin navigateur5611fc98e rapportés.
  Possède chat_contextuel.rs ; aucun Rust de production ni React annoncé.
- Mémoire : branche `cx-memoire-m8-plan`, base6e5f9458c, reprise74bced6fb.
  Possède fusion.rs/template_plan.rs/lib.rs et raccord commun minimal
  lecteur.rs/builtin_templates.rs/backend.rs/foreach_node.rs. ED confirme ne pas
  toucher backend.rs. Les sources fermées ne doivent pas substituer le builtin
  courant à une version fournie ni affaiblir les contrôles d'ouverture ordinaires.
- Arbre : WT `cx-wt-migration-transfert`, branche `cx-arbre-migration-transfert`,
  base6e5f9458c ; plan21d09bf82 relu. Premier diff pur aperçu autorisé ensuite,
  migrations/transfer.rs et déclaration/tests uniquement ; ciblé600s sous poste,
  mêmes limites. Source étrangère ne se refuse que face à une identité attendue
  explicite ; aucun producteur durable d'identité inventé. Pas de transfert exécuté.

Les quatre sessions ont confirmé leur reprise ; cinq sessions de chantier
affichées actives avec le diagnostic C++. Paquet C++ build02 vérifié séparément :
834 fichiers/110723932octets rehashés sur luciepc sans écart, neuf dépendances
directes et trois ELF relus. Un scénario borné a été autorisé conditionnellement
après contrôle du driver ; son lancement n'est pas attesté ici.

La répartition demandée est réalisée. Les tickets fonctionnels restent ouverts
jusqu'à leurs propres critères de fin ; cette clôture ne livre aucune capacité.
Les preuves d'audit restent hors worktrees dans
`~/.cache/orchestration-notes/statut-12nov-1405/cpu-et-coordination.json`.
