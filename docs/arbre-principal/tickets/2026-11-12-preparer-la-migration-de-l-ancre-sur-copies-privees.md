# Préparer la migration de l’ancre sur copies privées

- **État** : ouvert, préparation documentaire après intégration du socle 1a ; aucune migration ni activation autorisée par ce ticket.
- **Famille** : produit
- **Gravité** : planification ; rendre le partage de contenu utilisable dans les vues de code sans transformer silencieusement les index existants.
- **Atteignable en service** : non pour cette préparation ; les gabarits et bases de service gardent leur comportement actuel.
- **Touche rag3weaver** : oui, contrat de migration, déclaration du gabarit code et protocole de validation à définir.
- **Pour** : arbre-principal, revue orchestration et ED selon le mécanisme existant.
- **Signalé par** : orchestration, suite du cap de Lucie et du lot 1b déjà défini dans docs/arbre-principal/11-11-2026/03-contrats-et-lots-des-vues.md.

## Point de départ

Socle 1a intégré sur master08086ba36 ; partage contenu/recette/cellule et projection des
occurrences déclarables par manifeste, cinq tickets clos avec leurs preuves. Cela ne bascule
ni Scope du gabarit code en production, ni les bases et binaires des deux postes. La pyramide
de couches et les mesures M3–M5 restent distinctes.

## Travail autorisé maintenant

Préparer un plan concret à partir des contrats du dépôt et des primitives déjà présentes :

- identifier le mécanisme de version/migration déclarée existant et les frontières manquantes ;
- décrire la déclaration du code (occurrence, contenu, références et index public), les invariants
  d’arêtes et de positions, sans changer les schémas des parseurs ;
- expliciter reconnaissance de l’ancien format, refus de l’ouverture non migrée et migration
  observable, récupérable et répétable sur des copies privées ;
- donner les témoins avant/après : couverture des occurrences et relations dans les deux sens,
  recherches et filtres, zéro nouvel embarquement, reprise après interruption, annulation ;
- chiffrer les besoins de test et préciser le périmètre du premier correctif reviewable.

Lecture et préparation documentaire uniquement à cette étape : pas de code produit, de copie
ou lecture du contenu des bases réelles, de mesure M3–M5 ni de nouveau service/modèle.
Un manque découvert a son ticket ou complète celui-ci avant transmission. Un blocage réel
se nomme ; ne pas inventer une abstraction si un mécanisme existant porte déjà le contrat.

## Critères de fin de cette préparation

Plan et protocole concrets publiés sur branche cx et relus par orchestration ; chemins,
contrats et preuves attendues nommés. Ce ticket de préparation peut alors être clos avec son
livrable, tandis que le ticket d’implémentation restant reste ouvert. Le document seul ne
constitue ni une migration livrée, ni une autorisation d’activer une base de Lucie.
