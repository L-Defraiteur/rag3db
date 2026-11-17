# Consigner les catégories nommées et leurs références dans les visions

- **État** : fait, vision et index publiés en1d7971592a le17novembre2026 ; demande documentaire réalisée, aucune implémentation revendiquée.
- **Famille** : produit
- **Gravité** : planification ; conserver une direction de conception sans la présenter comme une fonctionnalité livrée.
- **Atteignable en service** : non pour ce lot documentaire ; future capacité de mémoire structurée.
- **Touche rag3weaver** : oui, vision des fiches, des catégories partagées et des filtres par références.
- **Pour** : orchestration.
- **Signalé par** : Lucie, fichier fourni « Recommandations horreur lovecraftienne.md », puis demande « ajouter aux visions » et clarification des catégories comme fiches recherchables.

## Demande

Retrouver le passage sur les enums nommés et conserver son idée dans une vision
indexée. La clarification de Lucie prime : les déclarations de catégories sont
elles-mêmes des fiches recherchables. Les autres fiches peuvent référencer une
catégorie ou une de ses valeurs ; filtrer doit être possible sur les deux.

Les passages sources utiles commencent aux lignes1060 et1254 ; la précision
de périmètre de la ligne1918 distingue les simples déclarations référencées
d'une interprétation libre de nouveaux langages. Ne pas publier le reste de
la conversation personnelle ni en tirer une autorisation d'implémentation.

## Critères de fin

- Vision autonome, avec la provenance et la clarification actuelle de Lucie.
- Déclarations recherchables, références et filtres catégorie/valeur explicités.
- Lien dans l'index des visions ; document présenté comme futur et non livré.
- Vérification des liens locaux et des en-têtes ; commit publié sur la branche.

Aucun code, schéma effectif, migration, test natif ou service à modifier.

## Réalisation

Vision [Des catégories nommées qui sont elles-mêmes des fiches](../../orchestration/visions/2026-11-17-des-categories-nommees-comme-fiches-de-memoire.md),
indexée dans le README des visions, commit1d7971592a publié sur la branche
cx-orchestration-reprise-16nov-r1. Le document reprend la distinction de Lucie
entre définition recherchable, référence de catégorie et référence de valeur.
La conversation complète et ses éléments personnels ne sont pas recopiés.

Contrôle des fichiers et du lien d'index VERT ; deux filtres distincts et statut
de vision future relus, gitdiffcheck VERT. Ce ticket ferme la consignation,
pas une future capacité de mémoire structurée, qui n'a pas été implémentée.
