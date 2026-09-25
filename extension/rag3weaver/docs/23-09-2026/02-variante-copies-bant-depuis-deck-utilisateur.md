# Variante copies Bant depuis le deck utilisateur

La demande a évolué : examiner les ajouts à `paradox engine copy`, ajouter le blanc et Reflection Net, puis retirer Paradox Engine pour privilégier les copies et la production de mana. Ojer Taq a ensuite été explicitement demandé et ajouté à la place de Season of Weaving.

## Source et recherche

Lecture ciblée des objets StartHook dans le Player.log actuel, sans afficher les autres données du journal. Deux versions du deck étaient présentes, 60 puis 69 cartes ; la dernière a été conservée comme source privée. Elle ajoute Relm's Sketching, Extravagant Replication, deux Doppelgang, For the Common Good et quatre terrains à la première proposition.

Recherche réelle via MCP : sélection des cartes clés et terrains, deux recherches hybrides sur la production de mana et les copies blanches, sélection exacte des impressions finales. Les deux recherches hybrides ont un `vectorCount > 0` et `partial: false`. Le pool d'assemblage ajoute l'enregistrement Ojer Taq déjà retourné par la première sélection MCP. Les quantités sont contrôlées indépendamment sur le snapshot de collection ; pas de nouvelle capture mémoire ni de réingestion complète pendant cette tâche.

## Résultat et contrôles

70 cartes, 28 terrains, 15 cartes de copie, deux multiplicateurs de jetons. Dix créatures produisent naturellement du mana ; deux Cryptolith Rite et deux Enduring Vitality le permettent à toutes les créatures. Trois Reflection Net, un Niko et un Ojer exploitent le blanc. Aucun Paradox Engine ni Transit Mage. Les ajouts de copies personnels sont préservés.

Le constructeur valide les quantités par impression, le maximum de quatre par nom hors bases, les identités couleur, l'absence d'Engine, les ajouts conservés et les comptes. Sources terrestres potentielles : 17 vertes, 15 bleues, 9 blanches, complétées par le mana des créatures. Aucun craft requis dans le snapshot. Import Arena et performances non testés.

Les différences de règles sont explicitées dans le guide : créer un jeton, faire arriver une créature et transformer un permanent en copie sont distincts. Ojer triple seulement les jetons de créature ; Reflection Net n'en crée pas. Les gros déclenchements de pioche du Tyrant peuvent provoquer une défaite par bibliothèque vide.

## Livrables ignorés par Git

- `experiments/mtga/data/deck-research-2026-09-23/bant-copies/` : journal ciblé, requêtes, résultats, choix et provenance d'assemblage.
- `experiments/mtga/scripts/build_bant_copies.py` : assemblage reproductible et validation.
- `experiments/mtga/data/deck-drafts/experiments-2026-09-23/02-bant-miroirs-et-mana/` : export Arena, deck structuré, source, différences, validation et guide.
