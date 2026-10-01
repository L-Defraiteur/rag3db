# Retour en bibliothèque : élargissement des formulations

L'utilisatrice demande explicitement de rechercher plusieurs formulations, et pas seulement le texte littéral « retour dans le deck ».

## Recherches réalisées

Cinq requêtes vector seul via le MCP : shuffled into library from graveyard, leaves graveyard, bottom of library, return graveyard to library, graveyard from anywhere. Résultats conservés dans `experiments/mtga/data/deck-research-2026-09-20/mill-payoff/`, avec `wording-requests.json`. Chaque requête demande 50 résultats, sans filtre de couleur pour ne pas éliminer les pistes hors couleurs.

Complément dans le catalogue source Arena complet, y compris cartes non possédées : familles de déclencheurs autour de shuffle, leaves graveyard, put into a library, top/bottom, et référence à la carte elle-même. Les regex larges produisent des faux positifs (notamment effets d'arrivée avec une recherche puis mélange) ; les clauses déclenchantes et les zones ont été relues pour les candidats retenus. Preuves : `catalog-wording-families.json` et `wording-findings.json`. Ce complément n'est pas une recherche dense du catalogue non possédé.

## Nouveaux candidats utiles

| Carte | Coût | Possession | Effet pertinent | Zone nécessaire |
|---|---|---|---|---|
| Soul Enervation | 3B | 1 | Sortie d'une ou plusieurs cartes de créature : chaque adversaire perd 1, on gagne 1 | Champ de bataille ; flash permet une installation à une fenêtre de priorité pendant la pile |
| Undead Hand Ninja | 1B | 0, uncommon à crafter | Même drain ; deathtouch | Champ de bataille ; valeur de mana 2 compatible avec Unearth |
| Fuming Effigy | 3R | 0, commune | Sortie d'une ou plusieurs cartes quelconques : 1 blessure à chaque adversaire | Champ de bataille ; rouge hors du deck actuel |
| Ark of Hunger | 2RW | 0, rare | Sortie d'une ou plusieurs cartes : 1 blessure à chaque adversaire et gain de 1 | Champ de bataille ; coût/couleurs moins adaptés |

Toutes couvrent le remélange de Gaea's Blessing quand les types de cartes requis sont présents. Leur « une ou plusieurs » signifie **un seul déclenchement pour un mélange simultané**, pas un par carte. Konrad reste différent : un déclenchement par carte de créature qui quitte le cimetière.

La recherche a également trouvé le libellé exact « one or more cards are put into a library from anywhere » sur Dutiful Knowledge Seeker et Wan Shi Tong, All-Knowing : preuve qu'une recherche limitée à shuffle rate des familles. Ces cartes donnent respectivement des marqueurs et des jetons, pas des dégâts immédiats depuis une zone cachée.

## Contraintes strictes et limites

Creeping Chill reste le candidat direct trouvé pour infliger des blessures sans lancer ni installer la carte. Pas de second effet identifié remplissant toutes les contraintes du retour répété au deck sans présence sur le champ de bataille. Ce résultat ne constitue pas une preuve d'inexistence dans tout Magic ou toutes les combinaisons de cartes.

Gixian Recycler peut bien se déclencher lorsqu'il est meulé sans être installé et ajouter une copie au cimetière ; il ne blesse personne et ajoute des déclenchements. Oglor peut donner perpétuellement des capacités de sortie du cimetière, mais nécessite une installation préalable et produit des jetons. Ces cartes ne sont pas présentées comme des corrections du chronomètre.

Références complémentaires :

- Soul Enervation et règle du déclenchement groupé : https://media.wizards.com/2024/downloads/MKM_Release_Notes_82nnDTWVBdD/EN_MTGMKM_ReleaseNotes_20240125.pdf
- Rareté du Ninja : https://mtg.wtf/card/msc/670/Undead-Hand-Ninja
- Ark of Hunger et règle du déclenchement groupé : https://media.wizards.com/2026/downloads/SOS_Release_Notes_FwhcBWdFIE/EN_MTGSOS_ReleaseNotes_20260410.pdf

Pas de deck écrasé ni de craft exécuté durant cette recherche.
