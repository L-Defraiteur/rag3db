# Collection complète : ingestion réelle et correction des listes natives

Date : 19-09-2026. Suite à la demande de rétablir l'usage effectif du moteur pour les recherches Magic.

## Écart corrigé

Les variantes précédentes avaient été construites par lecture Python des exports, alors que l'objectif était d'utiliser rag3weaver. Les tests composables existants n'ingéraient qu'un corpus réduit. Cette distinction n'avait pas été rendue suffisamment claire.

La collection possédée du snapshot `e44fdd1e50b8916e9d7e` est désormais ingérée dans une base rag3db persistante, avec lucivy et le service BGE-M3 local réel (1024 dimensions, non factice). Aucun nouveau relevé mémoire n'est revendiqué.

Manifest et procédure : [backend Magic](../../../../experiments/mtga/backend/README.md). Preuves : [validation](../../../../experiments/mtga/data/engine-validation.json), [rapports d'ingestion](../../../../experiments/mtga/data/engine-ingestion-progress.json), [sorties moteur](../../../../experiments/mtga/data/engine-results).

- 5 380 impressions possédées, 10 352 exemplaires, dont 556 entrées terrain.
- Ingestion par lots de 128, environ 135 secondes dans cette exécution.
- Fermeture du backend, réouverture de la même base, puis recherches sans réingestion.
- Comparaison de tous les champs exportés des 5 380 records, pas uniquement des comptes.
- Les données non possédées, decks et relations de glossaire ne sont pas ajoutées à cette base par cette opération. Les capacités natives figurent comme tableaux structurés sur les cartes.

## Écriture par lots dans le framework

`EntityBatchNode` permet l'ingestion de snapshots externes via un graphe exposé par le manifeste. Validation JSON Schema de tout le lot avant écriture, identités explicites, rejet des doublons dans le lot, maximum 512 objets. Refus des politiques de révision, dates serveur, immutabilité et lifecycle pour ne pas contourner les garanties de l'écriture métier.

Le schéma du paramètre `records` annonce explicitement un tableau : le type `json` seul était exposé comme objet. Une erreur de stockage/indexation après validation peut laisser un état partiel ; aucune atomicité multi-index n'est promise.

## Problèmes révélés par l'usage réel

### Filtres de listes et recherche dense

`has_any` / `has_none` étaient traduits en lambdas de listes. Un parcours provoquait un crash natif ; le parcours vectoriel projeté refusait aussi une capture avec `Lambda paramName p.color_identity cannot found`.

Le dialecte rag3db expose maintenant une forme scalaire de membership. Le parseur compile les valeurs demandées en `list_contains` paramétrés reliés par OR/AND/NOT. Les cas vides sont explicites. PostgreSQL conserve ses opérateurs de tableaux. Les valeurs utilisateur ne sont pas interpolées dans le texte de requête.

### Quantificateurs sur grandes listes

Dans `quantifier_functions.cpp`, l'évaluation prenait la taille totale du vecteur enfant sans respecter les tranches préparées par `ListSliceInfo`. Les listes de capacités du corpus complet dépassaient la capacité des évaluateurs intermédiaires.

Correction native : évaluer chaque tranche, accumuler les correspondances par position de liste parente, puis finaliser ANY/ALL/NONE/SINGLE une fois toutes les tranches traitées. Les compteurs sont réinitialisés à chaque évaluation ; les listes vides et nulles sont traitées séparément.

### Champs de structures désynchronisés

Le remplacement provisoire de ANY par LIST_FILTER évitait le crash mais produisait 40 résultats au lieu des 71 attendus : 58 omissions et 27 faux positifs.

La transition de tranche affectait directement `ValueVector.state` sur le vecteur structuré, sans propager le nouvel état à ses champs. Les évaluateurs de `ability.text_en` pouvaient donc lire une autre position. `ValueVector::setState`, déjà présent, effectue cette propagation récursive. Il est désormais utilisé à l'initialisation, au changement de tranche et à la restauration.

Après correction, rag3weaver utilise de nouveau son expression ANY d'origine pour le filtre imbriqué. Le remplacement provisoire par LIST_FILTER a été retiré.

## Vérifications

| Sélection exhaustive dans le moteur | Attendu | Obtenu |
| --- | ---: | ---: |
| Inventaire | 5 380 | 5 380 |
| Type Creature | 2 764 | 2 764 |
| Terrains pouvant produire G, selon projection documentée | 156 | 156 |
| Terrains pouvant produire R et B, selon projection documentée | 52 | 52 |
| Texte Treasure, hors terrains, identité incluse dans rouge/noir | 71 | 71 |
| Même recherche dans un élément du tableau abilities | 71 | 71 |

Chaque ensemble est comparé par identifiants : zéro omission, zéro faux positif. Les recherches classées BM25, dense et hybride renvoient chacune vingt résultats respectant les restrictions ; les ports de métadonnées conservent les avertissements du moteur. Ceci vérifie le fonctionnement et le périmètre, pas une qualité de classement parfaite ni l'exhaustivité de l'ANN.

Tests natifs : suites lambda existantes et nouvelle régression `structured_slices.test` avec 5 000 éléments, listes de structures, sélections de lignes non contiguës, contraintes sur un même élément, quantificateurs et listes vides. Quatre suites lambda passées, puis les trois tests de prédicats existants (dont constantes et NULL) passés. Tests rag3weaver : 978 passés hors bac à sable ; les erreurs réseau rencontrées dans le bac à sable ne sont pas masquées.

## Filtres de terrains et limites restantes

Le schéma sépare `card_types`, `is_land`, `has_land_face`, `colors`, `color_identity` et les symboles de mana possibles/explicites. Les conditions de production restent attachées au terrain. Exemple : Wastewood Verge a une condition sur B ; un Thriving choisit sa couleur supplémentaire ; Deceptive Landscape produit C et ne produit pas directement W/B/G.

Cette projection du mana repose sur le texte et les types de terrains de base, pas sur un moteur de règles exhaustif. Une égalité des résultats avec la projection vérifie les filtres, sans prouver que tous les terrains dynamiques sont parfaitement interprétés.

Le script d'ingestion fait des upserts mais ne réconcilie pas encore les suppressions d'un nouveau snapshot. Une nouvelle capture exige cette étape avant de revendiquer une synchronisation complète. Le filtre imbriqué a été vérifié sur la sélection exhaustive ; son intégration à chaque forme de graphe vectoriel projeté ne doit pas être déduite de ce test.
