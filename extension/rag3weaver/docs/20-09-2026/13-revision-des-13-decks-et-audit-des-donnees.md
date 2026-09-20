# Révision des 13 decks avec le moteur et audit des données

Date : 20-09-2026.

## Livrables

Les 13 expériences du 19 septembre ont une révision dans :

`experiments/mtga/data/deck-drafts/experiments-2026-09-19/revisions-2026-09-20/`

Le README central donne les liens vers les imports Arena, les JSON, les changements et les plans de jokers. Les anciennes listes sont conservées. Tailles maintenues : 70, 80, 90 ou 100 cartes selon l'expérience. Aucun import dans Arena ni match effectué avec ces nouvelles listes.

La découverte utilise le vrai MCP rag3weaver : 37 recherches denses directes, deux recherches denses suivies de relations capacités → cartes, filtres de possession/couleur/type/coût et sélections exactes. Les requêtes et réponses sont conservées dans `research/`. Certaines formulations très larges ont produit des candidats hors intention ; les textes principaux des finalistes ont été relus. La similarité sémantique n'est ni une preuve de combo ni une recherche exhaustive.

## Choix de construction

- Réanimation : plus de récupération à faible coût, Zombify à quatre manas et un drain accessible avec Unearth.
- Cimetière autonome : Skeleton/Specimen, Ooze et drain ; suppression d'Orb pour contrôler les activations.
- Terrains : Analyst/Spelunking/Scute/Necrobloom et Corpse Knight. Vanguard est retiré pour ne pas remplacer les copies de Scute par des clones sans landfall.
- Ménagerie : davantage d'effets d'arrivée et moins de cibles dont l'intérêt dépend du lancement ou qui remélangent le cimetière avant la réanimation.
- Weaver : récupération générale, Timeless Witness et drain ; l'exil de Weaver depuis le champ de bataille ne déclenche pas Roots.
- Multiplicateurs : Ocelot, gains de vie et producteurs précoces ; deux Ocelot effectivement possédés.
- Crawler : la permission de jouer depuis l'exil ne dispense pas de payer ; Broodlord apporte la convocation aux sorts concernés.
- Plantes : conservation du type Plant et des marqueurs ; pas de Vanguard.
- Ultimatum et trésors : quatre couleurs conservées et sources rouges renforcées ; WWBBBRR reste exigeant. Les couleurs des créatures utilisées pour convoquer comptent.
- Rouge mixte, jetons-dégâts et célérité : passage à Jund pour donner au rouge une vraie place sans maintenir le blanc. Gatekeeper, Tremors et producteurs de célérité spécialisés.

Deux Gaea's Blessing restent dans chaque liste. Aucune n'a plus d'une Mesmeric Orb. Cela réduit la charge potentielle de déclenchements sans garantir l'absence de dépassement du temps Arena. Scute et les multiplicateurs peuvent encore produire des séquences longues.

Les capacités colorées des terrains sont documentées avec leurs conditions. Les comptes de sources ne constituent pas une simulation des premiers tours. Les Verge ne produisent pas toujours leur seconde couleur ; un Thriving ne devient pas une source simultanée de toutes les couleurs possibles.

## Jokers

Budget partagé selon le snapshot du 19 septembre à 16:36:49 UTC : **5 communs, 12 peu communs, aucun rare ni mythique**. Le calcul prend le maximum d'exemplaires demandé entre les variantes, pas la somme : les mêmes cartes peuvent servir dans plusieurs decks alternatifs.

Ocelot/multiplicateurs et rouge/dragons n'exigent aucun craft. Aucun achat exécuté. La disponibilité effective des impressions et la légalité finale restent à confirmer dans Arena.

## Défaut de possession corrigé

Le snapshot ne fournit pas toujours de lien entre une impression originale et sa version rééquilibrée. `ownership_model` les comptait alors comme deux familles indépendantes. Exemples vérifiés :

- Ocelot Pride : deux exemplaires originaux plus deux signalés pour leur version rééquilibrée donnaient quatre annoncés ; il faut compter deux.
- Moss-Pit Skeleton : quatre plus quatre donnaient huit ; il faut compter quatre.

Un rapprochement de secours utilise le nom normalisé, l'édition et le numéro de collection, uniquement avec un original primaire non-jeton unique. Le maximum des quantités est utilisé dans la famille, sans addition. Des éditions ou numéros différents restent distincts. Les liens explicites existants sont toujours traités.

Les quatre tests de `test_catalog_model.py` passent, dont le nouveau cas de lien absent. Les 360 lignes catalogue affectées ont été réingérées par MCP ; les quantités des cartes retenues ont été relues. Le générateur alloue ensuite les exemplaires par impression et vérifie que les seuls manques correspondent au plan de jokers.

## Nouveau défaut de persistance : capacités imbriquées — ouvert

La validation complète après réouverture **échoue** dès le premier lot de CatalogCard. Les identités et nombres de résultats correspondent, mais certains textes à l'intérieur de `abilities` sont associés aux mauvais éléments. Par exemple, une capacité de Roar of the Wurm reçoit un texte de protection alors que le champ principal `text_en` de la carte reste correct dans la comparaison.

Sur les **1 118 impressions** relues pour les finalistes, **46** présentent cet écart. Dans ce périmètre, tous les autres champs correspondent à la projection source, y compris noms, texte principal, couleurs, éditions et quantités. Ce constat ne démontre pas l'intégrité des autres entités ou de toute la base.

Le défaut a été observé après les mises à jour de possession et une fermeture/réouverture. Sa cause native précise n'est pas établie. Ne pas confondre une ingestion acquittée avec une preuve de persistance correcte. Ne pas marquer les tests d'intégration catalogue comme réussis.

Pour livrer les decks avec des données cohérentes :

1. La recherche et la sélection restent issues du MCP ; les réponses brutes sont conservées.
2. Chaque champ des impressions utilisées est comparé à la projection du snapshot.
3. Seul `abilities`, dont l'écart est identifié, est réparé dans les **exports** depuis cette source ; tout autre écart ferait échouer l'export.
4. L'audit est sauvegardé dans `research/export-integrity-audit.json`, et cette provenance est déclarée dans les JSON et README.

Cette réparation des exports **ne corrige pas le stockage du moteur**. Les filtres ou rendus fondés sur les tableaux de capacités persistés ne doivent pas être tenus pour fiables avant résolution. Les listes Arena exportées utilisent uniquement noms, éditions, numéros et quantités, vérifiés indépendamment des tableaux affectés.

Suite technique à traiter séparément : reproduction minimale d'une mise à jour partielle d'un tableau de structures contenant des chaînes, puis checkpoint/réouverture ; comparaison des lignes modifiées et non modifiées. Vérifier aussi les champs texte simples, les autres champs imbriqués et les relations. Après correction native, reconstruire les données affectées depuis le snapshot et repasser la validation intégrale.

## Validation des livrables

Les 13 listes passent les contrôles de taille, maximum de quatre hors bases, séparation terrains/sorts, allocation des impressions, coût des jokers et cohérence entre JSON et texte Arena. Les hashes des JSON originaux sont conservés dans `validation.json`. Les liens locaux des README sont vérifiés. Les capacités réparées des exports sont vérifiées contre le snapshot brut.

Ces contrôles valident les fichiers et les données nécessaires à leur construction ; ils ne mesurent pas leur taux de victoire.
