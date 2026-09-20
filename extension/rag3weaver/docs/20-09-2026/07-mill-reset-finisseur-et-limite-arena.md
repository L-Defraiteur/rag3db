# Mill + reset : gagner pendant les déclenchements, avant de saturer Arena

## Problème rapporté

L'utilisatrice signale que Mesmeric Orb ×2, Gaea's Blessing et Insidious Roots produisent des séquences trop longues pour le chronomètre Arena. Les constructions précédentes optimisaient la génération de jetons sans vérifier suffisamment le temps de résolution ni prévoir un finisseur actif pendant cette phase. La v3 de résilience ne corrige pas ce problème.

Avec ces seules cartes, les jetons qui arrivent dégagés ne « deviennent » pas dégagés : ils ne déclenchent pas Orb. Les Orbs créent néanmoins deux meules par permanent dégagé ; avec beaucoup de jetons déjà engagés, les déclenchements secondaires rendent la séquence très longue. Une véritable boucle infinie demanderait un autre mécanisme. Aucun log de la partie n'a été analysé pour trancher le cas exact.

## Recherche

Preuves sous `experiments/mtga/data/deck-research-2026-09-20/mill-payoff/` : recherche MCP hybride et sélection des cartes nommées sur la collection possédée. Complément par prédicats de texte en lecture seule dans le catalogue Arena local complet, pour inclure les cartes non possédées ; résultats conservés avec leur provenance. Le catalogue complet n'est pas indexé dans `OwnedCard`.

## Réponse stricte : Creeping Chill

Carte non possédée dans le snapshot, rareté client 3 (uncommon). Meulée, elle peut être exilée depuis le cimetière pour infliger 3 blessures à chaque adversaire et faire gagner 3 points de vie. Elle n'a besoin ni d'être lancée ni d'être sur le champ de bataille.

Limite décisive : l'exil empêche Gaea's Blessing de la recycler. Quatre exemplaires donnent 12 blessures via ces déclenchements, hors modificateurs ; ce n'est pas un kill automatique depuis 20 points ni une boucle de dégâts à chaque reset. Si Chill et Blessing sont meulées ensemble, faire résoudre Chill avant le mélange pour pouvoir l'exiler ; si elle a déjà quitté le cimetière, son effet conditionné à cet exil ne fonctionne plus.

Référence officielle : https://magic.wizards.com/en/news/feature/ravnica-remastered-release-notes

## Réponse répétable avec une condition supplémentaire : Syr Konrad

Quatre exemplaires possédés. **Doit être sur le champ de bataille** : sa simple présence au cimetière ne fait rien.

Il inflige une blessure par carte de créature mise au cimetière depuis la bibliothèque, et par carte de créature qui quitte notre cimetière. Le mélange de Blessing avec douze cartes de créature crée ainsi douze déclenchements, alors que Roots se déclenche une fois par événement de sortie d'une ou plusieurs cartes. Ces blessures peuvent terminer la partie avant toute attaque.

Konrad est de valeur de mana 5 : Unearth ne le ramène pas. Il faut le lancer ou le réanimer par un effet adapté, tel que Victimize, avant la séquence. Le jouer une fois le temps consommé ne résout rien. Dreadhound, également possédé, exige aussi le champ de bataille et ne couvre pas le trajet retour vers la bibliothèque.

Référence officielle : https://media.wizards.com/2025/downloads/FIN_Release_Notes_3O87RLY5EO/EN_FIN_Release_Notes_2025_1_15.pdf

## Décision

Pas de modification silencieuse du deck ni de craft : la recherche répond au besoin exact mais n'a pas trouvé de finisseur répétable répondant à toutes les contraintes (jamais sur le champ de bataille, simple transit aller/retour, pas d'exil définitif). Creeping Chill est un complément fini ; Konrad est un moteur de victoire conditionnel à son installation préalable. La réduction du nombre d'Orbs reste une mesure séparée contre la saturation, sans garantie de chronomètre à partir d'une analyse de cartes seule.
