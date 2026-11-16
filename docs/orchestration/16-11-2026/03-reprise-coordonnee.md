# Reprise coordonnée du 16 novembre

Lucie autorise la reprise après la passation et précise : « que chacun explore les diffs récents de son sujet peut-être avant de commencer ».

## Référence et première étape

Référence lue et récupérée : `927941a67a5805450d49f1e04b39fdd542a6059b`.
Les treize pairs ont reçu leur page du 16 novembre et confirmé leur lecture. La première mission commune est un audit en lecture seule des changements récents de leur périmètre, de leurs tickets et de leur éventuel WIP contre master.

Chaque retour donne sa référence, les changements de contrat, le travail déjà livré à ne pas rejouer, le premier ticket encore valide, les chemins à réserver et les validations proposées. Les anciens GO et échéances restent éteints. Le démarrage de code et la répartition des passes suivent ces constats ; aucun tour de push de code n'est ouvert pendant cet audit.

## Ressources relevées au démarrage

- Portable : `poste --qui` libre, zéro lourd sur une place.
- Luciepc : un lourd sur quatre, TSan du dictionnaire (`concurrence_test.tsan`, PID du tenant 2369995). Son résultat et son propriétaire sont à relever par cœur, sans arrêt ni relance par l'orchestration.
- Arbre de Lucie : les modifications préexistantes du sous-module codeparsers et le dossier non suivi `extension/rag3weaver/docs/lucie-tmp/` sont conservés.

Ces relevés sont ponctuels : ils ne réservent pas une place pour une passe future. Les validations utilisent les verrous, les plafonds et les garde-fous courants du dépôt, dont les suites exigées.

## Priorités de reprise

1. Moteur : lire le résultat TSan existant, puis reprendre les défauts par gravité avec cœur et hotfixs sur des fichiers attribués.
2. Algareth : reprendre le lot du pas qui va au bout et les défauts de contexte/délivrable ; rejouer la campagne empirique après les changements concernés.
3. Vues et recherche : distinguer ce qui est livré (écrivain, chargeur, outils sous vue) de la suite par branche/worktree et des bascules qui restent à préparer.
4. Lots indépendants des autres périmètres : choisir à partir des audits, sans refaire les corrections du week-end ni mettre deux propriétaires sur les mêmes chemins.

La lecture des audits précède cette attribution. Aucun chantier nouveau n'est créé pour occuper une session.

## Opérations séparées de la reprise de code

Les poses de bibliothèque et les migrations de bases restent sous leur plan et leur propriétaire. Une reprise générale n'est ni une réactivation d'un ancien créneau ni une permission de contourner le refus de fusion de la PR lucivy #42. Aucun nettoyage, tag ou publication n'est lancé à ce stade.

Le suivi de la reprise est dans [le ticket](../tickets/2026-11-16-reprendre-les-perimetres-apres-lecture-des-diffs-recents.md).
