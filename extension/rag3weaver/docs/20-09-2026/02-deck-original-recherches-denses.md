# Deck original : recherches denses et validation des interactions

## Demande et résultat

Construire un deck plus farfelu depuis la collection, en réfléchissant aux stratégies adverses plutôt qu'en reproduisant une liste de classement. Résultat : **Le fisc de Rakdos**, Historic BO1, 60 cartes dont 24 terrains, sans craft selon le snapshot. Nouvelle construction, non dérivée des anciens decks.

Artefacts dans `experiments/mtga/data/deck-drafts/experiments-2026-09-20/02-le-fisc-de-rakdos/` : `arena.txt`, `deck.json`, `validation.json`, `README.md`. Reproduction de l'export : `python3 experiments/mtga/scripts/build_rakdos_engine.py`.

## Chemin effectivement utilisé

Client SDK MCP officiel → backend généré → rag3db persistant, lucivy, embeddings BGE-M3 locaux. Requêtes et réponses conservées sous `experiments/mtga/data/deck-research-2026-09-20/farfelu/`.

- `requests.json` : quatre recherches hybrides BM25 + vector (punition, dégâts, conditions de victoire atypiques, artefacts) et sélection de candidats par nom.
- `requests2.json` : trois recherches vector seul (dégâts, gains de vie ciblés, renvoi des dégâts), plus sélection exacte des supports possibles.
- `requests3.json` : propriété des pièces de punition/pioche, des réponses et de terrains.
- `requests4.json` : autres supports, récursion, sorts rapides et terrains conditionnels.
- `requests5.json` : sélection finale complète ; `chosen_deck_pool.json` est la seule source des fiches du constructeur de deck.

Les résultats MCP déjà sauvegardés lors de la recherche précédente ont aussi été parcourus pour identifier des interactions supplémentaires et des terrains. Aucun filtrage du fichier brut de collection n'a remplacé le moteur. Les textes des cartes viennent du moteur ; le web a servi à vérifier les interdictions et le contexte officiel du format, sans copier de decklist.

## Difficultés et choix

1. **Sandbox réseau** : le premier lancement ne pouvait pas joindre `127.0.0.1:7878` (`Operation not permitted`). Relance autorisée hors sandbox ; recherches terminées. Ce n'était pas un crash du moteur ni une disparition du snapshot.
2. **Pertinence sémantique** : des recherches larges sur les blessures ou la vie remontent aussi des cartes seulement apparentées. Le dense sert à découvrir ; la validation lit les capacités exactes. Les résultats vector seul restent des top-k, pas une preuve d'exhaustivité.
3. **Plusieurs pistes trop peu redondantes** : le renvoi de dégâts avait deux Nemesis et un Barbed Servitor, mais pas les grosses pièces ciblées pour former le plan principal ; Tainted Remedy avait deux exemplaires, sans supports de gain de vie ciblé convaincants dans les résultats examinés. Ces recherches n'ont pas été présentées comme un inventaire exhaustif de toutes les combos possibles.
4. **Distinction blessures/perte de vie** : Grievous Wound se déclenche sur les blessures ; Scrawling Crawler ne la déclenche pas. Les deux moteurs se recoupent via Needlehead et Ob Nixilis, mais ne sont pas interchangeables.
5. **Mana et auto-punition** : terrains engagés, exigences RR et mode BBB occasionnel d'Avarice explicités. Magebane blesse aussi son contrôleur. Pyroclasm et Extinction Event peuvent supprimer nos propres créatures.

## Vérifications et limites

Le constructeur vérifie 60 cartes, 24 terrains, 14 créatures, séparation terrains/non-terrains, quantités possédées par impression, maximum quatre par nom hors bases et export Arena. JSON conservé avec identifiants stables, textes, rôles, snapshot et provenance. Pas de craft, pas d'écriture dans Arena.

L'import n'a pas encore été testé et aucun match n'a validé les performances. Le README donne une séquence arithmétique vérifiable et des plans par stratégie adverse, sans inventer un taux de victoire ni une distribution de métagame. La base de mana est le premier compromis à mesurer en partie.

Sources officielles consultées le 20 septembre 2026 :

- https://magic.wizards.com/en/banned-restricted-list
- https://magic.wizards.com/en/news/mtg-arena/state-of-the-formats-2026
