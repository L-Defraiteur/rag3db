# Verbes de recherche : première implémentation dans le moteur

Date : 20-09-2026. Suite de la [proposition 12](12-verbes-composables-pour-les-templates-de-recherche.md).

## Décision

La composition vit dans la bibliothèque Rust `rag3weaver`, indépendamment des transports. Le module [`dataflow::search_chain`](../../src/dataflow/search_chain.rs) compile des chaînes typées vers les nœuds du DAG existant. Il n'y a ni nouvel exécuteur ni logique Magic dans le compilateur. Le MCP n'a pas reçu de nouvel outil spécifique aux verbes.

Trois accès sont disponibles :

- `SearchProgram::compile(&SearchSchema, &NodeRegistry)` : compilation sans connexion, avec schémas d'entités et relations fournis par l'application.
- `SearchPlan::execute(&NodeRegistry, ServiceRegistry)` : exécution dans le runtime existant ; sorties structurées et métadonnées accessibles par leurs ports.
- `PreparedBackend::compile_search_program` / `Backend::run_search_program` : mêmes opérations à partir d'un backend déclaratif. L'exécution renvoie `result`, `metadata`, `graph_hash`, `candidate_bounded`, `diagnostics`, et `presentation` si un rendu est demandé.

Les schémas et le registre proviennent de l'application ; l'appelant qui utilise directement `SearchPlan::execute` monte les services habituels du catalogue. `Backend` les monte lui-même. La compilation ne prouve pas que les index existent ou sont prêts : ces informations restent vérifiées et exposées à l'exécution.

## Verbes disponibles

| Verbe JSON | API Rust | Comportement |
|---|---|---|
| `search` | `Chain::search(Search { … })` | BM25, vectoriel, sparse ou combinaison ; budget `candidates` obligatoire, entre 1 et 10 000. |
| `select` | `Chain::select(entity, filter)` | Sélection exacte sans top-k implicite. |
| `from` | `Chain::from(branch)` | Réutilisation d'une branche nommée, compilée une seule fois. |
| `follow` | `.follow(relation, Direction::Incoming/Outgoing)` | Parcours sans limite implicite, promotion des voisins en résultats et conservation des témoins. |
| `where` | `.filter(condition)` | Restriction typée ; les filtres immédiatement après `search` ou `select` sont poussés à cette source. |
| `within` | `.within(branch)` | Intersection par `(entité, UUID)` ; conserve le score de gauche. |
| `fuse` | `Chain::fuse(Fusion { … })` | Branches nommées, poids positifs, stratégies `rrf` ou `weighted`, doublons `merge` par défaut ou `keep`. |
| `page` | `.page(limit, offset)` | Pagination finale seulement ; un filtre après pagination est refusé. |
| `render` | `.render(template)` | Dernière étape facultative ; le résultat structuré reste accessible. |

Une chaîne commence par `search`, `select`, `from` ou `fuse`. Une fusion intermédiaire se nomme comme branche, puis se réutilise avec `from` ou une nouvelle fusion. `page` et `render` sont réservés au pipeline final.

Exemple de composition Rust, avec des branches préalablement déclarées :

```rust,ignore
let indirect = Chain::from("abilities")
    .follow("HasAbility", Direction::Incoming);

let pipeline = Chain::fuse(Fusion {
    inputs: [("indirect".into(), 2.0), ("direct".into(), 1.0)].into(),
    strategy: FusionStrategy::Rrf,
    duplicates: DuplicatePolicy::Merge,
})
.within("owned_cards")
.page(20, 0)
.render("tree");
```

Cet exemple illustre le schéma de test `Card → HasAbility → Ability`, pas une requête prête à envoyer au backend Magic. Les branches `indirect`, `direct` et `owned_cards` doivent toutes produire la même entité `Card`. Si la collection et le catalogue ont des entités différentes, il faut suivre leur relation explicite pour obtenir une identité commune avant `within`. Aucun rapprochement automatique par nom n'est effectué.

## Exemple utilisable sans MCP

[`notebook-review.json`](../../templates/queries/notebook-review.json) combine deux recherches pondérées sur les notes relues du backend de démonstration. Les restrictions de statut sont appliquées dans chaque source avant son top-k.

Depuis la racine du dépôt :

```bash
cargo run --offline --manifest-path extension/rag3weaver/Cargo.toml \
  --example search_chain_plan -- \
  extension/rag3weaver/templates/backends/notebook/backend.json \
  extension/rag3weaver/templates/queries/notebook-review.json
```

