# Validation, limites et reprise

Complément du [rapport des problèmes et corrections](01-magic-rag3weaver-problemes-et-resolutions.md).
État constaté le 19 septembre 2026 ; modifications non commitées.

Suite de la session : le [rapport 03](03-backends-declaratifs-et-persistance.md)
documente le nouveau backend déclaratif, la validation JSON Schema complète
des écritures de ce backend, les dates, la persistance et le MCP généré.
Les limites ci-dessous décrivent le jalon précédent et restent applicables
aux parcours que le rapport 03 ne couvre pas.

## Tests exécutés

| Vérification | Résultat | Portée |
|---|---|---|
| Mapping JSON Schema et compilation de filtres | Tests ciblés passants | Types, références locales, nullabilité, chemins, paramètres |
| Suite unitaire rag3weaver hors sandbox | 972 passants, 0 échec | Exécutée avant le dernier ajustement du prédicat nul et du port terminal |
| `structured_roundtrip_and_same_element_filter` | Passant après les corrections finales | Base réelle, tableaux natifs, nulls, relecture, réingestion inchangée, filtres paramétrés |
| `mermaid_dag_filters_with_lucivy_and_local_dense` | Passant après les corrections finales | Template Mermaid réel, lucivy, BGE-M3 réel, hybride, restriction avant page finale, rejet de chemin invalide |
| `git diff --check` | Passant au contrôle effectué | Espaces parasites dans les modifications suivies |

Les deux tests d’intégration se sont terminés ensemble par :

```text
running 2 tests
mermaid_dag_filters_with_lucivy_and_local_dense ... ok
structured_roundtrip_and_same_element_filter ... ok
2 passed; 0 failed
```

Le premier test de stockage utilise explicitement `HashEmbedder` avec les signaux
recherche désactivés ; il n’évalue pas la pertinence sémantique. Le second utilise
le vrai démon BGE-M3. Un corpus aussi petit prouve le raccordement fonctionnel,
pas la qualité de classement sur toute la collection ni les performances à grande
échelle.

## Reproduire

Les commandes partent de la racine du dépôt. Le script est spécifique au montage
local : il attend le binaire du démon construit, le cœur partagé corrigé,
l’extension vectorielle existante et un démon d’embeddings réel sur le port 7878.
Il ne télécharge pas les poids et ne lance pas ce démon GPU.

```bash
# Construire le cœur partagé avec la correction de copie des paramètres.
cmake --build build/lecteurs-csv --target rag3db_shared -j 2

# Construire le démon avec le transport des paramètres typés.
RAG3DB_SHARED=1 \
RAG3DB_LIBRARY_DIR="$PWD/build/lecteurs-csv/src" \
RAG3DB_INCLUDE_DIR="$PWD/build/lecteurs-csv/src" \
cargo build --offline --manifest-path extension/rag3weaver/Cargo.toml \
  --features daemon,rag3db-native --bin rag3daemon -j 2

# Suite unitaire : l'environnement doit autoriser les sockets locaux.
cargo test --offline --manifest-path extension/rag3weaver/Cargo.toml --lib -j 2

# Base jetable en mémoire sur 8736 ; arrêt du processus de test à la sortie.
bash extension/rag3weaver/scripts/test_structured_payloads.sh
```

Le script est [test_structured_payloads.sh](../../scripts/test_structured_payloads.sh).
Le port 8736 doit être libre. La sonde actuelle vérifie la disponibilité du service,
pas l’identité du processus : ne pas exécuter ce script sur un port déjà occupé.
Les premières investigations ont aussi lancé des démons temporaires sur 8734 et
8735 ; ne pas les confondre avec la destination Magic sur 8732.

Le binaire du démon utilisé pendant les tests finaux avait été compilé avec le
header amalgamé du build `lecteurs`, puis chargé avec la bibliothèque partagée
corrigée `lecteurs-csv` via `LD_LIBRARY_PATH`. La commande ci-dessus aligne les
deux chemins pour une reproduction plus explicite.

## Exemple du contrat de filtre

Condition sur un même objet du tableau :

```json
{
  "nested": {
    "path": ["abilities"],
    "condition": {
      "must": [
        {"field": {"key": "label", "value": "Flying"}},
        {"field": {"key": "level", "value": [{"op": "gte", "value": 3}]}}
      ]
    }
  }
}
```

Les champs `label` et `level` sont ceux du corpus de test, pas une nouvelle
interprétation des champs natifs Arena.

Pour une liste de couleurs :

