# Nouveau thème : grenouilles et effets d'arrivée

Après un retour utilisateur positif sur le deck Rakdos (retour informel, pas de logs de match analysés), demande d'une nouvelle création en changeant de thème/couleur/type de créature.

## Recherche et choix

Trois recherches hybrides MCP sur `OwnedCard` : endurance/défenseur, grenouilles/rebonds, véhicules. Réponses et requêtes : `experiments/mtga/data/deck-research-2026-09-20/tribal/`.

La piste endurance manquait des pièces centrales recherchées. La piste grenouilles avait une densité nettement supérieure : 4 Clement, 4 Prophet, 4 Scribe, 4 Druid, 2 Port-Mage, 2 Charmer, 3 Lurker, 3 Mentor possédés. Une seconde sélection MCP vérifie les noms, exemplaires et supports. Les quatre Breeding Pool et quatre Cavern of Souls améliorent aussi la faisabilité du thème tribal.

Choix final : **La mare à téléportation**, 26 grenouilles, 10 autres sorts, 24 terrains, Historic BO1. Trois Clement et deux Mentor dans la liste ; sélection d'une densité de moteurs, pas inclusion de tous les exemplaires possédés. Aucun craft ni reprise d'une liste de classement.

## Livrables et validation

- Dossier : `experiments/mtga/data/deck-drafts/experiments-2026-09-20/03-la-mare-a-teleportation/`.
- `arena.txt`, `deck.json`, `validation.json`, `README.md`.
- Reproducteur : `experiments/mtga/scripts/build_frogs_engine.py`, lisant exclusivement `frog_pool.json`, résultat réel du MCP.
- Assertions : 60 cartes, 24 terrains, 26 créatures dont le type contient Frog, maximum quatre non-basiques par nom, propriété par impression, bases débloquées, un seul snapshot, total de l'export.

Les recherches utilisent le même backend rag3db/lucivy/BGE-M3 local. Le script d'assemblage ne recherche pas les cartes dans le catalogue brut. Pas de modification du moteur nécessaire pour cette construction.

## Précisions de règles conservées

Portal et l'activation de Mentor sont des actions de rituel. Le blink perd les anciens marqueurs ; un jeton exilé ne revient pas. Clement et Cavern fournissent du mana coloré restreint, et le mal d'invocation compte pour la capacité de mana conférée aux grenouilles. Les réductions de Charmer n'enlèvent pas les symboles colorés et ne changent pas les valeurs de mana comparées par Clement.

Le README décrit une séquence à trois cartes piochées avec Portal + Prophet + un Port-Mage restant sur le champ de bataille. Aucune boucle infinie ni performance mesurée n'est revendiquée. Import et parties à tester par l'utilisatrice.
