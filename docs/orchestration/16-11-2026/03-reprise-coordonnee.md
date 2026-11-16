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
| codeparsers | Terminer le WIP des mots vides Rust et du std homonyme | `code.rs`, `e2e_usages` et chemins du sous-module annoncés ; pointeur et règle d'analyse dans le même lot |
| lucivy | Préparer la validation de l'ulp en parse et retrouver le corpus | Si absent, consigner le blocage avant l'entrée suivante ; aucune fusion de PR #42 ni publication |

Le témoin ADD COLUMN existe déjà sur master : c'est un résultat de l'audit de hotfixs, pas un nouveau vert d'exécution. Les pages du 13 sont historiques. Les champs et mécanismes de générations, vues, socle, archiviste et services livrés du 14 au 16 ne sont pas à réimplémenter.

## Opérations séparées de la reprise de code

Après le résumé demandé sur le travail du week-end, Lucie donne un mot distinct : « il faut pas hésiter à le faire nouvelle bibliothèque et migration des bases ». Ce mot autorise la fenêtre maintenant ; il ne faut pas redemander une heure à l'utilisatrice.

G prend la priorité : bibliothèque `59ac9c86bc`, audit de fraîcheur des artefacts, ABI et preuves, hôtes de la source `927941a67a5805450d49f1e04b39fdd542a6059b` si les binaires préparés sont périmés, qualification sans moteur ancien, puis pose aux deux postes selon le plan du 16. Le code de master reste gelé pendant cette préparation ; la documentation reste sur la branche de coordination. Le TSan reste sous le contrôle de cœur, que G contacte pour la place si nécessaire.

Arbre prépare les migrations et les réalise seulement après le FINI vérifié de G : instantanés et retour, contrôles des deux sens, maintenance, 1b et montée aux générations, repassage puis contrôles visibles, miroirs et vecteurs. Le retour est gardé 72 heures. Aucun retrait d'ancien index ni nettoyage des ressources n'est inclus maintenant.

Les poses et migrations restent sous leurs plans et propriétaires. Ce mot n'autorise ni un contournement du refus de fusion de la PR lucivy #42, ni un tag ou une publication. Une cause bloquante est rapportée avec ses preuves ; les autres préparations continuent.

## Premiers retours de préparation

- G confirme les deux bibliothèques au magasin : `librag3db` md5 `697c7a0cdf7a5dc7860a5c13b7e1135f`, `libvector` md5 `925b6b1331e78a3b1bd112497611bd95`. L'hôte de 927 est reconstruit, SHA256 `0e310a9cfe1c9e12534f3a579b3e781dcef7e9335e37d7e57349b16d053d5288` ; lucivy 4.3.9 au lock, essais 7991 verts aux deux postes. Le verdict des trois suites compile encore : **aucun vert global, aucune coupure à ce relevé**. Plan de G publié sur `cx-optimiseur-bascule-16`, tête `806715b2d6` puis complément `dc32e7073a` ; contrôle des backends par `/etat`, pas le vieux script qui ne jugeait pas un 404.
- Arbre a publié la préparation `04a3970be09a06fc5ff1fabd8f8b18cc71d3ee9e` sur `cx-arbre-bascule-bases-20261116-r1`, dont un nouvel outil privé explicitement en lecture seule ; G le compile contre 59ac. Aucun accès d'arbre aux bases vivantes avant FINI G. Banc a publié sa recette d'oracle `df02f39a06c9949d285acd08747824f30cfa0297`, avec un ticket propre, et attend copies finales, manifestes et empreintes du working tree/non suivis. Ses limites de score distinguent BM25 brut, fusion et dense.
- Archi : candidat `e8b147277fcd4cc91c293d5ada08aec42d482ce7`, deux patches conservés au rebase ; aucune nouvelle passe. ED : candidat des ex æquo `f1cfa89542247d5d575f9953c80ce6d83f61dce1`, quatre patches conservés ; préparation du témoin Vertex `d148fe0e8701f1653f6261d9ce601a3e4cbc5dd5`, non joué et sans correctif.
- Mémoire : WIP `61dc67eb18b7c60e81894c4670a4903ee8b02aee`, rouge/vert CPU du mécanisme Linux et de la commande extraite, **pas encore du helper Rust compilé**. Le lot ne revendique pas l'attribution unique du rouge historique. Tty et banc ont publié les témoins d'alias et de débit, non joués en Rust ; banc tête `675cf02000b4c0540a3bd89ebc7eb2384bd74059`.
- Embarquement : contrat de clé accepté (écritures automatiques `address@host`, lecture ancienne en secours ; API historiques explicites conservées), WIP `5fddd5481d0787654f243aad4a07a94f159822db`. Relecture statique mémoire favorable ; code et neuf témoins non compilés/non joués. La provenance lue après une mesure n'est pas une attestation atomique d'un index multi-fournisseur.
- Hotfixs : rapprochement du témoin ADD COLUMN publié `c92d557bd8` ; le rouge du juge global historique reste nommé, seul le contrat du ticket est rapproché des deux témoins réussis. Pour `rows_outside_graph`, la conception doit compter les lignes à vecteur valide de l'instantané absentes du graphe utilisé ; un watermark ou un nombre de candidats n'est pas ce compte exact. Inconnu vaut NULL, jamais zéro. Pas encore de modification de production.
- Lucivy : corpus Linux 7.2 non retrouvé, aucune substitution silencieuse du corpus 2.6.0 ; l'ulp reste bloquée sur la couverture complète. L'entrée `indexed:false` avance par une API de diagnostic pure, témoin publié mais non joué ; aucune fusion ni publication.