```json
{
  "path": {
    "path": ["colors"],
    "value": [{"op": "has_any", "value": ["Blue"]}]
  }
}
```

Le JSON Schema source pilote les types ; aucun échantillon de données n’est
utilisé pour déduire le type d’une liste vide.

## Fichiers ajoutés et modifiés

Points d’entrée principaux :

- [json_schema.rs](../../src/json_schema.rs) : mapping, descripteurs, normalisation, validation des branches structurées ;
- [filter.rs](../../src/filter.rs) : `Path`, `Nested` et quantificateurs ;
- [search_structured.mmd](../../templates/tools/search_structured.mmd) : template générique instancié dans le test ;
- [structured_payloads.rs](../../tests/structured_payloads.rs) : tests d’intégration reproductibles ;
- [parsed_parameter_expression.h](../../../../src/include/parser/expression/parsed_parameter_expression.h) : correction dans le cœur C++.

Les changements connexes portent sur le schéma, le dialecte, le transport du
démon, la conversion native, les nœuds d’écriture et la lecture du catalogue.
Les branches exhaustives de conversion de valeurs ont été adaptées au nouveau
variant de paramètre ; cela ne signifie pas que tous leurs parcours sont
couverts par les deux tests.

L’adaptateur `experiments/mtga/rag3bridge/` a un prototype compilé parlant JSON
sur stdio. Il prépare les commandes de schéma, ingestion, recherche et composition.
Il n’est pas encore le service d’ingestion Magic final. Comme tout le dossier
`experiments/mtga/`, il est ignoré par Git : un commit des corrections génériques
n’emporterait pas ce prototype.

## Limites et points à terminer

### Support générique

- Valider les mises à jour structurées et les suppressions/restaurations, dont
  l’undo et la reprise après crash. L’annotation de paramètres a été ajoutée aux
  insertions et mises à jour ordinaires ; les chemins d’undo et propriétés de
  relations ne sont pas encore tous raccordés et éprouvés.
- Compléter les essais sur listes de listes, imbrication récursive de conditions,
  caractères spéciaux et combinaisons booléennes. Les exemples simples ne
  suffisent pas à promettre tout JSON imbriqué.
- Le mapper est un mapping de stockage, pas un validateur complet JSON Schema.
  La source reste responsable de `required`, des enums, formats, bornes et autres
  contraintes non prises en charge par ce mapper.
- Les objets ouverts restent opaques et textuels ; les anciennes colonnes `Json`
  et `Tags` ne deviennent pas automatiquement des listes. Une évolution de type
  nécessite une stratégie de migration explicite.
- Le support natif de structures ajouté ici est destiné à rag3db. L’API
  d’enregistrement refuse ces types sur un autre dialecte ; les autres chemins
  de déclaration et backends n’ont pas été validés.
- Le fil du démon a été étendu avec `Typed`. Client et démon doivent être mis à
  jour ensemble pour ce parcours. L’ancien démon ne comprend pas cette variante.
- Relancer la suite unitaire sur l’état final avant commit, et faire une revue
  des modifications transversales. Aucun commit, PR ou déploiement réalisé.

### Raccordement Magic

- Exécuter le mapper sur tous les schémas et records réels de l’API source.
- Matérialiser les entités `Ability` et leurs relations, puis les liens exacts
  vers `Mechanic`. Préserver les identifiants externes indépendamment des UUID
  internes de rag3weaver.
- Finaliser les variantes collection/deck du template et les enrichissements
  quantité, profil, emplacement. Les restrictions doivent participer à la
  recherche, pas filtrer une page globale déjà tronquée.
- Pour découvrir une mécanique puis naviguer vers les capacités et cartes,
  prévoir la conversion générique du port `children` de `FetchRelatedNode` vers
  une liste de résultats dédupliqués. Ce parcours à plusieurs étapes n’est pas
  encore un template exécutable validé.
- Terminer la synchronisation idempotente : snapshot cohérent, suppressions,
  relations périmées, état des échecs et visibilité de l’avancement.
- Ingérer la collection complète puis mesurer la recherche réelle. La destination
  rag3db Magic et l’API source ne doivent pas être confondues avec les bases de test.
- Exposer et tester le MCP de resynchronisation et de recherche composée ; il
  n’est pas encore installé dans Codex.

Le cadre de responsabilité reste : extraction et sémantique Arena côté Magic ;
types de payload, filtres, DAG, indexation et mécanismes de synchronisation
réutilisables côté rag3weaver.
