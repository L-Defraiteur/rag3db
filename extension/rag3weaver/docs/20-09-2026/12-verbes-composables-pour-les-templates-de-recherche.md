# Templates de recherche : faciliter le chaînage par des verbes

Date : 20-09-2026. Proposition initiale ; première tranche désormais implémentée **dans le moteur**, voir [la note 14](14-verbes-de-recherche-dans-le-moteur.md). Le vocabulaire envisagé ci-dessous dépasse encore les verbes disponibles.

## Besoin et décision à retenir

Demande de Lucie pendant la révision des decks Magic : les templates de recherche pourraient bénéficier d'abstractions supplémentaires, **surtout des verbes pour enchaîner facilement les opérations**. Le branchement direct du MCP à Codex supprimera le client batch intermédiaire ; il ne simplifiera pas, à lui seul, l'écriture des recherches composées.

Direction proposée : une couche déclarative de verbes compilée vers le DAG existant. Les templates exposent des opérations et paramètres métier ; le compilateur assure les raccordements de ports, les conversions de résultats et les vérifications. Mermaid reste une représentation inspectable et un accès avancé. Éviter de créer un deuxième moteur d'exécution ou un nœud spécialisé par produit.

L'objectif est de réduire les manipulations techniques répétitives. Plusieurs recherches exploratoires restent utiles pour comprendre un corpus et comparer des hypothèses ; cette couche ne promet pas une réponse parfaite en un appel.

## Ce qui existe déjà dans le code

- [`search_structured.mmd`](../../templates/tools/search_structured.mmd) relie recherche BM25/dense, fusion, pagination, résolution des parents et rendu. Les filtres sont transmis dans les options de recherche.
- [`search_dense_related.mmd`](../../templates/tools/search_dense_related.mmd) compose recherche dense, résolution des parents et parcours de relation.
- [`search_related_scoped.mmd`](../../templates/tools/search_related_scoped.mmd) compose sélections structurées, traversées, intersections de périmètre, fusion pondérée et pagination finale.
- [`composable_results.rs`](../../src/dataflow/composable_results.rs) fournit `SelectRecordsNode`, `RelatedResultsNode` et `IntersectResultsNode`. Le parcours passe actuellement par `FetchRelatedNode`, puis par la promotion des enfants en résultats.
- `FuseResultsNode` et `RenderResultsNode` assurent respectivement la fusion des signaux et la présentation. Le rendu texte MCP et le retour JSON sont déjà séparés : voir [la note 11](11-rendu-mcp-et-lucivy43.md).

Une partie importante des opérations existe donc déjà. Le travail proposé concerne leur composition, la découverte de leurs contrats et la qualité des erreurs, plutôt que leur réimplémentation.

## Vocabulaire envisagé

Les noms ci-dessous sont des propositions, pas des alias déjà reconnus par le parseur.

| Verbe | Intention | Contrat à rendre explicite |
|---|---|---|
| `select` | Sélectionner des objets par filtres exacts | Entité et filtre typé ; exhaustivité distincte d'un top-k. |
| `search` | Chercher par sens, texte ou combinaison des deux | Entité, requête, signaux, filtres et budget de candidats. |
| `follow` | Suivre une relation | Relation, sens, type de sortie ; conserver les preuves du chemin. |
| `where` | Restreindre un ensemble par ses propriétés | Champs et opérateurs validés par le schéma ; pousser le filtre à la source quand c'est équivalent. |
| `within` / `intersect` | Restreindre au périmètre d'un autre ensemble | Identité commune ou correspondance de clés déclarée ; conserver le score de recherche. |
| `exclude` | Retirer un ensemble d'identités | Anti-intersection explicite, distincte d'un score négatif. |
| `fuse` | Réunir plusieurs branches et leurs contributions | Poids, stratégie, normalisation et politique de doublons. |
| `rerank` | Réévaluer les candidats avec une fonction supplémentaire | Modèle ou fonction et budget ; opération distincte de la fusion. |
| `take` / `page` | Borne finale des résultats | Ordre stable ; ne pas masquer une limite intermédiaire. |
| `project` | Choisir les données à retourner | Projection explicite, après les opérations qui ont besoin des champs retirés. |
| `render` | Présenter les résultats lisiblement | Template nommé ; conserver une sortie structurée utilisable. |
| `explain` | Inspecter une recherche | Plan compilé, chemins, contributions, limites et résultats partiels. |

Prévoir des branches nommées et leur réutilisation : une simple chaîne linéaire ne suffit pas pour fusionner une recherche directe et une recherche par relations.

## Exemple qui motive la couche

Pseudo-syntaxe de discussion uniquement ; ce bloc n'est pas exécutable aujourd'hui :

