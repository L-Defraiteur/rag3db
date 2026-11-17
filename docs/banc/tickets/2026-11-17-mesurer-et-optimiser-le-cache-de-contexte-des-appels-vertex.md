# Mesurer et optimiser le cache de contexte des appels Vertex

- **État** : ouvert, audit de préparation demandé le17novembre2026 ; aucune optimisation ni mesure fournisseur revendiquée.
- **Famille** : produit
- **Gravité** : coût et latence potentiellement évitables ; gain réel à mesurer, aucun taux cible de99% établi pour rag3weaver.
- **Atteignable en service** : oui, appels LLM Vertex de l'agent, des décisions et de l'archiviste.
- **Touche rag3weaver** : oui, composition des invites et outils, client compatible OpenAI, compteurs d'usage et consommation.
- **Pour** : Banc pour l'audit et le protocole de mesure ; Everything Declarative pour une éventuelle évolution commune du client et des options déclaratives.
- **Signalé par** : demande de Lucie du17novembre2026 sur les optimisations de cache Vertex ; orchestration assure le suivi.

## Demande et état vérifié

Étudier comment réutiliser les préfixes stables sans changer le sens des appels.
Une observation de taux de cache dans une autre application n'est ni une mesure
de rag3weaver, ni une promesse de routage vers une même machine.

Source locale927941a67a5805450d49f1e04b39fdd542a6059b relue ; présence des paramètres
recontrôlée par gitgrep sur master99fef92ce6ee9a0007eaf99c7c3a6a3789061c57 :

- `src/openai_llm.rs::request_body` transmet les messages et outils, sans raccord
  de cache explicite ni `prompt_cache_key` identifié.
- Le lecteur SSE sait reprendre `prompt_tokens_details.cached_tokens` et
  `cached_content_token_count` dans `Usage.cached_prompt_tokens`. Une valeur
  absente tombe aussi à0 : ne pas traiter ce0 comme preuve d'un cache mesuré vide.
- `src/llm.rs::Usage` et les traces de décision exposent déjà des compteurs.
  [Le lot usage](../../memoire/15-11-2026/04-l-usage-du-modele-dans-la-trace.md)
  et [l'étude de l'archiviste](../15-11-2026/02-jusqu-ou-l-archiviste-voit.md)
  évitent de recréer cette instrumentation ou d'annoncer un cache entièrement absent.
  Cette dernière avait déjà placé instruction puis contexte stable avant le bout courant.
- L'agent peut renouveler son catalogue d'outils. Auditer l'ordre et les versions
  réellement envoyés ; leur instabilité générale n'est PAS démontrée par cette lecture.

Chemins de code relatifs à `extension/rag3weaver/` ; aucun contenu d'invite privée,
secret ou identifiant d'authentification n'est nécessaire au premier audit.

## Contrats fournisseur vérifiés le17novembre

Google documente un cache implicite activé par défaut, favorisé par de grands
préfixes communs en début d'invite et des requêtes proches. Le cache explicite
référence une ressource créée, avec durée de vie et frais de stockage à considérer.
Ces mécanismes et leurs limites dépendent du modèle et de l'API.
[Vue d'ensemble Google](https://cloud.google.com/vertex-ai/generative-ai/docs/context-cache/context-cache-overview).

La compatibilité OpenAI documente `extra_body.google.cached_content` ; aucun
`prompt_cache_key` n'est listé et les paramètres non pris en charge sont ignorés.
Ne pas annoncer une optimisation après simple ajout d'un champ.
[Paramètres compatibles](https://cloud.google.com/vertex-ai/generative-ai/docs/migrate/openai/overview).

Les instructions et outils d'un contexte explicite ont des contraintes de
redéclaration : composer ce contrat avec les outils dynamiques de l'agent.
[Utilisation d'un contexte](https://cloud.google.com/vertex-ai/generative-ai/docs/context-cache/context-cache-use).

## Travail de préparation, puis décision

1. Auditer les vrais constructeurs communs d'invites : instructions, schémas et
   ordre des outils, contexte stable, historique et données variables. Repérer
   les premières différences au moyen de tailles/empreintes, sans publier les corps.
   Ne déplacer ni rôle ni instruction de confiance, ni réordonner l'histoire.
2. Définir des compteurs fiables : jetons d'entrée, jetons cachés observés et
   absence de compteur distincte, fraction pondérée `somme(cachés)/somme(entrée)`
   sur les appels renseignés, premier appel froid, latence et coût complet.
   Ne pas confondre fraction des jetons et proportion des requêtes avec un hit.
3. Proposer d'abord les gains possibles sur le préfixe stable ; comparer ensuite
   cache implicite et explicite si le volume et les répétitions le justifient.
   Vérifier modèle/endpoint/projet/région, seuils, TTL et prix applicables avant mesure.
4. Pour une future identité de contexte, lier contenu exact, modèle, version des
   gabarits/outils et périmètre d'accès. Aucun nouvel UUID par requête, aucune
   mutualisation de contexte entre permissions incompatibles. Une modification
   réelle de contenu ou d'outils doit invalider la référence correspondante.
5. Préparer les témoins du vrai `request_body` et du lecteur d'usage avant fix :
   préfixe stable, changement nécessaire respecté, paramètre effectif selon
   fournisseur, fusion des extensions sans écraser les options existantes,
   cache expiré/refusé et compteurs manquants. Les écrire/publier avant exécution.

Premier périmètre autorisé : lecture source/documentation et plan sourcé seulement.
Aucun build, test, appel LLM, création de cache cloud, changement de manifeste ou
réglage de production par ce ticket. Une mesure réelle et un éventuel correctif
auront leur recette, budget et validation distincts ; pas d'interruption du nettoyage
ED ni des autres lots déjà engagés.

## Critères de fin

Audit sourcé des chemins réels et décision mesurée, avec limites par fournisseur.
Si optimisation retenue : témoin rouge avant fix, contrôle du corps réellement
envoyé, compteurs observés et comparaison coût/latence à sens et qualité conservés.
Inclure création/stockage/expiration si cache explicite ; aucune simple acceptation
HTTP ou champ ignoré ne prouve un gain. Si le raccord est abandonné, documenter
pourquoi et ce qui reste comme suivi. Ticket ouvert jusqu'à réalisation ou abandon.
