# Magic : conserver les moteurs et améliorer les premiers tours

Date : 19-09-2026. Analyse du snapshot local de `mycotyrant_insidious TRIM (2)`, format déclaré Historic. Aucune modification dans Arena.

## Contraintes confirmées par Lucie

- Conserver au moins deux cartes qui remélangent la bibliothèque pour se protéger de l'auto-meule et de la meule adverse. Ce n'est pas une garantie absolue face à toute interaction.
- Conserver Thunderbond Vanguard : transformer les jetons en copies est une composante recherchée du deck.
- Améliorer l'accès à Vanguard et les actions utiles avant trois manas.
- Le passage de 100 à 60 cartes reste une proposition, pas une préférence confirmée.

Le premier brouillon noir/vert ne respectait pas ces priorités : son JSON expérimental est marqué `superseded_do_not_use`. Son export texte initial ne constitue pas une recommandation à importer.

## Constats et leviers

La liste exportée contient 100 cartes, 37 terrains sur leur face principale, dont 18 arrivent systématiquement engagés. Les cartes modales avec verso terrain sont en supplément. La difficulté avant trois manas vient donc aussi du tempo des terrains, pas seulement du nombre de sorts bon marché.

Thunderbond Vanguard coûte {2}{W}. Deux exemplaires sont possédés et déjà joués. Son coût permet de le ramener avec Unearth pour {B}, après meule ou surveil. Trois Unearth sont possédés, un seul est dans la liste actuelle : passer à trois est une piste immédiate, également utile pour Mycotyrant et les petites créatures. Unearth ne résout cependant pas une main sans créature exploitable au cimetière.

Malevolent Rumble, possédé en quatre exemplaires, coûte {1}{G} : il cherche un permanent parmi les quatre premières cartes, met le reste au cimetière et crée un jeton sacrifiable pour un mana incolore. Il peut trouver Vanguard, Roots ou un terrain, alimenter la réanimation et aider à payer les manas génériques de Vanguard. Il ne fournit pas le blanc. Une fois Vanguard présent, son jeton devient une copie de Vanguard et n'a plus sa capacité de sacrifice pour du mana.

Overlord of the Balemurk, déjà présent deux fois et possédé quatre fois, se joue pour {1}{B} avec impending : son arrivée meule quatre cartes et peut récupérer Vanguard en main. Sous impending, ce n'est pas immédiatement une créature capable de bloquer. Ne pas le compter uniquement comme un sort à cinq manas, ni comme une défense tour deux.

Assemble the Team coûte {B}{G}, cherche dans le tiers supérieur de la bibliothèque et peut trouver Vanguard. Quatre exemplaires possédés, un joué. Ce n'est pas un tuteur garanti sur toute la bibliothèque ; augmenter d'abord les cartes qui développent aussi le terrain de jeu paraît préférable pour tester le problème des premiers tours.

Petites créatures possédées : quatre Stitcher's Supplier, quatre Rubblebelt Maverick et quatre Snarling Gorehound. Le premier remplit le cimetière à son arrivée et à sa mort ; le deuxième surveille immédiatement et peut quitter ensuite le cimetière pour déclencher Roots. Gorehound demande une autre arrivée de créature : son intérêt dépend davantage de la densité du deck.

Terrains possédés pouvant accélérer les départs : deux Blooming Marsh, deux Overgrown Tomb répartis sur deux impressions, deux Temple Garden et un Godless Shrine. Les shocklands peuvent arriver dégagés au prix de deux points de vie. Comparer d'abord leur remplacement de terrains engagés, en préservant les sources blanches ; ne pas supprimer indistinctement Underground Mortuary, dont surveil soutient le plan.

## Comprendre le rôle de Vanguard

Vanguard transforme les nouveaux jetons en copies : les Fungus et Plants remplacés ne conservent pas leurs caractéristiques initiales. Mycotyrant peut continuer à produire des jetons devenant de grosses copies, mais ces copies ne sont pas des Fungus/Saprolings qui augmentent sa propre force. De même, Roots crée toujours un jeton et donne toujours sa capacité de mana aux créatures-jetons, mais sa distribution de marqueurs ne renforce que les Plants restant réellement de ce type. Le mal d'invocation reste à prendre en compte pour le mana.

Cette interaction est un changement de mode de victoire choisi, pas une raison de couper Vanguard.

## Première direction de test

Préserver resets et Vanguard ; augmenter Unearth, introduire Malevolent Rumble et davantage de petites créatures à effet immédiat, puis remplacer une partie des terrains engagés. Financer ces changements en réduisant les sorts chers redondants. Définir la liste exacte selon le choix de taille et la place souhaitée pour les grosses cibles de réanimation. Aucune performance en partie n'a encore été mesurée.

Sources : textes, quantités possédées et composition extraits des fichiers locaux `experiments/mtga/data/cards.jsonl` et `decks.json`. Il s'agit d'une analyse directe du snapshot, pas d'une nouvelle exécution de recherche via rag3weaver.