```text
capacites = search(CatalogAbility, $intention, signals=dense)
indirect = capacites.follow(CatalogCardAbility, incoming)
direct = search(CatalogCard, $intention, signals=hybrid)

fuse(indirect: 2, direct: 1, duplicates=merge)
  .where($filtres_cartes)
  .within($perimetre, on=$correspondance_identite)
  .page(limit=20)
  .render("magic")
```

Pour Magic : trouver les capacités relatives au cimetière, retrouver leurs cartes, compléter par le texte des cartes, favoriser la branche capacités, puis limiter à la collection ou à un deck. Les mêmes verbes doivent servir ailleurs : concepts → documents, symboles → fichiers, caractéristiques → produits.

`where` et `within` expriment ici le résultat voulu, pas une obligation d'exécution tardive. Le compilateur doit appliquer les restrictions avant la sélection des candidats lorsque possible. Avec un top-k dense initial, filtrer seulement après la traversée peut donner une page vide alors que des cartes pertinentes existent hors des premiers candidats. Il faut exposer le budget, prévoir son élargissement ou déclarer la recherche partielle ; ne jamais présenter une recherche ANN comme exhaustive.

Le catalogue et la collection possédée sont deux entités distinctes. `within(collection)` ne peut pas comparer naïvement leurs UUID internes : la relation ou la correspondance métier doit être déclarée. De même, agréger des réimpressions d'une carte n'est pas la même opération que dédoublonner le même objet retrouvé par deux chemins.

## Contrats pour éviter des itérations de plomberie

1. **Des ensembles typés entre les verbes.** Chaque étape connaît l'entité de sortie, sa clé d'identité et les informations de score/provenance disponibles. Une relation peut changer le type de l'ensemble.
2. **Des paramètres MCP décrits précisément.** Générer les champs, types, enums, relations et opérateurs autorisés depuis le schéma. Une recherche courante ne devrait pas demander à l'appelant de deviner un gros objet `options` JSON.
3. **Une validation avant exécution.** Signaler le verbe fautif, le champ ou la relation inconnue et les choix disponibles. Ne pas démarrer un parcours coûteux pour découvrir une incompatibilité évitable.
4. **Une identité et une fusion explicites.** Fusionner par défaut le même objet, en conservant les contributions et preuves des branches ; garder les occurrences séparées seulement sur option. Ne pas fusionner par égalité de nom ni additionner directement des scores de nature différente sans stratégie déclarée.
5. **Des plans inspectables.** Exporter le DAG Mermaid et les paramètres résolus. Les déplacements de filtres doivent préserver la sémantique, en particulier aux frontières top-k, fusion et parcours de relation.
6. **Un résultat réutilisable.** Pour les ensembles intermédiaires volumineux, étudier des références de résultats côté serveur, avec durée de vie, périmètre de session et fraîcheur du snapshot explicites. Cela éviterait de renvoyer puis recopier des centaines d'UUID dans le contexte MCP.
7. **Une présentation finale compacte.** Le rendu doit exposer les raisons du classement et les limites utiles sans dupliquer le JSON. La projection de présentation ne doit pas enlever les données nécessaires aux étapes suivantes.

## Première tranche raisonnable

Commencer par `search/select → follow → within → fuse → page → render`, en s'appuyant sur les nœuds présents. Décrire leurs signatures et compiler quelques chaînes/branches déclaratives vers les templates existants. Valider l'équivalence avec les DAG écrits à la main : identités, scores, provenance, doublons et périmètre.

Utiliser comme cas d'acceptation la recherche cimetière → capacités → cartes de la collection, avec une branche texte directe et une pondération différente. Ajouter des cas sans résultat, avec doublons de chemins, avec réimpressions et avec candidats pertinents au-delà d'un top-k initial.

Les références de résultats, la pagination persistante et les nouveaux verbes sans équivalent actuel pourront venir ensuite. La gestion du processus MCP, de sa fermeture durable et du partage de la base relève du cycle de vie du backend, pas de l'algèbre de recherche.

## État de cette note

Cette note conserve la proposition initiale. La [note 14](14-verbes-de-recherche-dans-le-moteur.md) décrit maintenant le compilateur, l'API de bibliothèque, les tests et les limites de la première tranche ; aucun nouvel outil MCP spécifique aux verbes n'a été ajouté. Les recherches effectuées pour les decks utilisent déjà le vrai MCP rag3weaver, via un client batch. Leurs requêtes et résultats sont conservés dans `experiments/mtga/data/deck-drafts/experiments-2026-09-19/revisions-2026-09-20/research/` ; les révisions des 13 decks sont documentées dans la [note 13](13-revision-des-13-decks-et-audit-des-donnees.md), avec un incident de persistance distinct découvert pendant la validation.
