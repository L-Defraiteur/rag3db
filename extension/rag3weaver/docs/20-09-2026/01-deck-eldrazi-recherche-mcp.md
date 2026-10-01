# Premier deck construit après validation du MCP complet

Demande : proposer librement un deck puissant avec la collection, puis chercher des remplaçantes possédées pour les cartes coûteuses, manquantes ou interdites. Format retenu : Historique BO1, cohérent avec les essais précédents. Aucun craft ni modification d’Arena.

Toutes les recherches de collection de cette construction passent par le MCP généré : `select_cards` et `search_cards` (lucivy + BGE-M3 local). Le petit client `experiments/mtga/scripts/mcp_queries.py` conserve les réponses structurées. Le serveur n’est pas automatiquement enregistré comme outil dans une session Codex déjà ouverte ; les appels de cette session passent par le SDK MCP officiel.

Exploration : cinq périmètres de couleur, moteurs sacrifice/aggro/lifegain/elfes, tous les terrains, puis vérification des moteurs Eldrazi/enchantements/tempo et des remplaçantes. Le choix final exploite les quatre Labyrinthes, quatre Cavernes et quatre Mycospawn possédés. Nulldrifter fournit à la fois une cible d’empreinte et une possibilité de pioche pour un coût réduit. Une recherche hybride a aussi retrouvé `Ugin, Eye of the Storms` : une recherche nominale précédente avec le mauvais nom `Ugin, the Eye of the Storms` ne le trouvait pas. Illustration utile de la complémentarité entre recherche exacte et sémantique.

Résultat : `experiments/mtga/data/deck-drafts/experiments-2026-09-20/01-eldrazi-faille-d-ugin/`, avec `arena.txt`, `deck.json`, `validation.json` et README. Reproduction : `scripts/build_eldrazi_engine.py`, qui lit uniquement les résultats MCP sauvegardés pour les choix et quantités. Les réponses de découverte sont dans `data/deck-research-2026-09-20/`.

Contrôles : 60 cartes, 24 terrains, quantités non-basiques possédées par impression, quatre exemplaires maximum par nom, impressions des terrains de base débloquées, onze cartes incolores de valeur de mana ≥7 pour l’empreinte. Légalité examinée dans les publications officielles ; import Arena et performance en match non encore mesurés. La documentation distingue les remplaçantes fonctionnelles des effets qui restent réellement absents.
