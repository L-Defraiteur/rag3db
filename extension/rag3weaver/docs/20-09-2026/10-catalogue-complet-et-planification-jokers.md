# Catalogue Arena complet et planification des jokers

## Besoin

La recherche dense portait sur les cartes possédées et leurs capacités. Les recherches de cartes à fabriquer utilisaient encore le catalogue SQLite local. Cette différence empêchait d'explorer les cartes manquantes avec les mêmes recherches composables.

L'extension ajoute au backend Magic trois entités déclaratives, dans une même base persistante rag3db : `CatalogCard`, `CatalogAbility`, `WildcardInventory`. `OwnedCard` conserve son périmètre. Le catalogue provient du client local : il ne constitue pas une garantie que chaque entrée est fabricable ou autorisée dans un format.

## Modèle et filtres

| Champ | Type | Usage |
|---|---|---|
| `arena_id` | entier | Impression/entrée native ; `key` reste son UUID stable |
| `rarity` | enum string | unknown, basic, common, uncommon, rare, mythic |
| `printed_mana_value` | entier | Valeur calculée des symboles imprimés ; X vaut zéro, pas le coût réellement payé |
| `owned` | entier | Quantité native pour cette entrée |
| `owned_printing_family` | entier | Maximum des faces et versions rééquilibrées liées |
| `owned_name_total` | entier | Somme des familles d'impressions de même nom canonique |
| `has_owned_copy` | booléen | Au moins un exemplaire, toutes impressions confondues |
| `missing_for_one`, `missing_for_playset` | entier | Manque pour une ou quatre copies ; terrain de base déjà débloqué : zéro |
| `craft_preferred` | booléen | Une impression candidate de rareté minimale par nom canonique |
| `is_craft_candidate` | booléen | Face principale non jeton/non rééquilibrée avec rareté fabricable, disponibilité non prouvée |
| `is_land`, `has_land_face` | booléen | Face terrain / au moins une face terrain |
| `card_types`, `subtypes` | tableau de strings | Creature, Land… ; Plant, Fungus, Demon… |
| `colors`, `color_identity` | tableau de strings | White, Blue, Black, Red, Green |
| `mana_symbols_possible` | tableau de strings | W, U, B, R, G, C ; possibilités de production des terrains |
| `mana_has_conditions`, `mana_conditions` | booléen, texte | Préserve le caractère conditionnel d'une source de mana |
| `abilities` | tableau d'objets | Capacités avec identifiant et textes EN/FR, filtrables par chemins |

Les noms sont normalisés par NFKC, casefold et espaces ; aucune fusion approximative par sous-chaîne. Les liens natifs réunissent les faces et versions rééquilibrées avant l'agrégation des quantités. Les terrains et coûts hybrides restent des informations imprimées : ce modèle ne simule pas toutes les règles du jeu.

Les relations `CatalogCardAbility` et `CatalogAbilityMechanic` donnent accès au texte précis et au registre de mécaniques. Lucivy et les embeddings BGE-M3 locaux indexent les cartes **et** les capacités. Les filtres structurés passent au moteur avant la fusion/pagination des signaux ; aucun filtrage Python des résultats de recherche.

## Outils et chemins

- MCP : `select_catalog`, `search_catalog`, `select_catalogability`, `search_catalogability`, `search_catalog_ability_cards`, `select_wildcardinventory`.
- `run_search` reste disponible pour un graphe Mermaid de recherche composé.
- Exemple de filtre combiné : `experiments/mtga/backend/queries/catalog/cheap-graveyard-drain.json`.
- Projection reproductible : `experiments/mtga/scripts/prepare_engine_catalog.py` ; appelée également par `prepare_engine_collection.py`.
- Synchronisation du snapshot local : `bash experiments/mtga/scripts/sync_engine.sh`. Un seul processus hôte doit posséder la base pendant l'opération. `refresh.sh` recapture d'abord Arena si nécessaire ; sync ne prétend pas mettre à jour la collection depuis un jeu arrêté.

## Planification

`experiments/mtga/scripts/plan_wildcards.py` lit les cartes et l'inventaire **via le MCP réel**, puis calcule les coûts. Les quantités demandées sont des totaux souhaités, non des achats supplémentaires. Les noms répétés sont additionnés. Une entrée inconnue bloque la conclusion « budget suffisant ». Le script ne fabrique aucune carte.

```bash
experiments/mtga/.venv/bin/python experiments/mtga/scripts/plan_wildcards.py \
  experiments/mtga/backend/queries/catalog/wildcard-targets.json \
  experiments/mtga/data/catalog-research/wildcard-plan.json
```

Le budget reste celui du snapshot daté, et ne tient pas compte d'un achat rapporté oralement après capture. Légalité de format et disponibilité du craft doivent être vérifiées dans Arena. Le calcul métier est local au produit Magic, les sélections et recherches restent des outils génériques du backend déclaratif.

## Difficultés rencontrées

Le backend impose au moins un champ de contenu même pour une entité sans signaux de recherche. La première ouverture a rejeté `WildcardInventory` pour cette raison, avant ingestion. Son `snapshot_id` sert désormais de champ de contenu ; aucun index dense n'est demandé pour l'inventaire. Le lanceur Python affiche maintenant la fin du journal natif si le backend ferme son entrée, au lieu d'une seule erreur BrokenPipe.

## Validation

Le test unitaire `test_catalog_model.py` couvre les réimpressions, faces/rééquilibrages, normalisation, coûts hybrides, copies manquantes, terrains de base et noms absents.

`test_engine_catalog.py` vérifie après réouverture les payloads persistés par lots, les combinaisons de filtres contre le snapshot source, les recherches denses réelles et la traversée capacité → carte. Les preuves sont écrites dans `data/engine-catalog-validation.json` et `data/catalog-research/`. Le statut final de cette validation est à compléter après l'ingestion.

La reconstruction Lucivy 4.3.0 utilise désormais `data/engine-search-lucivy43-recovered.rag3db` avec `fts_positions: false` et un pool de 15 Gio. Le détail du rendu MCP, de la mise à jour et de l’incident mémoire est dans [11-rendu-mcp-et-lucivy43.md](11-rendu-mcp-et-lucivy43.md).

## Résultats concrets

La recherche dense avec les filtres non possédée, commune/unco, valeur de mana ≤ 2, hors bleu/rouge et hors terrain classe `Undead Hand Ninja` en premier pour le départ de créatures du cimetière avec perte de vie ; `Corpse Knight` arrive en premier pour les arrivées de créatures avec perte de vie.

Le plan d'exemple demande deux Corpse Knight, quatre Creeping Chill et quatre Unearth au total. Le snapshot possède déjà trois Unearth : coût proposé **six jokers unco et un commun**, aucun rare/mythique. Inventaire capturé le 19 septembre à 16:36 UTC ; aucun craft n'a été exécuté. Les fichiers de preuve sont dans `data/catalog-research/`.
