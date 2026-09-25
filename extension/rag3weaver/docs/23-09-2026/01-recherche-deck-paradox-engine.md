# Recherche Paradox Engine et copies via le moteur

Demande : produire une liste Arena autour de Paradox Engine et de copies de créatures, avec une orientation bleue.

## Chemin réellement exécuté

Le pilote `experiments/mtga/scripts/mcp_queries.py` appelle le MCP généré par `serve_engine_mcp.sh`. Recherches dans OwnedCard, avec lucivy et les vecteurs BGE-M3 existants, puis sélections structurées via rag3db. Aucun remplacement de la recherche par un filtrage Python du snapshot. Python sert ensuite à allouer les impressions et à vérifier indépendamment les quantités.

Sept recherches hybrides : copies, dégagement, production de mana, pioche sur créatures, menaces, retours en main et cibles de copie. Les métadonnées attestent `vectorCount > 0` et `partial: false`. Une sélection exhaustive des textes contenant `copy` complète la découverte approximative ; une sélection exacte récupère le pool final. Les terrains sont cherchés via leur type et leur production de mana, puis leurs restrictions sont relues.

## Difficultés et résolution

1. Le bac à sable interdisait l'accès à `127.0.0.1:7878`. Relance du même pilote avec l'autorisation réseau locale ; les recherches hybrides fonctionnent.
2. Un filtre exploratoire sur `mana_value` a été rejeté : cette propriété n'existe pas dans le schéma OwnedCard utilisé. Retrait de ce filtre et nouvelle recherche avec les propriétés disponibles. Les coûts sont relus depuis `mana_cost`. Aucun champ fictif ni résultat partiel n'est présenté comme valide. Amélioration future : exposer une valeur de mana structurée tenant compte des faces et des sorts à X ; pas de modification du moteur dans cette tâche.
3. Les requêtes lexicales sans opérateur booléen signalent une interprétation OR de sous-chaînes. Le dense est bien actif, mais les résultats doivent être relus ; les sélections exactes/exhaustives complètent cette découverte.

## Livrables privés

Dans `experiments/mtga/`, toujours ignoré par Git :

- `data/deck-research-2026-09-23/paradox/` : requêtes et réponses MCP.
- `scripts/build_paradox_engine.py` : assemblage reproductible et assertions.
- `data/deck-drafts/experiments-2026-09-23/01-paradoxe-des-miroirs/` : export Arena, JSON, validation et guide de jeu.

Résultat : bleu-vert, 60 cartes dont 24 terrains, dix créatures de mana et sept cartes dédiées aux copies. Aucun craft requis selon le snapshot déjà ingéré ; inventaire non rafraîchi. Quantités vérifiées par impression, identités couleur contrôlées, 16 sources terrestres potentielles de vert et 15 de bleu. Import client et parties non exécutés.

La construction relève aussi un risque fonctionnel : cinq copies simultanées de Vaultborn Tyrant devant l'original déclenchent 30 pioches, sans multiplicateur supplémentaire. Le guide le signale afin d'éviter une défaite par bibliothèque vide.
