# Résolution : idées à conserver et retour au cas Magic

Date : 19-09-2026. Suite au [design initial](06-design-optimisation-adaptative-des-acces.md) et à sa [revue](07-design-review-gpt-pro.md).

**Décision : conserver les directions suivantes, sans lancer leur implémentation maintenant. Reprendre l'amélioration du deck Magic avec les outils existants.** Les propositions ci-dessous ne sont pas des capacités déjà disponibles ni une promesse de calendrier.

## 1. Une recherche conserve son identité quand son exécution change

Le besoin métier — « les cartes de mon deck liées au cimetière » — doit rester identifiable si son exécution passe d'un scan à un parcours relationnel ou à un index. Les usages déclarés référencent une recherche ou un fragment ; ils ne deviennent pas un second langage de requêtes.

Ancrage : [GraphTool](../../src/dataflow/graph_tool.rs) fournit déjà des graphes nommés et paramétrés ; [FilterCondition](../../src/filter.rs) fournit les prédicats, dont les contraintes imbriquées. Mais nos DAG décrivent des opérateurs et des choix d'exécution : une représentation logique commune reste à construire. Conserver le hash structurel de [GraphDefinition](../../src/dataflow/checkpoint.rs) pour vérifier les reprises ; ajouter séparément une identité d'usage et une version de contrat. Ne pas prétendre déduire automatiquement l'équivalence de graphes arbitraires.

## 2. Des résultats porteurs de leurs garanties

Un ensemble de résultats devrait annoncer sa portée, son exhaustivité ou sa troncature, le domaine de son classement, l'état des données observé et les preuves demandées. Cela permettrait au framework de refuser une optimisation qui change la question, même si elle renvoie des objets du bon type.

Exemple Magic : sélectionner les meilleures cartes globales puis garder celles possédées n'est pas équivalent à chercher les meilleures cartes dans la collection. Une fusion doit aussi préserver les liens capacité → carte qui justifient ses scores.

Ancrage : [NodeSchema](../../src/dataflow/node_registry.rs) décrit ports et paramètres, sans ces garanties. [SearchMeta](../../src/search.rs) fournit déjà cohérence, disponibilité et diagnostics ; `partial = false` ne prouve pas l'exhaustivité du corpus. Étendre ces bases progressivement, avec sémantique inconnue par défaut pour les nœuds personnalisés. Distinguer garanties statiques et faits constatés à l'exécution.

## 3. Un framework capable de choisir une configuration pour ses usages

La vision dépasse « créer l'index d'une requête lente » : choisir un ensemble de ressources qui sert plusieurs parcours, dans un budget de mémoire, de stockage et d'écriture. Deux accès peuvent se compléter ou faire double emploi. Le planificateur utilise les ressources disponibles ; le contrôleur décide lesquelles devraient exister.

Garder cette abstraction assez ouverte pour de futures matérialisations, sans les inclure dans une première implémentation. Commencer par comparer un candidat à la configuration réellement présente ; une recherche globale de configuration optimale serait prématurée.

Ancrage : [SearchBackend](../../src/search_backend.rs) distingue déjà certaines capacités, recherches filtrées et récupération d'objets depuis leurs identifiants. Il manque des descriptions composables des préconditions, de l'ordre et de la projection fournis. Ne pas recréer l'optimiseur interne de rag3db, ni annoncer un index composite dont la disponibilité n'a pas été établie.

## 4. Optimiser le service rendu, avec une décision explicable

Les objectifs de latence appartiennent au parcours complet exposé à l'utilisateur. Accélérer une branche non limitante peut économiser des ressources sans accélérer la réponse. Une recherche rare mais critique doit pouvoir être prioritaire.

Chaque décision devrait conserver la configuration initiale, les mesures, les objectifs, les alternatives, la configuration désirée et les raisons du choix. Séparer trois diagnostics : recherche correctement réalisable, accélération disponible, objectif mesuré et atteint. Le statut unique `unsupported` du document initial est trop ambigu.

Ancrage : [ExecutionReport](../../src/dataflow/report.rs) possède déjà durées globales et par nœud. Il reste à construire l'agrégation de charge et les mesures de chemin critique ; additionner les durées de branches parallèles serait faux. Éviter de conserver tous les payloads pour une simple télémétrie de performance.

## 5. Le framework applique ses décisions avec ses propres graphes

Décrire un état désiré, observer l'état réel et appliquer un DAG de transition : construire → valider → publier → retirer. Réutiliser le runtime, l'observabilité et les mécanismes de reprise plutôt que créer un moteur de workflows parallèle.

Ancrage : [migrations](../../src/dataflow/migrations.rs) et [runtime](../../src/dataflow/runtime.rs) donnent une base, pas une garantie d'exécution exactement une fois. Le checkpoint de fin de nœud est sauvegardé après son exécution : une opération à effet de bord doit supporter le rejeu. Le verrou de migration dans [catalog.rs](../../src/catalog.rs), avec TTL fixe de dix minutes et acquisition par lecture puis écriture, n'établit pas à lui seul un verrou renouvelable et protégé contre les anciens propriétaires pour des constructions longues et concurrentes. Ces constats viennent de la lecture du code, pas d'un test de panne.

Les migrations séquentielles de schéma et la réconciliation adaptative de ressources restent deux usages distincts, même s'ils partagent leur runtime.

## Suite immédiate

Reprendre `mycotyrant_insidious TRIM (2)` : vérifier la liste et son format dans le snapshot local, analyser courbe, mana, auto-meule, réanimation et production de jetons ; chercher des remplacements possédés en conservant texte et preuves. Proposer une version à tester, sans modifier le deck Arena ni fabriquer de nouvelles cartes. Les difficultés concrètes rencontrées pourront ensuite guider les extensions du framework.
