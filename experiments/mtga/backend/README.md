# Collection réellement ingérée dans rag3weaver

`backend.json` décrit la base persistante `data/engine-search-8g.rag3db`, le schéma de `OwnedCard`, lucivy et le dense BGE-M3 1024 via le service local 7878. Le backend ouvre seul le catalogue ; ne pas lancer deux clients hôtes simultanément sur ce fichier.

```bash
python3 experiments/mtga/scripts/prepare_engine_collection.py
python3 experiments/mtga/scripts/engine_collection.py --ingest
python3 experiments/mtga/scripts/engine_collection.py --verify
python3 experiments/mtga/scripts/engine_collection.py --request experiments/mtga/backend/queries/hybrid_treasures.json --output /tmp/treasures.json
```

La préparation transforme TOUS les records possédés du snapshot source, sans présélection thématique. Elle ne fait pas la recherche. Les outils de sélection et recherche exécutent les graphes dans rag3weaver ; les comparaisons Python servent de contrôle indépendant. Le journal `data/engine-ingestion-progress.json` conserve les rapports des lots et `data/engine-results` les sorties des outils.

## Contrat de filtre

- `card_types`: tableau (`Land`, `Creature`, `Artifact`, `Enchantment`, etc.). Les types multiples sont conservés.
- `is_land`: face principale de cette entrée ; `has_land_face`: inclut les faces liées de terrain.
- `colors`: couleur de la carte ; un terrain peut être incolore.
- `color_identity`: identité couleur du client Arena ; utile pour délimiter les cartes d'un deck, distincte du mana produit.
- `mana_symbols_possible`: possibilités de mana détectées dans le texte de production et les types de terrains de base. Les terrains Thriving peuvent choisir une couleur et apparaissent donc dans plusieurs recherches, sans produire toutes ces couleurs à la fois.
- `mana_symbols_explicit`: symboles explicites de production, **pas une garantie de disponibilité inconditionnelle**.
- `mana_has_conditions`, `mana_conditions`: conditions détectées et texte complet à vérifier. Ce classificateur textuel n'est pas un moteur de règles exhaustif ; les capacités dynamiques complexes peuvent nécessiter une amélioration. La possibilité de chercher un terrain d'une couleur n'est pas assimilée à produire cette couleur : Deceptive Landscape produit C.
- `abilities`: tableau d'objets natifs avec texte et identifiant ; `nested` contraint un même élément.

Les filtres sont appliqués avant la sélection des candidats dans les branches de recherche. `select_cards` est exhaustif et sans top-k ; `search_cards` est classé et borné. Le dense est approximatif. La fusion déduplique l'identité de l'impression ; elle ne fusionne pas des impressions de cartes différentes par nom.

## Portée et limites

Cette base contient la collection possédée du snapshot, pas toutes les cartes non possédées, ni encore les decks et entités de glossaire reliées. Les capacités imprimées sont embarquées dans les records. La capture mémoire n'est pas relancée par l'ingestion. Le script rejoue des upserts ; il ne supprime pas encore les anciennes entrées devenues absentes après un changement de snapshot. Ne pas présenter ce script comme une synchronisation complète avec suppressions.

Le nœud `EntityBatchNode` valide tout le lot avant les écritures, refuse les identités dupliquées et refuse de contourner les politiques de révision, immutabilité, dates serveur et lifecycle. Un incident pendant les écritures/indexations peut néanmoins avoir des effets partiels ; le lot n'est pas une transaction atomique multi-index.

Le pilote et le lanceur MCP utilisent un pool rag3db de 8 Gio par défaut, modifiable via `RAG3DB_BUFFER_POOL_SIZE` (octets). Le modèle comprend aussi Ability, Mechanic, Deck et DeckEntry. Voir `describe_backend` pour les schémas et les relations.
