# Revoir les services d'embarquement résidents selon les usages

- **État** : fait 7caaad4ecc34f4c0aaf6cbad283de15d75d22d6d ; audit, arrêts et recettes de retour attestés, conception et suivis futurs consignés.
- **Famille** : produit
- **Gravité** : coût ; VRAM occupée par des services dont l'utilité dans le flux courant doit être vérifiée.
- **Atteignable en service** : oui, processus 7878/7879/7880/7882 résidents sur card2.
- **Touche rag3weaver** : oui, services d'embarquement, appelants et cycle de vie des modèles.
- **Pour** : embarquement pour les usages et primitives ; optimiseur pour les opérations ; orchestration pour la consolidation.
- **Signalé par** : Lucie : vérifier la pertinence des services maintenus, décharger les usages absents du flux traditionnel et envisager un mécanisme de warming.
- **Ouvert le** : 12 novembre 2026

## Demande

Utiliser card2 et les modèles quand ils sont utiles, tout en évitant une
accumulation de résidents sans usage. Relier chaque service à ses appelants
effectifs et prévus, aux espaces d'embeddings existants et aux ressources
mesurées. Une absence de connexion instantanée ne suffit pas à établir qu'un
service est inutile au flux courant.

Au relevé lecture seule de 18:40 Paris : 7878 et 7882 annoncent granite-278m
(768, Flex32), 7879 bge-m3 (1024, dense/creux, avec relecteur et OCR annoncés),
7880 granite-107m (384, Flex32). Trois unités utilisateur et un privé 7882.
Card2 PCI04:00.0 : 25 680 760 832 octets utilisés sur 34 208 743 424.
La santé annonce des capacités, pas la résidence effective de chaque sous-modèle.
Le banc k10 est terminé et 7882 vient d'être restauré ; aucun lourd en cours.

## Travail attendu

1. Inventorier PID/unité, identité de modèle, VRAM par processus/PCI, usages
   documentés, configurations non sensibles et clients réels. Distinguer
   flux habituel, essais ponctuels, anciennes réservations et doublons privés.
2. Décharger les services réellement inutilisés après contrôle et remise du
   porteur, avec recette de redémarrage conservée. Ne pas remplacer un modèle
   d'index par un autre pour libérer de la VRAM : identité, dimension et
   capacités dense/creux appartiennent au contrat de l'index.
3. Relire les primitives existantes de chargement, activation, arrêt et
   résolution des modèles ; préciser ce qui est déjà paresseux ou déchargeable.
   Concevoir l'usage résident/à la demande et le préchauffage anticipé depuis
   ces contrats, avec demande concurrente, annulation et erreur observables.
4. Publier décisions et preuves, et ouvrir le suivi de toute capacité future
   manquante avant d'en transmettre l'implémentation. La conception seule ne
   devra pas être décrite comme un mécanisme de warming livré.

## Fin attendue

Tableau des services et usages, ajustements réellement effectués et gain de
VRAM mesuré, services conservés et raison, recettes de retour, limites des
preuves d'activité et proposition de cycle de vie reliée aux primitives.
Les travaux futurs auront leur ticket propre ; clore cet audit une fois ce
bilan et ces actions terminés. Aucun arrêt de seat1 ni suppression de modèles
ou de données demandé.

## Réalisation du 12 novembre

[Bilan](../../embarquement/12-11-2026/01-audit-residence-et-prechauffage.md) :
7882 privé arrêté à 18:44:52, 107m arrêté puis instance passée au démarrage
manuel à 18:54:16. 8,637 Gio libérés, deux services partagés conservés avec
leurs PID et santés. Les recettes de retour et les preuves aux deux postes
sont conservées sous `~/.cache/optimiseur-notes/audit-residence-12nov/`.
Aucun modèle/donnée effacé ni changement d'espace vectoriel.

La [conception](../../orchestration/visions/2026-11-12-residence-des-modeles-et-prechauffage.md)
compose les primitives existantes. La gestion automatique reste au
[ticket de capacité ouvert](../../embarquement/tickets/2026-11-12-gerer-activation-et-residence-des-modeles.md) ;
les déclarations BGE/portgranite ont leur suivi séparé. Ces suites ne sont
pas déclarées livrées par la clôture de l'audit.