L'[exemple Rust](../../examples/search_chain_plan.rs) compile seulement : il n'ouvre pas la base, ne lance pas le GPU et ne contacte pas le MCP. Sa sortie contient le DAG concret, les diagnostics et un export `{mermaid, arguments}`. Pour exécuter, une application qui possède déjà un `Backend` appelle `backend.run_search_program(&program)`.

`plan.template()` exporte un GraphTool Mermaid avec des paramètres typés séparés. La relecture avec `GraphTool::from_mermaid(...).instantiate(&arguments)` reconstruit le même DAG, même si une requête contient des apostrophes, guillemets, `$`, `%%`, flèches ou sauts de ligne. Les valeurs ne sont jamais interpolées dans la syntaxe Mermaid.

## Difficultés et résolutions

1. **Filtres historiques permissifs.** Le filtre `Field` existant autorise certains chemins de jointure qui échappent à la validation locale des champs. Dans les chaînes, il devient un `Path` local validé contre le schéma, y compris sous `nested`. Les jointures passent explicitement par `follow`. Les anciens appels ne sont pas modifiés.
2. **Export Mermaid et objets imbriqués.** L'export brut existant peut transformer un objet JSON en chaîne de caractères. Le nouvel export utilise les paramètres typés du GraphTool et conserve les arguments JSON séparément. Le parseur Mermaid historique n'est pas réécrit.
3. **Branche réutilisée et scores.** Un nouveau nœud générique `LabelResultsNode` nomme les contributions sans recalculer le classement. Sans paramètre `signal`, il sert de sortie terminale transparente. Les tests vérifient le partage d'une branche et la conservation des scores et témoins.
4. **Filtres après un top-k.** Aucun déplacement à travers une relation, fusion ou intersection n'est supposé équivalent. Le plan signale les restrictions tardives et porte `candidate_bounded=true` dès qu'une recherche bornée participe au résultat, y compris comme périmètre. Ce drapeau concerne les candidats, pas la pagination finale ni l'état d'indexation.
5. **Validation de non-régression.** L'ajout d'un nœud exigeait d'actualiser le nombre de nœuds intégrés. Certains tests ouvrent des sockets locaux et un test de détection de dépôt était perturbé par un dépôt Git présent dans `/tmp` : la suite complète a été relancée hors sandbox avec `TMPDIR=/var/tmp`.

## Validation et limites

Validation : **997 tests unitaires réussis**, dont huit nouveaux tests couvrant l'équivalence avec un DAG écrit à la main (identités, scores, témoins), les ensembles vides, les doublons conservés, la réutilisation d'une branche, les types incompatibles, les cycles, les champs inconnus, le placement des filtres, les budgets invalides, l'export typé et l'exécution par `Backend` sans transport.

La commande d'exemple ci-dessus a également été exécutée avec succès : compilation du plan `Note`, export Mermaid/arguments et vérification des filtres de statut dans les sources, sans ouverture de base.

Les nouveaux tests d'exécution utilisent les vrais nœuds et le runtime avec une connexion de test contrôlée. Ils ne certifient ni la pertinence du dense sur Magic ni la persistance native. L'incident de données imbriquées décrit dans la [note 13](13-revision-des-13-decks-et-audit-des-donnees.md) reste distinct et non résolu par ce travail.

Limites de cette tranche :

- Pas encore de verbes `exclude`, `rerank`, `project`, ni de références de résultats persistants ; les DAG avancés restent disponibles.
- Pas de recherche vectorielle exhaustive promise ; le compilateur ne relance pas automatiquement la recherche en augmentant le budget après une intersection vide.
- `select` et `follow` peuvent produire de grands ensembles, matérialisés par les nœuds actuels. Pas de traitement en flux ni de poussée automatique des intersections dans la base.
- Pas de correspondance de clés entre entités implicite, ni de fusion des réimpressions portant des UUID différents.
- Limites structurelles : 32 branches, 64 verbes par chaîne, 256 nœuds et 1 024 arêtes dans le plan. Ce ne sont pas des budgets mémoire ou temps d'exécution.
- La syntaxe JSON est la sérialisation des types Rust, pas une nouvelle grammaire textuelle libre. Mermaid reste l'export et l'accès avancé.

La prochaine intégration dans un transport pourra appeler cette API commune, sans y déplacer l'algèbre de recherche.
