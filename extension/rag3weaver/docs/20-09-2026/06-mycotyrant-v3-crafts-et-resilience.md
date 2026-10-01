# Amélioration de Mycotyrant / Vanguard v2

Entrée exacte : `experiments/mtga/data/deck-drafts/mycotyrant-vanguard-v2.arena.txt` et JSON associé. Sorties distinctes : `mycotyrant-vanguard-v3.arena.txt`, `mycotyrant-vanguard-v3.json`, `mycotyrant-vanguard-v3-upgrade.md`. La v2 n'est pas écrasée.

## Recherche et provenance

Preuves sous `experiments/mtga/data/deck-research-2026-09-20/mycotyrant-upgrade/` : recherche hybride des effets de sortie du cimetière, sélection de candidats nommés, sélection des terrains et supports, puis `final_pool.json` pour toutes les quantités de la liste finale. Transport MCP officiel, backend rag3db/lucivy et dense BGE-M3 local.

Les métadonnées de rareté et les candidats non possédés ont été consultés par nom exact dans le catalogue SQLite source local et enregistrés dans `craft-metadata.json`. Cette étape complète le moteur : `OwnedCard` ne contient ni les cartes non possédées ni la rareté. Aucune recherche sémantique sur un catalogue complet ne doit être revendiquée. Les codes de rareté bruts ne sont pas remappés globalement sans preuve ; les raretés des deux crafts discutés sont confirmées par les références liées dans le guide.

## Choix

- Deux Plaines vers deux Bleachbone Verge déjà possédés : accès noir précoce accru, blanc désormais conditionnel pour ces deux terrains.
- Skeleton, Six et Spider pour les déclenchements réguliers et la récupération après interaction.
- Quatrième Unearth contre un Victimize : coût prévu d'un joker commun, marqué non exécuté.
- Vile Mutilator contre Ghalta : effet d'arrivée utile avec une main pauvre ; Hoarding Broodlord conservé.
- Deux Blessings, deux Orbs et noyau Mycotyrant/Vanguard conservés.
- Haywire Mite discuté en option, hors liste ; aucun joker rare/mythique proposé comme achat obligatoire.

## Vérification et limites

Reproducteur : `experiments/mtga/scripts/build_mycotyrant_v3.py`. Validation du total 60, 24 terrains, des deux resets et des contraintes centrales, des quantités par impression couvertes par propriété ou craft explicite, de l'export et du snapshot unique.

Une première tentative de réutiliser les fiches de la v2 a échoué sur l'absence de `owned` (ancienne représentation `owned_this_printing`). Résolution : toutes les fiches exportées viennent du nouveau pool MCP, la v2 ne fournit que la composition de départ et l'identité source.

Pas de craft réalisé, pas de resynchronisation des jokers, pas d'import ni partie de validation. La v3 vise la résilience et comporte des compromis documentés, notamment deux créatures à un mana de moins et le blanc conditionnel. Elle n'est pas déclarée supérieure sur la base d'un taux de victoire inexistant.
