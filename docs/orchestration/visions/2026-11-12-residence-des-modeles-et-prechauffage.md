# Résidence des modèles et préchauffage selon l'usage

12 novembre 2026. Demande de Lucie : utiliser les modèles utiles sur card2,
sans conserver indéfiniment tous les services d'essai en VRAM. Cette note
prévoit le cycle de vie ; elle ne livre pas d'activation ou d'éviction automatique.
[Audit des services](../../embarquement/tickets/2026-11-12-revoir-les-services-d-embarquement-residents.md),
[suivi de la capacité](../../embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md).

## Ce que l'on compose déjà

- `ModelSource` exprime modèle, fournisseur, adresse, capacité et repli explicite ;
  ses connecteurs locaux construisent le modèle et ses connecteurs de service
  joignent un fournisseur déjà disponible. La forme `service` ne démarre pas
  le fournisseur distant.
- `Serveur::assurer` sait sonder une identité, démarrer un processus et attendre
  une réponse bornée, avec journal et erreurs nommées. `DaemonEmbedder::assurer`
  le réutilise. `Attache` distingue l'enfant lancé par le client du service
  d'autrui ; sa politique de fin conserve cette propriété.
- Le serveur d'embarquement charge l'embedder et les éventuels relecteur/OCR
  avant écoute. Il chauffe déjà des formes typiques de l'embedder au démarrage.
  Il conserve ensuite ses `Arc` jusqu'à sa fin. BGE dense/creux/dual partagent
  le même objet ; relecteur et OCR sont d'autres objets dans le même processus.

Points source : `src/model_source.rs`, `src/serveur.rs`,
`src/daemon/embeddings.rs` et `src/bin/rag3weaver-embeddings.rs`.
Le simple lancement n'appelle donc pas un deuxième démon à inventer.
Les manques sont la coordination commune des usages et la politique de résidence,
notamment entre plusieurs clients et sur un fournisseur distant.

## Le contrat recherché

Le code, la mémoire et les autres traitements demandent une capacité avec son
identité de modèle. La déclaration exprime la politique ; le runtime fournit
activation, acquisitions et libération. Les agents composent ces capacités.

L'identité du modèle et de sa représentation reste distincte de l'adresse et
de l'instance qui le sert. Un modèle froid doit redevenir disponible avec la
même identité, dimension et capacités requises. Une autre famille de vecteurs
n'est pas une solution de remplacement implicite ; le repli reste explicite.

Trois comportements à exprimer par configuration, noms encore à choisir :

| Politique | Comportement proposé |
|---|---|
| Résident | Maintenir disponible un modèle du flux habituel ; lui réserver une part du budget. |
| À la demande | Charger à la première acquisition puis décharger après une période réellement sans usage, selon le budget et le coût de reprise. |
| Préchauffage anticipé | Donner un indice de besoin avant un traitement connu : ouverture d'un espace, recherche prochaine, document à OCR. Réutiliser le même chargement que la demande réelle. |

Le nombre de processus n'est pas un quota d'autorisation. La décision dépend
du budget de VRAM, des usages actifs, du délai admissible et du coût de reprise.
Les durées d'inactivité et marges restent des politiques à mesurer, avec de
l'hystérésis pour éviter charger/décharger à chaque requête.

## Acquisition, concurrence et arrêt

Une acquisition rend une capacité prête à utiliser et un bail d'usage. Sur le
poste qui porte la ressource, une seule activation par identité/profil et
périphérique rassemble les demandes concurrentes ; un mutex propre à chaque
client ne suffit pas entre processus. Il faut qualifier la recette et le
propriétaire autorisé à lancer ou arrêter le service.

Un appel annulé rend son usage sans tuer les autres. Les opérations en cours,
les acquisitions en attente et les chargements actifs empêchent une éviction
non coordonnée. Quand le dernier usage est rendu, la politique décide de la
résidence ou du délai avant déchargement. Un service externe rejoint par URI
n'est pas arrêté simplement parce qu'un client le quitte.

La granularité initiale doit correspondre au mécanisme réel : les unités
actuelles se libèrent par processus. Le groupe BGE/relecteur/OCR ne devient
pas déchargeable composant par composant par un changement de vocabulaire.
Une éventuelle séparation ou charge paresseuse par capacité devra préserver
les contrats et les usagers déjà attachés.

## Chauffe et attente visibles

Distinguer absence, chargement, service disponible, chauffe et usage ; une chauffe
partielle peut être décrite comme telle sans effacer l'identité fonctionnelle
du service. Conserver les formes/capacités effectivement chauffées, les erreurs
et la durée. La chauffe actuelle ne porte que sur l'embedder et son compteur
ne décrit pas les erreurs de chaque classe ; ce n'est pas une preuve que
relecteur et OCR sont chauds.

Un traitement qui attend une activation doit le dire. Le budget de chargement
et celui de l'appel calculé ont des rôles distincts ; un service froid connu
ne doit pas être présenté comme un fournisseur muet ou comme une réussite vide.
Le préchauffage sert notamment à éviter de faire payer tout le démarrage à
une recherche interactive dont le délai est court.

L'état commun devrait permettre de connaître identité/profil, propriétaire,
périphérique PCI, usages en cours, dernière utilisation et coûts observés de
chargement/chauffe/résidence. Les mesures par processus et les estimations par
modèle doivent être distinguées. Une socket ouverte ne mesure pas l'utilité
d'un service et l'absence de compteur ne constitue pas une valeur nulle.

## Première tranche à concevoir et qualifier

Conserver le routage `ModelSource` et réutiliser les descriptions `Serveur`.
Ajouter une frontière commune d'acquisition gérée avant la connexion au modèle,
avec politique de résidence et activation contrôlée sur le bon poste. Pour les
services actuels, composer les unités existantes et leurs sondes, plutôt que
dupliquer le contrôle des processus dans chaque outil ou interface.

Preuves attendues : deux clients simultanés ne chargent qu'une instance,
identité et capacité contrôlées avant usage, une annulation n'interrompt pas
l'autre client, une ressource utilisée n'est pas évincée, le modèle peut être
déchargé puis redemandé sans changer d'espace, et les erreurs de chargement ou
de chauffe restent visibles. La topologie portable/luciepc fait partie du contrat.
Les opérations manuelles de l'audit ne valent pas livraison de cette tranche.
