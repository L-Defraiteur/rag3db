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

Les treize audits ont été reçus : chacun cite la référence ci-dessus, les changements de contrat, le WIP et le premier ticket valide. Aucun chantier nouveau n'est créé pour occuper une session.

## Attribution après les audits

Les préparations se font sur des branches `cx-…`, avec les tickets déjà désignés dans les passations. Aucun tour de push de code n'est ouvert. Pendant la fenêtre ci-dessous, aucun nouveau lourd des autres sessions ne part ; les lectures, le code privé et les petits témoins hermétiques bornés peuvent avancer.

| Porteur | Premier travail confié | Frontière |
|---|---|---|
| optimiseur | Audit frais, qualification et bascule de la bibliothèque, hôtes à la source gelée | Seul porteur des poses et des services ; préavis avant coupure, preuves et FINI aux deux postes |
| arbre principal | Préparer puis réaliser la migration des trois bases après le FINI de G | A1 des sondes reporté ; instantanés, retour, contrôles de graphe et vecteurs |
| cœur C++ | Lire le TSan en cours et poursuivre le diagnostic du dictionnaire | Ne pas intégrer le correctif non prouvé ; seul propriétaire de sa chaîne TSan |
| hotfixs | Rapprocher le ticket ADD COLUMN de son témoin déjà livré ; ensuite inspection `rows_outside_graph` | Ne pas recréer le témoin ; aucun élargissement aux fichiers du cœur sans coordination |
| archi | Préparer le landing du lot où le pas va au bout | `teammate.rs`, `teammate_tests.rs` ; nouvelle qualification après la fenêtre |
| everything declarative | Préparer le landing de l'identité stable des ex æquo, puis le jeton Vertex | `search.rs`, `generic_search_nodes.rs`, témoin de `backend.rs` ; pas de nouvelle passe avant FINI G |
| recherche | Rafraîchissement et datation de `fts_counts` depuis le relevé existant | Pas de purge ni nouveau jeton ; landing après ED, fonctions de backend/catalogue à préciser |
| embarquement | Clé du débit par adresse et poste, reprise de `rate-key` | Aucun fournisseur activé pour lire une clé ; `None` ne devient pas « local » |
| banc | Témoin du débit au faux service, entrée 6 pas 2 | Aucun appel aux services partagés ; clé à coordonner avec embarquement |
| mémoire | Reproduire et corriger le `Text file busy` des fixtures | `codex_threads.rs` côté tests ; hypothèse du FD hérité à prouver |
| tty | Alias anglais compatibles des commandes et options | `commandes.rs`, `main.rs`, tests et README ; aucune modification d'hôte ou de thème |
| codeparsers | Terminer le WIP des mots vides Rust et du std homonyme | Réservation à confirmer avant code ; pointeur et règle d'analyse dans le même lot |
| lucivy | Préparer la validation de l'ulp en parse et retrouver le corpus | Si absent, consigner le blocage avant l'entrée suivante ; aucune fusion de PR #42 ni publication |

Le témoin ADD COLUMN existe déjà sur master : c'est un résultat de l'audit de hotfixs, pas un nouveau vert d'exécution. Les pages du 13 sont historiques. Les champs et mécanismes de générations, vues, socle, archiviste et services livrés du 14 au 16 ne sont pas à réimplémenter.

## Opérations séparées de la reprise de code

Après le résumé demandé sur le travail du week-end, Lucie donne un mot distinct : « il faut pas hésiter à le faire nouvelle bibliothèque et migration des bases ». Ce mot autorise la fenêtre maintenant ; il ne faut pas redemander une heure à l'utilisatrice.

G prend la priorité : bibliothèque `59ac9c86bc`, audit de fraîcheur des artefacts, ABI et preuves, hôtes de la source `927941a67a5805450d49f1e04b39fdd542a6059b` si les binaires préparés sont périmés, qualification sans moteur ancien, puis pose aux deux postes selon le plan du 16. Le code de master reste gelé pendant cette préparation ; la documentation reste sur la branche de coordination. Le TSan reste sous le contrôle de cœur, que G contacte pour la place si nécessaire.

Arbre prépare les migrations et les réalise seulement après le FINI vérifié de G : instantanés et retour, contrôles des deux sens, maintenance, 1b et montée aux générations, repassage puis contrôles visibles, miroirs et vecteurs. Le retour est gardé 72 heures. Aucun retrait d'ancien index ni nettoyage des ressources n'est inclus maintenant.

Les poses et migrations restent sous leurs plans et propriétaires. Ce mot n'autorise ni un contournement du refus de fusion de la PR lucivy #42, ni un tag ou une publication. Une cause bloquante est rapportée avec ses preuves ; les autres préparations continuent.

Le suivi de la reprise est dans [le ticket](../tickets/2026-11-16-reprendre-les-perimetres-apres-lecture-des-diffs-recents.md).