### Exception de capacité pour C1

Pour ne pas bloquer le diagnostic du moteur pendant la compilation de G, une seule validation C++ privée est admise après publication du lanceur exact : nouvelle borne de 1 800 s, un poste lourd, 32 Gio, `-j8` au plus, target privé borné, aucune bibliothèque du magasin ni service partagé. Le témoin direct est publié sous `cx-coeur-c1-dictionary-diagnostic`, compléments `2f88b3df36591b3b8a4275018c7cddec6828a8d1` ; il ne porte aucun correctif de production et reste non joué à ce relevé.

Le relevé de capacité doit rester frais : TSan, G et C1 occupent au plus trois des quatre places ; G garde la priorité. Cette exception n'ouvre ni les passes Rust, ni les modèles/GPU, ni les bases réelles. Un premier rouge se conserve et se diagnostique avant la suite. La bibliothèque autonome de ce test C++ n'est pas la bibliothèque posée.

## Préavis et coupure des hôtes

G déclare les trois suites de déploiement vertes à la source gelée 927, sous 59ac sans moteur ancien : code 36/36, lectures 27/27 et démon 24/24, zéro ignoré. Les essais isolés et contrôles des manifestes aux deux postes sont également verts. Cela valide la préparation, pas encore la pose ni les migrations.

Le préavis d'au moins une minute a été relayé aux douze autres porteurs : aucun appel au 7970 ni nouvelle passe pendant la fenêtre ; 7878 et 7879 restent inchangés. G a engagé la coupure à 11 h 27 CET : sockets et hôtes des deux postes inactifs, MainPID zéro ; arrêt propre du portable en environ vingt secondes. Les instantanés et le retour 72 heures sont en préparation, avant les deux sens sur copies et la pose. Un écart de nom de poste (`LuciePC` contre `luciepc`) a refusé la capture d'identité avant copie ; G conserve ce rouge et paramètre explicitement le rôle, sans attribution à une corruption de base.

L'outil RO d'arbre, corrigé après le `RETURN` manquant, a été rejugé par G sur une fixture : identité moteur 59ac, compte exact, écriture refusée et octets inchangés. Il ne remplace pas les contrôles de migration. Les trois copies de sources du banc restent des candidates tant que l'admission du scanner (.ignore, liens, taille) n'est pas recoupée ; leurs déclarations privées ont `database=null` et ne sont pas exécutables en l'état. Aucun accès d'arbre aux bases vivantes avant le FINI explicite de G.

Le suivi de la reprise est dans [le ticket](../tickets/2026-11-16-reprendre-les-perimetres-apres-lecture-des-diffs-recents.md).
