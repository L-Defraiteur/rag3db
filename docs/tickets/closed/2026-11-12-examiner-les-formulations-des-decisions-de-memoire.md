# Examiner les formulations des décisions de mémoire

- **État** : fait, lecture des prompts et appelants, critères et formulations consignés dans52783aa29 le12novembre2026 ; aucun prompt livré modifié, aucun modèle exécuté.
- **Famille** : produit
- **Gravité** : planification ; préciser les instructions et entrées utiles aux décisions de rappel et de rangement.
- **Atteignable en service** : non, lecture et propositions sans activation.
- **Touche rag3weaver** : oui, prompts déclarés de mémoire et conception M9.
- **Pour** : orchestration.
- **Signalé par** : Lucie ; examiner les formulations et la manière de poser les prompts des projets voisins.

## Travail

Lire les prompts réels et leurs appelants, comparer leur tâche à notre mémoire et proposer des
formulations adaptées. Distinguer pertinence thématique, apport au contexte déjà disponible,
classement durable, abstention et état de preuve. Les propositions ne modifient aucun prompt
de production et ne valent pas validation par un modèle.

## Critère de fin

Exemples concrets et sources épinglées ou datées rendus à Lucie, avec les adaptations nécessaires
et leurs limites ; pas de copie d'une ancienne constante inutilisée comme politique actuelle.

## Prompts et appelants examinés

- [Graphiti : dédoublonnage des faits](https://github.com/getzep/graphiti/blob/ebef986a0b2360211b65751c111cacb4a7d790eb/graphiti_core/prompts/dedupe_edges.py).
  Deux listes candidates sont présentées séparément ; les identifiants autorisés diffèrent selon
  l'opération. Les critères distinguent même fait, détail supplémentaire et incompatibilité.
  Un événement plus récent ne suffit pas à invalider un rôle ou une responsabilité. Des exemples
  contrastés montrent les distinctions ; les listes vides sont une issue normale.
- [Graphiti : extraction](https://github.com/getzep/graphiti/blob/ebef986a0b2360211b65751c111cacb4a7d790eb/graphiti_core/prompts/extract_edges.py).
  Messages actuels, anciens messages, entités et date de référence ont des rôles distincts.
  L'ancien contexte aide à interpréter la source actuelle ; il n'autorise pas à inventer une
  nouvelle information. Les dates sont rapportées au moment du message, utile pour les archives.
- [Mem0 : ADDITIVE_EXTRACTION_PROMPT](https://github.com/mem0ai/mem0/blob/main/mem0/configs/prompts.py),
  utilisé par [_add_to_vector_store](https://github.com/mem0ai/mem0/blob/main/mem0/memory/main.py),
  lecture du12novembre2026. Nouveaux messages, résumé, extractions récentes, souvenirs existants
  et historique récent sont séparés. Les souvenirs anciens servent au rapprochement et aux liens,
  les nouveaux messages à l'extraction ; un détail nouveau sur une entité connue n'est pas
  automatiquement un doublon. L'attribution distingue propos utilisateur et recommandations
  de l'assistant. Sa préférence pour l'extraction abondante diffère de notre archiviste sélectif.
- [LangGraph : exemple CRAG, Retrieval Grader](https://github.com/langchain-ai/langgraph/blob/main/examples/rag/langgraph_crag.ipynb),
  lecture du12novembre2026. Consigne système courte, document/question dans le message de données,
  sortie structurée binaire. L'exemple accepte une correspondance de mots ou de sens ; ce critère
  ne démontre ni apport supplémentaire au contexte ni nécessité d'injection. Cet exemple utilise
  un LLM appelé, distinct du T5 spécialisé du CRAG original. API et noms anciens non repris.

La lecture du fichier mémoire Letta actuel ne retrouve pas le rôle général annoncé par un
résultat de recherche antérieur : son contenu courant vise une réparation Git. Ne pas utiliser
ce résultat ancien comme preuve du comportement courant ; les autres sources suffisent ici.

## Formulations proposées pour notre contexte — non testées

**Rappel, avant recherche :** « Repère une information nécessaire à la demande qui manque au
contexte effectivement visible. Si tu ne peux nommer aucun manque utile, termine en expliquant
brièvement pourquoi. L'existence de fiches sur le sujet ne suffit pas à justifier une recherche. »

**Rappel, après recherche :** « Pour chaque élément proposé, indique quel manque il comble ou
quelle décision il pourrait changer. Écarte ce que le contexte contient déjà. Une proximité de
sujet ou une première place au classement ne constitue pas, à elle seule, un apport. Tu peux
ne rien injecter. Référence les sources et les versions retenues. »

**Rangement :** « Identifie les décisions, corrections ou connaissances durables apportées par
les nouveaux messages. Utilise les fiches et reçus antérieurs pour comprendre et comparer.
Distingue une proposition, une décision et un résultat vérifié ; conserve auteur et source.
Un nouveau suivi ne remplace pas automatiquement la décision antérieure. Si rien ne justifie
une écriture, appelle terminer(raison). »

Ces formulations sont des brouillons adaptés à la conception
[M9](../../memoire/11-11-2026/05-archiviste-automatique-du-chat.md), pas des traductions de prompts
tiers ni une activation. L'appel doit fournir réellement demande, contexte visible et génération,
fiches/version, reçus utiles et nouveaux messages sourcés. Nom/description d'une fiche seuls ne
permettent pas de certifier qu'un fait précis y est déjà conservé. L'archive livrée fait actuellement
un choix de destination de section ; lui attribuer toutes ces décisions serait inexact.

Prévoir quelques exemples contrastés pour les futurs essais : information déjà visible,
fiche seulement voisine, correction nouvelle, hypothèse non vérifiée, décision confirmée et
contexte devenu incomplet après compression. La bonne abstention fait partie du résultat attendu.
Les critères seront confrontés au banc existant avant tout remplacement d'un prompt livré.
