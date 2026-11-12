# Comparer Dagster et les décideurs RAG libres

- **État** : en cours, revue documentaire demandée par Lucie le 12 novembre 2026.
- **Famille** : produit
- **Gravité** : planification ; repérer les modèles réutilisables et les différences de périmètre.
- **Atteignable en service** : non, étude sans modification ni lancement de modèle.
- **Touche rag3weaver** : oui, traitements déclaratifs, indexation, mémoire et décisions de rappel.
- **Pour** : orchestration.
- **Signalé par** : Lucie, lien https://github.com/dagster-io/dagster et demande de projets libres voisins.

## Périmètre

Comparer les mécanismes documentés par leurs auteurs à notre socle et à ses chantiers ouverts.
Examiner Dagster, les moteurs d'indexation incrémentale et quelques projets de mémoire/agents.
Distinguer classement de passages, décision de rechercher, choix de stratégie et évaluation
du résultat ; rechercher des implémentations de petits modèles réellement dédiés à ces décisions.

Aucun paquet, poids, service, banc ou changement d'architecture n'est autorisé par cette étude.
Les sessions de validation et leurs ressources continuent leur travail indépendamment.

## Critère de fin

Restitution sourcée à Lucie : rapprochements, différences, pistes prioritaires, limites des preuves.
Les hypothèses de transfert restent distinctes des capacités livrées et des mesures locales.

## Résultat de la lecture du 12 novembre

Dagster : assets persistants, dépendances, versions de code/données, conditions d’automatisation
et contrôles explicites. Les Components permettent des DSL YAML. Inspiration : représenter le
résultat attendu et expliquer pourquoi il doit être recalculé, au-delà d’un simple historique de runs.
Sources officielles : [assets](https://dagster.io/docs/guides/build/assets),
[versions](https://dagster.io/docs/guides/build/assets/asset-versioning-and-caching),
[automatisation](https://dagster.io/docs/guides/automate/declarative-automation),
[composants](https://dagster.io/docs/guides/build/components).

CocoIndex : état cible déclaré, transformations et mémoïsation incrémentale des calculs.
Ses [concepts](https://cocoindex.io/docs/programming_guide/core_concepts/) et
[clés de mémoïsation](https://cocoindex.io/docs/advanced_topics/memoization_keys/) sont proches
du chantier contenu partagé/invalidation. Ses environnements isolés ne prouvent pas des vues Git.
Rapprochement précis : [ticket Code170](https://github.com/cocoindex-io/cocoindex-code/issues/170)
et [PR171](https://github.com/cocoindex-io/cocoindex-code/pull/171), ancre commune/couches
branche et dirty/tombstones. API relue : PR fermée le27juin2026, merged=false, aucun merge_commit.
L’auteur annonce l’abandon le1juillet dans les commentaires du ticket fermé. Ne pas annoncer
ce mécanisme livré ; le code de la PR n’a pas été exécuté ni audité. Examiner nos propres
invariants : identité de contenu/occurrence, masquage avant top-k, coordonnées, cycle de vie.

Voisins complémentaires :
[Haystack](https://docs.haystack.deepset.ai/docs/pipelines), composants et graphes RAG avec
[sérialisation YAML](https://docs.haystack.deepset.ai/docs/serialization) ;
[LangGraph](https://docs.langchain.com/oss/python/langgraph/persistence), état durable,
interruptions/reprise, avec [effets rejoués à rendre idempotents](https://docs.langchain.com/oss/python/langgraph/interrupts) ;
[Graphiti](https://github.com/getzep/graphiti), épisodes sources et faits temporels ;
[Letta Code](https://github.com/letta-ai/letta-code), agent de code et mémoire Git,
dont le [sous-agent mémoire](https://github.com/letta-ai/letta-code/blob/main/src/agent/subagents/builtin/memory.md)
travaille dans un worktree privé. Ces proximités partielles ne valent pas équivalence d’architecture.

Décideurs RAG : [CRAG](https://github.com/HuskyInSalt/CRAG) entraîne un T5-large à une sortie
scalaire, évaluateur de pertinence requête/passage. Le [papier](https://arxiv.org/html/2401.15884v3)
décrit0,77B paramètres et les branches correct/incorrect/ambigu ; seuils empiriques par corpus,
donc aucune probabilité universelle ni qualité sur nos fiches acquise. Le code d’entraînement
officiel confirme T5ForSequenceClassification(num_labels=1).
[Adaptive-RAG](https://github.com/starsuzi/Adaptive-RAG) entraîne un classifieur T5-large pour
choisir sans recherche/simple/itératif. Ces tâches QA ne sont pas encore notre décision de rappel
en présence du contexte déjà visible. [Self-RAG](https://github.com/AkariAsai/self-rag) est une
autre piste, avec décisions/critiques intégrées à la génération.

[Decision1.0 de vLLM Semantic Router](https://vllm-sr.ai/blog/decision-models/) propose décisions
typées Choice/Noul/Score ; [Eos0,8B](https://huggingface.co/vllm-sr/Decision-1.0-Eos-0.8B) est un
candidat général à comparer ultérieurement à Qwen3-4B et Laya. Ce n’est pas la preuve d’un
rappel mémoire déjà prêt ; français, calibration, données et runtime de nos machines restent à
vérifier. [ARES](https://github.com/stanford-futuredata/ARES) apporte une méthode d’évaluation
par classifieurs et annotations, distincte d’une politique de rappel en production.

Deux précisions évitent de recycler des descriptions périmées : la licence actuelle de
[Pathway](https://github.com/pathwaycom/pathway/blob/main/LICENSE.txt) est BSL1.1, donc ne pas
le ranger sans nuance dans la sélection libre ; le README actuel de
[Mem0](https://github.com/mem0ai/mem0) décrit une extraction additive v3, alors que des prompts
historiques ADD/UPDATE/DELETE/NONE restent consultables. Ne pas confondre ces versions.

Priorités proposées, sans nouveau lot engagé : CocoIndex pour incrémentalité/vues ; Dagster
pour état dérivé et preuves ; CRAG/Adaptive-RAG pour décomposer les décisions ; Letta/Graphiti
pour mémoire/reprise. Pour nos essais futurs : cas sans fiche utile, contexte déjà suffisant,
négatifs proches, fiches périmées, séparation des sessions entre apprentissage et évaluation,
coût d’un faux rappel et latence. Les scores de publications ne se comparent pas directement
à notre banc de fiches.

## Décision de présentation de Lucie

Lucie retient l’accueil CocoIndex comme référence de design pour vendre le framework.
Direction consignée dans [la vision de vitrine](../visions/2026-11-12-vitrine-produit-reference-cocoindex.md)
et indexée. Pas de chantier web, de déploiement ou d’expérimentation modèle lancé.
API et extraits de sources primaires conservés hors dépôt sous
`~/.cache/orchestration-notes/tour-horizon-12nov/sources.json`.
Clôturer ce ticket avec les références consultées ; toute expérimentation ultérieure a son périmètre.
