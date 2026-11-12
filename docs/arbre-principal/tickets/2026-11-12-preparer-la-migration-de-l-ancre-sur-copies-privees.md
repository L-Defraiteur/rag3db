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

## Prise en charge et plan publié pour revue

Arbre-principal reprend sur worktree documentaire neuf
`/home/lucied/.cache/rag3weaver-build/cx-wt-migration-ancre-plan`, branche
`cx-arbre-migration-ancre-plan`, base **bc34624dc**. Aucun WT distant, target,
service ou accès à une base réelle pour cette étape.

Livrable : [plan et protocole1b](../12-11-2026/01-plan-de-migration-de-l-ancre.md).
Primitives relues : MigrationRunner/journal/checkpoints, SchemaFacts/fournisseur
code, identité dérivée et chunks, KnownVectors/VectorStorage, gardes de config.
Douze témoins proposés, jeu synthétique borné, premier patch d’admission/préflight
séparé de la transformation et de l’activation. Aucun nouveau test joué.

L’[implémentation future](2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md)
a son ticket ouvert avant transmission. Points à relire : format applicatif et
raccord effectif du runner, publication conditionnée, profil dense+BM25 initial
avec refus du sparse non transférable, notion d’équivalence BM25. Le plan ne
transforme ni les garanties actuelles de checksum/undo, ni l’absence d’export
sparse prouvé, en fonctionnalités déjà livrées.

Préparation conservée ouverte jusqu’à l’avis de l’orchestration ; aucune clôture
ni autorisation de code déduite de la seule publication de ce document.

## Première relecture orchestration

Plan52c7ad6ba lu en entier : séparation des formats, reprise/publication et
limites d’undo acceptées. Profil initial BM25+dense/refus sparse avant effets
accepté ; pertinence BM25 susceptible de changer, garanties de couverture,
filtres, positions, provenance et ordre des égalités requises, sans équivalence
complète bit pour bit. Le préflight futur peut lire les métadonnées nécessaires
avant initialize, sans DDL/réparation/modèles ; aucune base réelle lue ici.
ED relit encore le runner/admission/checksum et la frontière du premier patch.
Préparation toujours ouverte, aucun code autorisé pour l’instant.

## Révision après les trois avis ED

Le plan propose maintenant les API communes d’inspection/admission, de résolution
et qualification du plan, la façade d’ouverture sur disque et le point d’entrée
explicite migration plan ; noms proposés, aucune API livrée. Les chemins hôte/CLI
avant RW/modèles sont cités, l’identité résolue et les refus d’historique sont
explicites, ainsi que l’exclusivité réelle et running durable avant effets.
Publication configs/format/génération/journal appliqué atomique sans DDL proposée,
clôture avant ouverture normale et reprise explicite des états ambigus.
Le profil sparse dépend des données/signaux à transférer, pas du seul lecteur
models.sparse. Même ticket d’implémentation, pas de nouveau Rust ni de test joué.
Préparation ouverte jusqu’à validation de cette révision par orchestration.
