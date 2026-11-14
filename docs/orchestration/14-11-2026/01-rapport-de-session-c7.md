# Rapport de session c7 — 14 novembre 2026

Suite du [rapport du 13](../13-11-2026/02-rapport-de-session-c7.md), qui court jusqu'à 4 h 35 (la nuit :
1a, la liaison, le flux partagé, l'empreinte, les thèmes, la lib 0c41f9fe6, la perte de Note prouvée).

## 13 h — reprise

- Lucie revient (quota hebdomadaire remis à zéro ; la session s'était arrêtée dans la nuit). Rien n'a bougé
  depuis 4 h 35 : master = 7d7524ff0, courant = 835d5298f aux deux postes, hôte 7970 sous 835d5298f.
  Toutes les sessions dormaient.
- **La pose de 0c41f9fe6** : mon message à G (« Lucie est là », déroulé de la pose) a été **refusé par le
  classifieur du mode auto** (arrêt d'hôtes = action destructrice). Je ne le reformule pas ; Lucie écrit
  le mot à G elle-même, c'est le même mot (G l'a dit cette nuit).
- Lots du matin donnés : cœur → correctif (a) de la perte de Note ; arbre principal → 1b sur copie ;
  lucivy → 4.3.7 (score sous masque) puis la mesure ; archi → thèmes de l'IDE (hôte à deux fils) ;
  recherche → Surveillant (le plafond au mot de Lucie) ; embarquement → la démo sans service, puis le
  vrai jeu après la pose ; ED → relecture de la page 30 finale, puis le plan de 2b-B ; hotfixs → la remise
  orpheline, le relevé du gabarit éphémère. 96 et codeparsers attendent les décisions de Lucie
  (archivage, import_binding) ; tty sans lot, à proposer.
- **archi, cause du rouge des thèmes, plus précise** : `/ide/etat` lit index_states, qui prend le catalogue par `catalog_read::lock_for_read` (attente ≤ READ_WAIT, 1 s) ; la recherche en mode auto (generic_search_nodes.rs:446 et :467) prend le même catalogue par un **try_lock nu** : tenu quelques ms par une lecture d'état, elle conclut « occupé ou illisible » et balaye, et la vue Search ne relance pas. La requête /ide/themes.json ne verrouille rien, elle décale l'ordre des requêtes au chargement — **défaut de master, latent**, rendu déterministe par le témoin des thèmes. Décision : la recherche prend le catalogue par lock_for_read (borné à 1 s), le balayage reste pour une indexation entière et la réponse le dit ; **lot à part devant les thèmes**, ticket + témoin rouge d'abord.
- **ED, relecture de la page 30 finale** : ses deux correctifs sont dans le code ; **un défaut neuf** : la parente déclarée d'une tâche est ignorée si elle n'est pas suivie (la tâche n'ajoute que sa branche, `resoudre` écarte l'estampille dont la parente n'est pas suivie → une tâche ouverte depuis `atelier-commandes` serait réinférée sur l'ancre, son estampille « declaree » écrasée) — correctif demandé à arbre principal : la parente déclarée entre aux suivies avec la tâche, une estampille « declaree » ne s'écrase jamais ; la page 30 en retard (décisions 1 à 4 encore en questions, 1a futur) : section « ce qui est livré » à ajouter. ED écrit le plan de 2b-B (un agent relève les faits du code des trois signaux).
- **Décision de Lucie, 13 h 00 : l'archivage prend la voie 3**, la création de fiches par sujet avec le dédoublonnage (« il crée un nouveau sujet, c'est le but ; l'important c'est que ce soit bien rangé »), avec deux garde-fous : les fiches créées restent « proposées » jusqu'à confirmation et la page de la passe les liste ; un journal donne une fiche par chantier. 96 : garde-fous rouges d'abord, instantané de la mémoire de Lucie avant, première vraie passe sur sa mémoire (432 sections) à luciepc, page des fiches créées.
- **cœur, correctif (a) écrit** (annulation-garde-les-autres-2, 9b0e73c9c, rebasée sur 6b60d77aa) : si `transaction->commit` lève, `rollBackRefusedCommitNoLock` annule et retire la transaction **sous le verrou** puis l'erreur repart ; garde en tête de `rollbackInsert` (nœuds et relations en mémoire) : plus de lignes que startRow + numRows → KU_ASSERT en debug, RuntimeException en Release, la base est condamnée au lieu de tronquer ; mesure du temps d'annulation sous le verrou sous RAG3DB_PROFILE_CHECKPOINT. Passe à luciepc (~1 h 30) : témoin rouge ×3, vert ×5 avec mesure, transaction_test, concurrence known_red, DdlEveryMinute ×10 sous profil.
- **hotfixs, les deux remises de la pile** : stash@{0} « refus: tests rouges + constante » (3 oct., cœur, contenu repris et étendu sur master par 6d0c540f5) ; stash@{1} « marche1-etapeB+correctif » (2 oct., moteur-concurrence, dite obsolète par son rapport). **Les deux peuvent partir sans perte ; décision à Lucie** (pile de son dépôt). Hotfixs passe au relevé de templates/.
- **cœur : RefusedCommitRollback rouge → vert** — rouge ×3 sur cd525c2c7 (20 lignes de B perdues à chaque tour), vert ×5 sur 9b0e73c9c (les 3 cas) ; annulation d'un COMMIT refusé sous le verrou : **médiane 17,6 ms, max 28,9 ms** sur 110 mesures (lots de 60 000 lignes). Suites entières puis DdlEveryMinute ×10 sous profil, verdict vers 14 h 40.
- **Décision de Lucie, 13 h 05 : import_binding, option 1 — on embarque tout**, avec la **provenance distinguée** du nom lié : builtin (bibliothèque standard), gestionnaire de paquets (dépendance), projet (module du dépôt) ; elle croyait que codeparsers le faisait déjà. Lot à codeparsers : dire ce qui existe (projet/non-projet oui par la sonde ; builtin/paquet ?), puis un scope par nom lié avec `provenance ∈ {builtin, dependency, project, inconnue}`, le juge des 415 rejoué, le coût mesuré.
- **Décision finale de Lucie, 13 h 10, import_binding** (après discussion, la première formulation retirée) : **pas de scope par nom lié** (la zone de liaison de master reste, option 4) ; **la provenance se distingue sur la liaison** : builtin / dependency / project / inconnue, jamais devinée. Lot à codeparsers : ce qui existe, puis le champ, rouge d'abord sur trois cas par langage, le juge des 415 rejoué, une ligne dans l'IDE.
- **ED : le plan de 2b-B est sur master** (0d2393fa6, docs/everything-declarative/14-11-2026/01-plan-du-chargeur-durable-des-couches-2b-b.md), arbre principal relit. Composition : `occurrence_roots_for` rend les occurrences visibles d'une génération ; les trois signaux en découlent (dense par le graphe projeté, plein texte par allowed_ids, creux par les offsets) → masque avant le top-k par construction ; à l'ouverture, les versions neuves seules ; sous une vue, aucun demi-instantané, un signal sans masque refuse ; les relations aux deux bouts visibles sont le plus gros morceau ; douze témoins rouges d'abord dont le cruel ; **cinq points pour Lucie (§ 7)** : la forme des occurrences d'une couche dans le gabarit, le refus de lecture sans instantané sous une vue, le parcours des relations restreint, le repli exhaustif du dense, la rétention.
- **hotfixs, relevé de templates/** pendant une lib entière au portable (13 h 07 → 13 h 09, master fb6157803, lib 835d5298f) : inotify via ctypes (inotifywait absent), 55 dossiers et 230 fichiers surveillés, **0 événement**, outil éprouvé (fichier créé et effacé dans la même ms, dossier neuf rattrapé). Le fantôme ne s'est pas montré ; ticket ouvert. Le seul rouge est la L3 connue sous l'ancienne lib. Décision : l'outil entre dans `tools/poste/` avec sa page, confié à arbre principal pour ses libs entières au portable.
- **embarquement a poussé la démo Node.js corrigée** (b7a026f16 : test, fix, ticket fermé ; garde à jour pour de vrai) : la cause était **un réglage du poste** — le binaire lit `RAG3WEAVER_SERVICE_EMBED` avant l'alias `RAG3WEAVER_EMBED_SERVICE`, le portable pose la variable commune (fish, zsh, environment.d), la démo n'effaçait que l'alias ; granite-278m par 7979 répondait ; sans les trois variables, plein texte seul et avertissement visible (pas de repli local). Correctif : la passe « sans service » retire toutes les variables, chaque passe nomme son embarqueur par /sante ou dit « aucun ». Témoin JS rouge puis vert, sans cargo (lot JS). Ticket ouvert : `test/essai.js` hérite des variables du poste (même cause, pas jouée). Poussé sans « à toi » : rappelé (seules les docs pures en sont dispensées) ; ménage par garde_worktree.py.
- **96, les deux garde-fous de la voie 3 (lot archjour)** : un sujet est un chantier, une chose ou une règle, jamais une date (refus d'une fiche neuve nommée par un mois ou un jour, section gardée au L0, témoin rouge puis vert) ; la fiche créée reste `proposed` (déjà le cas, verrou ajouté, vert dès sa pose et dit ainsi) ; **la promotion par citation n'existe pas et n'est pas codée — question ouverte pour Lucie** ; e2e_archiviste 11/11, ticket du journal clos. Rejeu sur 0c41f9fe6 ; « à toi » au vert. **Pour la vraie passe sur la mémoire de Lucie, 96 lui demande directement** (où est la base, quand arrêter l'hôte pour l'instantané) : son mot sur ses données, pas le mien relayé.
- **codeparsers, la provenance, ce qui existe** : projet / non-projet existe à l'analyse (`ImportReference.is_local` par langage ; un non-projet devient `USES_LIBRARY` vers un nœud `Library` sans genre) ; **builtin / dépendance n'existe pas dans le code vivant** (listes dans `src/import_resolution/`, code mort : std/core/alloc, builtins Node, en-têtes standard C/C++, stdlib Go, BCL .NET ; Python sans liste) ; `dependency` demande les manifestes (Cargo.lock, package.json, pyproject, vcpkg.json), lus par rag3weaver dans origin.rs pour le seul nom du paquet englobant ; cas du workspace Rust/npm (une crate du dépôt importée par son nom). **Forme retenue : la provenance sur le nœud Library et sur l'arête liaison, pas sur la zone** (qui mélange). Deux lots : codeparsers (builtin par listes fixes, Python par `sys.stdlib_module_names` figée, project par is_local, inconnue sinon) ; rag3weaver (dependency par les manifestes, workspace → project).

## Vers 13 h 1x — le poids du plein texte vient de ce qu'on indexe

- **lucivy, mesure sur copie (5cd68564e)** : texte indexé **928 Mo** (pas 37), index 4,9× son texte (normal avec positions) ; **10 % des documents portent 91,6 % du texte** (médiane 275 o, p90 20 Ko, p99 133 Ko, max 1,9 Mo : utf8proc, un fichier Google de 470 Ko, des bundles JS minifiés en plusieurs versions) ; 21 % de contenus en doublons exacts (71 024 distincts / 89 886) ; supprimés 12 % seulement (pas la graisse d'avant 4.3.5) ; positions ~37 % de l'index, texte stocké 10 %, dictionnaire 11 %. **Écart à vérifier : 89 894 documents vivants pour 68 585 Scope annoncés.** Leviers : (1) ne pas indexer le généré, l'importé, le minifié (le plus gros) ; (2) positions: false (~37 %) ; (3) comprendre les doublons ; (4) purger ; (5) compression. 4.3.7 : PR ouverte, verte à luciepc, publication au mot de Lucie.
- Lots : **recherche** → page de règles d'exclusion mesurées sur code-lourd et rag3db (hors-git, dépendances, .min.*, générés, taille max), faux positifs cités, part des doublons imbriqués ; **Lucie dit ce qu'on n'indexe pas**. **hotfixs** → sonde de cohérence .fts / graphe (orphelins ?).
- **recherche, le Surveillant** : le témoin disait faux — les sondes du Surveillant se gênaient entre deux appels en vol (deux fils producteurs par défaut) ; correctif dans le témoin (sondes une à une ; un `;` après le `if let` sans lequel la garde du try_lock survivait à la sonde — règle des temporaires de fin de bloc) ; deux témoins rouges d'abord ; l'original 10/10 ; ticket fermé dans le lot ; passe à luciepc depuis 13 h 15 (surveillant-2, 42b233d28, 0c41f9fe6). Puis la page d'exclusion.
- **lucivy** : sans lot hors la 4.3.7 (PR #31, 30/32, deux jobs en cours, publication au mot de Lucie) ; deux réserves au mot de Lucie : trusted publishing de lucivy-wasm (réglage du compte npm, à sa main), l'avertissement `indexed: false`. Lot donné : depuis la copie, les 1 000 plus gros documents par chemin et nature, le texte par dossier, les doublons par chemin — matière de la page de recherche. La copie (4,3 Go reflink) gardée jusqu'à la sonde de hotfixs.
- **lucivy, document par document (5fa98e131)** : les minifiés portent **92,3 % du texte** (856 Mo, 10 677 documents ; le code source 67 Mo) ; **844 Mo de doublons exacts** (2 588 contenus répétés, 18 862 documents de trop, un morceau de 133 Ko 484 fois) ; **mécanisme déduit : un scope reçoit les lignes qu'il couvre** — dans un bundle minifié d'une ligne, chacune des centaines de fonctions reçoit la ligne entière. Deux règles règlent presque tout : ne pas indexer les minifiés ; borner le contenu d'un scope à ses octets. Lucivy_Scope n'a pas de chemin (jointure par le graphe). **ED prend le correctif « octets, pas lignes »** (Rust, rouge d'abord, réindexation des bases existantes à dire) ; recherche garde la page des règles d'exclusion pour Lucie.

## Vers 13 h 1x — la parente déclarée entre aux suivies ; la pose attend toujours le mot de Lucie

- **arbre principal, parente-suivie** (rouge 63b31b1eb, vert 1cf81534a) : la parente d'une tâche entre aux suivies à l'ouverture (sauf ancre ou déjà suivie), une déclaration écartée ne s'estampille plus par l'inférence, le rapport des couches le dit ; couches 11/11, tâches 11/11. **« À toi »**. La parente reste suivie à la fermeture de la tâche (juste : la rétention décidera). Puis la page 30, puis 1b (la copie peut se prendre dès maintenant).
- **La pose n'a pas commencé** : mon message à G est refusé par le classifieur, Lucie doit écrire le mot elle-même ; rappelé plusieurs fois.
- **hotfixs, deux lots prêts, « à toi »** (docs et outils) : releve-templates-poste (releve-templates.py, lib-entiere-releve.sh, page d'usage, éprouvé sur une lib entière réelle) ; coherence-fts-depot — sur une copie fidèle de avant-835d5298f : Lucivy_Scope 88 471 documents vivants + 11 879 supprimés dans leurs segments ; graphe 79 662 Scope tous avec leur document ; **8 809 documents vivants sans Scope : des orphelins** (Scope effacés du graphe, jamais retirés de l'index). Ticket produit demandé (quel chemin d'effacement oublie le .fts ; les recherches rendent-elles ces orphelins ?). Les 89 894 / 68 585 d'instants différents ne se comparent pas.
- hotfixs : le lot de cohérence porte maintenant le **ticket « un Scope effacé du graphe garde son document plein texte »** (8 809 orphelins ; risque : un _node_id qui ne mène à rien ou à une ligne réutilisée ; pistes : DETACH DELETE de fin d'index, réindexation qui déplace un Scope, nettoyage des absents) → **Pour : recherche**, après sa page.
- **96 : rejeu d'archjour rouge** sur un seul test hors lot, `physical_index::tests::native_process_death_recovers_same_program_and_lost_ack_once` (code de sortie 91 attendu de l'enfant, None reçu), pas dans instables.txt ; tout le reste vert (lib 2363, F8, navigateur 43/43, archiviste 11/11). Isolé seul ×10 ; vert seul → environnement nommé + ticket pour arbre principal (fixture sous /tmp à déplacer) ; rouge seul → défaut de master.
- **archi** : rouge posé pour `recherche-lit-le-catalogue` (témoin `a_short_hold_of_the_catalog_does_not_turn_a_search_into_a_scan` : catalogue libre → index ; tenu 100 ms → « illisible » ; tenu 1,5 s → la raison dite) ; deux jeux vides écartés ; correctif appliqué ; passe à lancer sur 0c41f9fe6 sans attendre la pose.

## Vers 13 h 2x — la cause des 844 Mo est dans codeparsers : les octets d'un scope dérivent de ses lignes

- **ED a confirmé dans le code** : rag3weaver découpe déjà par octets (`own_texts`, code.rs:4380-4440) mais **codeparsers dérive `scope_start_byte`/`scope_end_byte` des lignes** (`finalize`, parser_worker.rs:177-186), voulu et documenté (types.rs:393-398 : `File` source de vérité des positions, lecture par tranche, couverture). Dans un bundle d'une ligne, chaque fonction reçoit la ligne entière. ScopeInfo n'a ni colonnes ni octets du nœud. **Décision : lot en deux dépôts, porté par ED** — codeparsers : deux champs optionnels `node_start_byte`/`node_end_byte` (empan du nœud tree-sitter, ~48 sites, le contrat à la ligne inchangé : une précision ajoutée, pas une rupture) ; rag3weaver : `own_texts` prend l'empan du nœud s'il existe. Effet sur les bases : les adresses de contenu changent (presque tout pour les minifiés, rarement pour le code ordinaire), règle (A) : à migrer, jamais en silence. Coordination avec la session codeparsers (provenance en cours sur le même dépôt) : premier vert atterrit, second rebase, master linéaire.
- 96 : le test du raccord physique est instable sous charge (3/3 seul) ; rejeu relancé, ticket pour arbre principal, entrée dans instables.txt avec sa raison.

## Vers 13 h 2x — ce qu'on n'indexe pas : la page de recherche ; le reste de la file

- **hotfixs a poussé** ses deux lots (0118fa366 cohérence du .fts avec le ticket des orphelins Pour : recherche ; b05aedb15 le relevé de templates dans tools/poste/).
- **recherche, page « ce qu'on n'indexe pas » (0e5188bd3), pour Lucie** : l'arbre admis pèse **67 Mo** (63 sur code-lourd) contre 928 Mo indexés ; **presque tout l'écart vient de `templates/ide/app/app.js`**, le bundle minifié de l'IDE, suivi par git, une ligne de 133 294 o, **52 versions commitées depuis le 25 octobre** — le morceau répété 484 fois. Règles mesurées : (1) minifié par le contenu (une ligne > 5 000 o) retire app.js, c'est celle qui compte ; (2) dossiers d'import : 15 Mo, du code que le moteur lie, à Lucie ; (3) sous-modules et (4) hors-git retireraient nos propres sources (codeparsers, lucivy) : à ne pas retenir ; (5) taille max de code à 1 Mio (pas au p99 : catalog.rs fait 671 Ko à la main), ne reste qu'utf8proc_data.h. Les copies de fichiers ne pèsent que 0,2 à 0,3 % ; les doublons sont les scopes de la ligne de bundle (correctif d'ED).
- **codeparsers, provenance codée et verte** dans codeparsers (`provenance-imports`, c7c4b2b : listes réveillées, Go par la règle de la chaîne d'outils, Python 3.14.9 figé 297 noms, project/inconnue, aucune devinette ; 291/0) et rag3db (c596560e5 : pointeur, `Library.provenance`, propriété d'arête `provenances` triée distincte sur l'arête fondue par `merge_by_target`, témoin e2e étendu). Lib à épingler : 0c41f9fe6 (dit).
- **archi** : correctif de la recherche vert sur ses tests (37/0), ticket fermé, a66955274 ; passe complète à luciepc sur 0c41f9fe6 depuis 13 h 22.
- 96 : le test du crash physique seul **13/13** sur le même arbre et la même lib → rouge d'environnement sous la charge ; fixture gardée (~/.cache/rag3weaver-build/fixtures-gardees/physical-crash-2026-11-14/ à luciepc), ticket Pour : arbre principal, une ligne dans instables.txt avec sa raison ; ce qui chargeait le poste à 13 h 11 inconnu (pas inventé). Rejeu relancé, push au vert (« à toi » tient).
- **recherche, surveillant-2 vert** sauf un rouge étranger : `a_progress_card_reaches_the_page_and_never_the_model` (délai ; la lib a mis 112 s au lieu de 50 sous une **charge de 26 à 31 à luciepc** — plusieurs passes à la fois), rejoué vert seul 3/3, noté au ticket d'ED des tests qui rougissent sous charge. Push avec le rouge nommé si sa garde l'admet, sinon relance dans un creux. **Orphelins du plein texte, cause probable (hotfixs)** : `DeleteRecordNode` ne trouve l'offset du document que dans un cache uuid → offset qui naît vide à chaque ouverture ; après un redémarrage, effacer un Scope ancien laisse son document. Correctif demandé : repli par requête (uuid ou _node_id) quand le cache ne sait pas, journal de chaque effacement sans document ; purge des 8 809 en lot à part sur copie.
- **Deux rouges d'environnement en une heure à luciepc** (crash physique chez 96, délai chez recherche) : la charge des passes parallèles ; à surveiller, les lourds-max de luciepc (4) sont peut-être trop pour des lib entières à la fois.

## Vers 13 h 2x — Lucie tranche : la pose, et le reste

- **Lucie, 13 h 25 : oui à la pose** (mot transmis à G : « Lucie est là », déroulé avec préavis aux passes en cours à luciepc) **et oui au reste** : règles d'exclusion 1 (minifié par le contenu) et 5 (1 Mio), pas la 2 ; le plafond d'instantané retiré avec le recommencement en filet ; le correctif (a) de la perte de Note confirmé ; la publication de la 4.3.7 dès CI verte ; les deux remises git du 2 et 3 octobre peuvent partir (hotfixs, patchs gardés) ; le worktree de docs de la passation Codex peut partir (tty, par la garde). Le tunnel 7981 de sa config systemd reste à sa main ; le trusted publishing npm aussi (lucivy écrit les gestes).
- Deux fiches de la nuit : la fusion de deux lignes d'index mémoire a laissé l'index à 200.
- **Erreur d'horloge de c7, corrigée** : mes messages de la matinée disaient « 14 h », « 15 h » alors qu'il était 13 h 00 à 13 h 30 (les en-têtes de ce rapport, lus par `date`, sont justes ; les heures en prose ont été rectifiées). G avait lu « 15 h » comme l'heure de la pose : repris, la pose se fait dès que les lib entières en cours finissent.
- **lucivy retire son alerte npm** : lucivy-wasm publie déjà par trusted publishing (NODE_AUTH_TOKEN vide à l'étape de publication, `--provenance`, « Signed provenance statement » sur chaque paquet ; l'avertissement « npm tokens that bypass 2FA are being restricted » est un message général). **Rien à faire pour Lucie** ; en option, sur npmjs.com par paquet : Settings → Publishing access → « Require two-factor authentication and disallow tokens ». PR #31 fusionnée, tag v4.3.7 à la CI verte de main.
- tty : cx-docs-tty-features effacé par la garde (administration archivée, cherry à 0, sans --force), preuves de recherche intactes. Sans lot : knowledge dump de la semaine en docs ; **tmux et mosh à installer au mot de Lucie** (demandé).
- **hotfixs : les deux remises ne sont pas effacées** — `git stash drop` refusé par le classifieur de sa session (action destructive) ; refus non contourné, pas confié à une autre session (ce serait un blanchiment). Patchs archivés (~/.cache/rag3weaver-build/archives/remises-2-3-octobre/), chaque hunk prouvé sur master ou dépassé. **La commande d'effacement est à la main de Lucie** (par index relu à l'instant) ; elle est dans son message et ci-dessous.
- **archi : le rouge des thèmes tombe de lui-même** une fois les thèmes rebasés sur le correctif de la recherche (ide-themes-route-3, 0f624a1a5) — le témoin des vues joué seul passe vert 1/0, rouge à chaque jeu avant. La passe des thèmes au portable sur 0c41f9fe6, derrière celle de la recherche à luciepc ; G a son « va » d'archi pour la pose (ses passes épinglent leur lib, ne parlent pas à 7970).
- tty : knowledge dump et « Reprendre » à jour (185cc5190) ; sans lot, attend le mot de Lucie pour tmux et mosh.

## Vers 13 h 3x — Lucie dit oui aux cinq points du chargeur de couches

- **Lucie : oui aux cinq points de la § 7 du plan de 2b-B** — entité à part pour les occurrences d'une couche ; refus d'une lecture sans instantané sous une vue ; relations restreintes aux occurrences visibles ; repli exhaustif du dense à mesurer avant toute règle ; rétention telle que posée plus le bail sur une génération épinglée. Son mot : « au pire ce sont des décisions qui peuvent se retravailler après pour proposer amélioration ou flexibilité ; partons dessus jusqu'à ce qu'on se rende compte d'un truc emmerdant pour l'user ». ED code dans l'ordre de sa § 8, après le lot des octets.
- **Les deux remises sont effacées**, sur l'autorisation explicite de Lucie donnée dans la session de hotfixs (le refus du classifieur levé par elle, pas contourné) : e0c621ebb et 041f94b98, par index relu ; pile vide ; patchs archivés.
- **archi : recherche-lit-le-catalogue verte en passe complète** (a66955274 : F8, script, lib 2365/0/70, e2e_hooks, navigateur 43/43) ; file : arbre principal (parente-suivie, en dernier pas) → archi → 96 (au vert) → thèmes de l'IDE.

## Vers 13 h 3x — archjour sur master ; le test du crash physique récidive sous charge

- **96 a poussé archjour** : master = 7961fae7f (les deux garde-fous de la voie 3 ; rejeu sur 0c41f9fe6 : F8, navigateur 43/43, archiviste 11/11, cinq suites ; au push le crate n'avait pas bougé). **`native_process_death_recovers_same_program_and_lost_ack_once` rougit à chaque lib entière sous charge** (0/2) et passe seul 14/14 : ce n'est plus un hasard du poste ; ticket Pour : arbre principal, après 1b. 96 attend le mot direct de Lucie pour la vraie passe (chemin de la base, créneau d'arrêt de l'hôte) ; knowledge dump de la semaine d'ici là.
- Leçon pour moi : deux « à toi » ouverts en même temps (arbre principal parente-suivie, 96 au vert) → 96 a poussé le premier et arbre principal rebase ; pas de dégât, mais **un seul « à toi » ouvert à la fois**.
- **Lucie : « est-ce qu'on s'éloigne du tout-déclaratif ? »** Bilan des lots du jour : resté déclaratif (garde-fous de l'archiviste en gabarits, branches suivies déclarées, provenance en champ de donnée, empans en champs, chargeur par un point de composition, délais nommés). **Deux glissements redressés** : les règles d'exclusion de l'index deviennent des champs du manifeste du backend avec des défauts (dit à recherche), et les listes de builtins par langage ont pour forme cible un fichier de données par langage livré avec le crate (dit à codeparsers, sans bloquer le lot). La mécanique de cohérence du moteur (verrous, instantanés) n'est pas du domaine déclaratif ; sa règle est de dire ce qu'elle fait dans la réponse.
- recherche : ordre reçu (Surveillant dans un creux, sa garde n'admet pas de rouge nommé ; plafond ; exclusions 1 et 5 ; orphelins avec le témoin rouge d35328f82 en file) ; « va » à G.

## Vers 13 h 3x — la lib 0c41f9fe6 est posée à luciepc

- **G, 13 h 37 min 49 s, luciepc posé** après le « va » de toutes les sessions : lib 0c41f9fe6, binaire 012be4a58 (md5 547a8f56, check-lib 3bc77ad5), ancien binaire gardé ; instantané avant-0c41f9fe6 de code-principal et code-lourd avec RETOUR.sh ; réveil vert à 13 h 37 min 55 s (0c41f9fe6 dans les maps et l'env, 7878 et 7879 à 4-5 ms, bases rouvertes, 0 erreur) ; `lib-des-passes` de luciepc → 0c41f9fe6 (ancien gardé) ; **courant = 0c41f9fe6 aux deux postes** à 13 h 37 min 59 s, readlink et md5 relus (9bd53729 / 0fc4f8f0). Coupure du portable à 13 h 39, Lucie prévenue.
- codeparsers : les listes de builtins en fichiers de données ne sont pas trois lignes (un fichier lu à l'exécution demande une option du résolveur passée par rag3weaver, une règle quand il manque, et le retrait ou le rebranchement des résolveurs morts) → ticket 99770093d, pas suivant ; forme cible écrite (`data/builtins/<langage>.txt`, version et source en tête, Go en règle, témoin « un nom ajouté devient builtin sans recompiler »). Passe en cours sur 0c41f9fe6.

## Vers 13 h 4x — le portable posé ; la provenance verte ; 96 pose deux questions à Lucie

- **Le portable est posé** : l'hôte 7970 relancé (pid 3236455 → 2943666), /sante dit rag3db-build-0c41f9fe6. « Posée » complet de G attendu.
- **codeparsers, provenance verte** (rag3db c596560e5, codeparsers c7c4b2b ; lib 2364/0, e2e_code 53/53, postgres 36/36, usages 19/19). Répartition des 415 : inconnue 241 (les paquets npm, `dependency` au second lot), builtin 77, builtin+inconnue 31, project 1, plus de liaison 65 (dont les 4 chaînes ôtées). Aucune vue ne montre encore la provenance : lot IDE pour archi plus tard. File : arbre principal → archi → codeparsers.
- **96** : rapport du 14 et knowledge dump de la semaine (5f2179f05) ; sans lot. **Deux questions pour Lucie** : la base de mémoire que sert son hôte paraît être `~/.local/share/rag3weaver/memoire-longue.rag3db` (non lue, à confirmer par elle) ; une passe « à luciepc » sur une base du portable demande une copie puis un retour, ou de jouer au portable — à trancher avec elle et G.

## Vers 13 h 4x — « posée » aux deux postes ; parente-suivie sur master ; la règle de Lucie sur les redémarrages

- **G, 13 h 42 : posée aux deux postes** — lib 0c41f9fe6, binaire 012be4a58 (547a8f56, check-lib 3bc77ad5) ; portable coupé de 13 h 39 min 38 s à 13 h 40 min 06 s (MainPID 2943666), instantané avant-0c41f9fe6 de memoire-longue et code-rag3db-depot, cinq backends rouverts, 0 erreur ; luciepc 13 h 37 min 49 s ; témoin par le chemin de l'hôte vert (/sante 0c41f9fe6, search_code « getPrimaryKeyIndexes » ok par 7970, atelier 21 outils, /ide/ 431 éléments, 0 erreur de console) ; md5 relus aux deux postes ; `lib-des-passes` de luciepc = 0c41f9fe6, aucun au portable ; gardées : e93710f36, 835d5298f, 207701bb2, 29a7bc52a ; page des libs docs/optimiseur/14-11-2026/01-pose-de-0c41f9fe6.md (4e868f8ab). Disque du portable : 224 Gio (262 hier soir), G surveille.
- **arbre principal a poussé parente-suivie** : master = 912ea2084 (rouge face54de8, vert 912ea2084) ; rejeu court vert sur la base avec archjour ; garde à jour. Retard dit : un target recréé sans provenance refusé par la garde (target neuf, bâti complet ; `target-scope-version` à ranger), et POSTE_MEM_MAX=32G oublié. Page 30 à jour (ba7dcd122). Puis 1b (copie par reflink depuis l'instantané avant-835d5298f).
- **« À toi » à archi** (recherche-lit-le-catalogue, puis les thèmes dans le même tour), codeparsers après.
- **Règle de Lucie, 13 h 45** : « plus d'inquiétude de faire redémarrer mon IDE ni de relancer un service, je viens voir très rarement, quand tu me dis explicitement de tester ; faut pas que ça empêche d'avancer ». En mémoire : un redémarrage se fait quand le lot le demande, annoncé aux sessions ; on ne lui demande un créneau que pour ses données. Transmise à embarquement avec le « va » du vrai jeu.

## Vers 13 h 4x — la première vraie passe d'archivage sur la mémoire de Lucie

- **Lucie à 96, directement** : « oui d'accord pour tout, tant que tu t'accordes avec orchestration et les sessions pour pas vous emmêler les pinceaux ». Vérifié en lecture seule par 96 : l'hôte 7970 sert la mémoire par `~/.config/rag3weaver/memoire-longue/backends/memory/backend.json` (database `~/.local/share/rag3weaver/memoire-longue.rag3db`, files_root `~/memoire-longue`) ; son gabarit du 25 octobre n'a pas l'outil archive mais ses entités, relations et schémas sont identiques à master : une base écrite par la passe reste lisible par son hôte. **Plan validé** : (0) binaire de passe rebâti à luciepc sur master, lib 0c41f9fe6 ; (1) G ferme le seul backend memoire-longue du portable (POST /backend/close), l'index de code reste servi ; (2) instantané complet vers `instantanes/avant-archivage-14-nov/` ; (3) la passe à luciepc (52 documents, 432 sections, allow_create, gemini-3.8-flash, ~70 min) ; (4) retour, G rouvre, recall_fiches et select_apports ; (5) la page (d). Échec → l'instantané se remet. Ajouts c7 : aucune écriture de fiche par aucune session pendant la fenêtre (annoncée) ; l'instantané se garde jusqu'à ce que Lucie ait survolé la page (d).
- **ED, plan de 2b-B à jour (5e7098cdf)** : § 7 bis les décisions de Lucie avec sa phrase, § 7 ter la relecture d'arbre principal, pas 2 coupé en 2a′/2b′. **Confirmé par c7** : `Generation` est l'entité à part (ensemble d'identifiants : occurrences, retraits, parente épinglée) ; les occurrences restent des lignes `Scope` à `_uuid` = hash(clé stable, génération), clé stable en colonne — pas de double schéma de code. Lot des octets : rouge vu dans codeparsers (deux témoins rouges pour la bonne raison, le troisième « trous » vert), correctif aux 44 sites nés d'un nœud, trois trous à None ; suite entière en cours, puis rag3weaver.

## Vers 13 h 4x — 1b attend la forme v2 du lot des octets

- **arbre principal** : le lot des octets d'ED change ce que rend `code-scope-v1` (le `content` dans l'empan du nœud) → ED monte la forme à **`code-scope-v2/content`** ; 1b vise la forme du binaire en paramètre et ne promet plus « zéro appel à l'embarqueur » : **seules les versions au texte changé s'embarquent**. Ordre validé : mécanique de 1b sur fixtures maintenant → le lot d'ED atterrit → 1b vers v2 sur la copie réelle (prise : `~/.cache/rag3weaver-build/1b-copie/2026-11-14-depuis-avant-835d5298f/`, reflink de l'instantané de G). Pas de v1 d'abord. Réserve : pas de `git show` (une ancre indexe l'arbre vivant) ; 1b réanalyse le texte que `File` porte, sinon lit l'arbre du poste et le dit.

## Vers 13 h 4x — la fenêtre d'archivage est ouverte ; les données de Lucie sont des données de test

- **G a fermé le backend memoire-longue du portable** à 13 h 45 min 38 s (closed: true, .wal absorbé, fuser vide, relevé de 3 643 fichiers gardé) ; l'hôte sert toujours l'index de code ; « va » à 96. Correction de 96 (éprouvée par G le 25 oct.) : un appel rouvre un backend fermé, l'écriture passe — l'annonce est le seul verrou ; garde-fou au retour : diff de ~/memoire-longue et des comptes contre l'instantané.
- **Règle de Lucie, 13 h 50** : « même mes données, osef : tout ce qui concerne rag3db, ce sont des données de test au profit du projet, sauf le cas où j'ai vraiment besoin d'une version sur le long terme pour autre chose ». En mémoire : instantané puis on fait, sans lui demander son mot ; un écart au retour s'écrit dans la page, il n'arrête pas la passe. Dit à 96. Vaut pour 1b sur ses bases aussi.
- **arbre principal : `File` ne garde pas le texte** (path, absolute_path, content_hash, size_bytes, lines_of_code, revision, cursor) ; seul `Scope.content` en porte, à l'empan des lignes. 1b lira l'arbre du poste au `absolute_path` : hash égal au `content_hash` → analyse v2 sur ce texte, rattachement par `Scope.key`, une version au texte inchangé garde morceaux et vecteurs ; hash différent ou fichier disparu → **dette d'index nommée dans le rapport**, rattrapée par la synchro ordinaire à la réouverture (seul ce fichier s'embarque). Master a l'inspection, le plan, la qualification et l'aperçu du transfert, pas l'exécution ni la publication : c'est ce que 1b bâtit, sur fixtures d'abord.

## Vers 13 h 4x — la passe réelle d'archivage tourne

- **96 : fenêtre ouverte**, passe partie à 13 h 47 à luciepc, fin vers 14 h 55. Instantané au portable `~/.local/share/rag3weaver/instantanes/avant-archivage-14-nov/` (13 h 46, backend fermé) : 113 Mo, 3 713 fichiers, 19 fiches ; base, .extensions, .fts, .readers, .checkpoints et ~/memoire-longue ; ni .wal ni .ecrivain ; sha256 à côté, chaque pièce comparée à l'original ; gardé après le succès. La passe sur une copie à luciepc (sommes vérifiées), binaire de master 91c9f43d0, lib 0c41f9fe6, poste lourd, GPU_DUTY 70, budget 4096, 32G ; deux premiers documents 7/7 et 9/9. Un raté dit : un premier lancement parti sans la base (commande distante tombée sur fish), tué par ses pids à la seconde, base vide effacée dans son dossier, rien au portable.
- Lucie demande ce que sont tmux et mosh : expliqué (multiplexeur qui traduit tout ce qui passe ; shell résilient qui ne laisse pas tout passer) ; recommandation tmux oui, mosh si elle s'en sert ; installation à son sudo sur les deux postes.

## Vers 13 h 5x — tmux et mosh installés ; la règle « rien à installer en plus »

- Lucie a installé tmux et mosh sur les deux postes (après un `-Syu` des deux côtés, miroirs trop datés ; pas de redémarrage pour l'instant, à voir si instabilité). tty mesure dessous. **Règle produit de Lucie** : « les gens n'installeront rien en plus de rag3weaver » → le terminal se tient dans ce que l'utilisateur a et se dégrade en le disant ; la livraison devra porter sa lib (vision mise à jour, a35d75a81).
- codeparsers : en attendant son tour, commence le second lot de la provenance (dependency par les manifestes, workspace → project, côté origin.rs).
- Précaution dite à Lucie pour un redémarrage : luciepc après le « fenêtre fermée » de 96 ; le portable termine toutes les sessions, cinq minutes de préavis pour pousser le vert et écrire les points de reprise.

## Vers 13 h 5x — l'embarqueur à la demande joué pour de vrai par l'hôte

- **embarquement : vrai jeu vert** (docs 5286273ef, ticket « un fournisseur à la demande ouvert par l'hôte ne s'évince jamais » fermé). Hôte 7970 de luciepc (012be4a58 / 0c41f9fe6), index de mesure sur une copie de codeparsers/src (79 fichiers, 2 194 morceaux), fournisseur granite-107m 384 dim. sur 7890, card2, `a_demande { inactivite 60 s, rendre le bail 30 s }`. Déroulé : ouverture et describe sans rien lancer ; index 1 → activation pendant l'appel, identité vérifiée, 9,2 s ; bail rendu +29,5 s, éviction +60,5 s (card2 revenue à 20 395 Mio) ; index 2 → réactivation 4,0 s, 6,2 s ; bail rendu, éviction ; fermeture propre. Mémoire : card2 20 395 → 26 087 Mio au plus fort (5,7 Gio le fournisseur), card0 88 Mio constant. Un premier essai cassé sur `wait_output` (dit au ticket). Ménage par garde_worktree.py.
- essai.js : ticket confirmé (la variable du poste) ; **un second écart** : `search_code` avant tout index ne balaie plus (`adaptive_search_decision` ne balaie que si `has_file_source`) alors qu'alpha.1 balayait → **ticket pour recherche**, l'épreuve coupée en deux assertions.

## Vers 13 h 5x — urgence : openssl 4.1.0 casse tout bâti neuf

- **archi** : la mise à jour système de 13 h 51 (le `-Syu` de Lucie, les deux postes) a passé openssl de 3.6.5 à **4.1.0** et posé `openssl-3` 3.5.9 à côté ; **tout bâti neuf de `openssl-sys` 0.9.111 échoue** (« only compatible with OpenSSL 1.0.2 through 1.1.1, or 3 »). Son rejeu court est rouge à check_features pour cette seule raison ; la passe des thèmes l'avait passé à 13 h 46. Les targets déjà bâtis tiennent ; **tout target neuf ou cargo clean est touché**.
- Décision : lot urgent à archi — (1) un openssl-sys plus récent s'il connaît OpenSSL 4 (bump de Cargo.lock, lot Rust) ; sinon (2) `exporter_la_lib` exporte PKG_CONFIG_PATH/OPENSSL_DIR vers openssl-3 quand l'openssl du système est ≥ 4, en le disant au journal, les deux postes ; témoin : un target neuf compile ; ticket. Annonce aux sessions : pas de target neuf ni de cargo clean avant le mot d'archi. Pour la livraison : la forme « vendored » à évaluer ensuite.
- **hotfixs, sonde des quatre bases (de9d70314, « à toi »)** : aucune ligne du graphe sans document ; documents sans ligne : dépôt Scope 8 809 + **File 124 (neuf)**, code-lourd Scope 103, code-principal Scope 102, memoire-longue 0 ; Symbol et Library 0 partout ; les 103 et 102 égalent les places vides (Scope de fichiers disparus effacés par le DETACH DELETE de fin d'index — le chemin reproduit par recherche). **Portée de la purge : 8 933 / 103 / 102 / 0** (copies du 13).
- **lucivy 4.3.7 publiée** au mot de Lucie (tag sur main 9a09170 ; crates.io, npm 7 paquets, PyPI). **La montée de rag3weaver en 4.3.7 est à lucivy**, après le mot d'archi sur openssl.
- **arbre principal** confirme l'alerte openssl (contournement par variables : `OPENSSL_INCLUDE_DIR=/usr/include/openssl-3 OPENSSL_LIB_DIR=/usr/lib/openssl-3` ou `PKG_CONFIG_PATH=/usr/lib/openssl-3/pkgconfig`) ; luciepc touché aussi ; G vérifie par `ldd` que le binaire posé et les libs trouvent encore libssl.so.3.
- hotfixs : orphelins-quatre-bases sur master (c3b112324). En attendant la copie migrée : inventaire de ce que les passes laissent dans /tmp sur les deux postes (par suite responsable, âge, Gio), au ticket de recherche du 18 oct., rien d'effacé.
- recherche confirme openssl (target-97p neuf refusé ; `PKG_CONFIG_PATH=/usr/lib/openssl-3/pkgconfig` à éprouver). Décision tenue : le contournement vit dans `exporter_la_lib` seul (pas de `.cargo/config` dans l'arbre : la CI et un utilisateur n'ont pas openssl-3), archi porte le lot, les autres lui disent si l'env tient.
- **G, 14 h 01 : openssl ne touche rien de posé** — `ldd` sur le binaire, courant/librag3db.so, libvector, check-lib, rag3weaver-embeddings : aucune ligne ssl/crypto, 0 « not found » ; les processus vivants n'ont aucun mappage libssl/libcrypto ; **le TLS du Rust passe par rustls**. Un redémarrage d'hôte ne casse rien, aucune pose à refaire. Donc `openssl-sys` n'est qu'une dépendance de bâti tirée sans usage : archi cherche qui la tire (`cargo tree -i`) ; si c'est une feature par défaut, la retirer (rustls-tls) est la vraie correction, valable pour la CI et les utilisateurs.

## Vers 14 h 0x — la passe du correctif (a) ; la montée 4.3.7 prête

- **cœur, passe du correctif (a) sur 9b0e73c9c** : transaction_test vert (376) ; concurrence known_red **rouge sur un seul cas**, `SingleWriterCrash.DeathDuringDeletesAndVectorSearchKeepsTheIndexExact` (une manche tuée rouvre « without the vector extension » — mot pour mot le rouge du 7 nov., ticket du 10 oct., 1/6 sous charge, 0/36 seul, retiré de probabilistic le 8 nov.) ; le correctif ne passe pas par là (SIGKILL, aucun « Rollback refused » au journal) ; la charge du moment : la pose à 13 h 42. DdlEveryMinute sous profil 2/10, **0 counts-match-commits**, 678 et 155 refus du pool, 362 et 91 annulations sous le verrou. Plan validé : finir DdlEveryMinute, le cas ×36 avec et sans le correctif, le juge rejoué ; rouge aussi sans → indépendant, ticket de hotfixs, « à toi » au juge vert.
- **lucivy** : montée en 4.3.7 prête sans bâtir (témoin `une_recherche_filtree_classe_comme_sa_vue` 9bb44df45 à jouer rouge sous 4.3.5 ; Cargo.toml et Cargo.lock a0665f6e2) ; une ligne de nouveautés (4.3.5 et 4.3.7) ; attend le mot d'archi sur openssl.
- Lucie : « ils ont mis à jour mais pas partout ? on va se débrouiller ? » — non, c'est openssl-sys qui est en retard sur OpenSSL 4, CachyOS a laissé la 3 à côté ; oui : rien de posé ne lie openssl (rustls), la dépendance de bâti se retire ou se contourne.

## Vers 14 h 0x — l'inventaire de /tmp

- **hotfixs, residus-tmp-14-nov (b45ede783, « à toi »)** : portable (tmpfs 61 Gio, 16 pris) 14,9 Gio dans /tmp dont 1,9 de plus de 24 h — l'essentiel n'est pas à nos tests : dossiers des sessions Claude Code 7,0 Gio (sairen 3,0, rag3db 2,0), cache jest de Sairen 5,6 Gio, tmp Python de Sairen 1,0 Gio **dont 1 025 dossiers `secrets/` avec des clés en clair** (à Lucie, hors rag3db, aucun contenu cité) ; nos tests ~1,2 Gio (profils Chrome 327 Mio, tests .tmp 455 Mio, cibles de test et témoins 167 Mio, url_fetcher 253 Mio, rag3weaver_cache 129 Mio). luciepc (/tmp sur btrfs) 6,3 Gio dont 3,2 de plus de 24 h : profils Chrome 2,9 Gio, Chromium 0,55, **lucivy 1,7 Gio** (test_rag3db_clone du 7 nov. 574 Mio, 1,17 Gio du jour sous des noms fixes), tests .tmp 478 Mio, rag3weaver-bulk 283 Mio, rag3weaver_cache 101 Mio. Suites qui récidivent : navigateur.rs (profils Chrome), daemon_host et e2e_equipier, commande.rs, claude_code_hook.rs, builtin_templates.rs, e2e_code ; hors rag3weaver : lucivy_core/tests, Sairen. **Pour le mot de Lucie** : les orphelins de plus de 24 h (1,9 Gio portable, 3,2 Gio luciepc). Lucivy prévenue (nettoyage + TempDir dans ses tests).
- hotfixs : residus-tmp-14-nov sur master (47f1493d4) ; la version finale ne donne que le chemin parent, le compte et la nature des dossiers `secrets/`. **L'historique garde un nom** : le premier commit de la branche (b767aa82f, sur master) citait le nom d'une variable de clé et le nom du service — aucune valeur ; l'effacer demanderait un push en force sur master, à Lucie seule. Porté à Lucie.

## Vers 14 h 0x — Lucie dit d'effacer /tmp

- **Lucie : « oui on supprime tout ça »** — les orphelins de plus de 24 h des deux postes et les dossiers tmp Python de Sairen (secrets/ compris) : à hotfixs, après fuser, jamais `claude-1000/` (sessions vivantes) ni `jest_rs/` (non nommé) ; les trois entrées lucivy du worktree Codex aussi. Lucivy a déjà rendu 1,8 Gio de ses propres restes à luciepc ; elle commence TempDir et le témoin « rien sous /tmp » dans lucivy_core/tests en attendant le mot d'archi sur openssl.
- openssl expliqué à Lucie (la caisse de pont en retard sur OpenSSL 4, la 3 gardée à côté ; nos binaires n'en dépendent pas ; retirer le sac vide si archi le trouve, sinon pointer vers la 3).

## Vers 14 h 0x — le lot des octets d'un scope est vert, « à toi »

- **ED** : codeparsers `empan-du-noeud` (112e353 témoin rouge, b9029dc correctif : `node_start_byte`/`node_end_byte` aux 44 créations depuis un nœud, None pour les trois trous et le texte brut ; 291/0) ; rag3weaver `scope-borne-a-ses-octets` (témoin sur `bundle.js` — un `.min.js` est écarté par son nom ; rouge « function a(){return 1}function b(){return 2} » puis vert ; `own_texts` borné à l'empan du nœud ; page et nouveauté) ; passe sur 0c41f9fe6 : lib 2367/0, F8 8/8, e2e_code 36/36, code_sync 17/17, scope_versions 8/8, postgres 36/36 ; Cargo.lock inchangé. **Pas de code-scope-v2** : la recette ne change pas, l'adresse suit le texte, 1b migre le `content` que la base a. **« À toi »** : codeparsers master en avance rapide par ssh (identité gmail, jamais gh), puis rag3db avec le pointeur ; archi passe après, codeparsers rebase provenance-imports.
- **Le compte `gh` actif du portable est le pro (`luciedefraiteur-sairen`)**, L-Defraiteur inactif : aucune session ne l'emploie ni ne le bascule ; à Lucie.
- 96, 14 h 10 : passe réelle à 24 documents sur 52 en 22,4 min, 0 erreur, 63 fiches au total (44 créées, 19 au départ) ; fin vers 15 h, retour et « rendu » à G vers 15 h 05. (Mon horloge disait « 15 h 15 » à Lucie : il est 14 h 10 ; les en-têtes du rapport sont justes.)

## Vers 14 h 1x — openssl-sys : le sac est vide

- **archi** : `openssl-sys` est tiré par `native-tls`, sans usage — `hf-hub` (features par défaut alors qu'on n'appelle que `api::sync`) et le `reqwest` des dev-dependencies (default-tls). Les deux passent à rustls : Cargo.lock perd openssl-sys, openssl, native-tls et 9 autres paquets, aucune version ne monte. Rouge joué (témoin sur le lock), correctif, passe complète sur target neuf **sans aucune variable OPENSSL**. **Ce lot passe devant toute la file**, « à toi » au vert ; nouveauté « rag3weaver ne dépend plus d'openssl au bâti ».
- **ED a poussé le lot des octets** : codeparsers master b5c91e1..b9029dc (ssh, gmail, sans gh) ; rag3db master **500b1ede4** avec le pointeur ; garde : aucun code apporté (912ea2084 et 7961fae7f déjà dans la base jouée), écart tree poussé/joué 0, « garde à jour », dernier pas sous poste push, trois contrôles. ED passe à 2a′ (la montée 4.3.7 reste à lucivy). File : archi (openssl au vert, puis recherche-lit-le-catalogue et les thèmes) → codeparsers (provenance, rebasée sur b9029dc) → lucivy (4.3.7).
- **Lucie : « tous les week-ends on se focus sur le projet, donc oui on switch »** → compte `gh` actif basculé sur **L-Defraiteur** par c7 (`gh auth switch`, vérifié) ; les PR sur codeparsers et lucivy peuvent passer par gh ce week-end.

## Vers 14 h 1x — essai.js prêt ; les lots suivants d'embarquement

- **embarquement, essai-sans-service-2 (8869f1309) vert, « à toi »** : ticket pour recherche « search_code avant l'index ne balaie plus » (alpha.1 balaie, 012be4a58 répond « jamais construit » ; piste lue : `&& !projected` de c67ca39c6, vraie quand la cible porte une occurrence_projection) ; `test/essai.js` retire toutes les variables de service et sépare exigences et rouge connue (« OK (rouges connues : 1) ») ; ticket d'embarquement fermé. **Ordre validé des lots suivants** : reprise après plantage d'un fournisseur sous bail ; le groupe bge-m3/relecteur/OCR à la demande d'un bloc (vrai jeu avec un fournisseur à part) ; les services résidents par Activation **au mot de Lucie** ; le préchauffage mesuré ; granite-278m attend sa sortie.
- **Lucie, la règle derrière le compte gh** : « jamais push sous le nom de mon compte pro, pour éviter des problèmes légaux — imagine qu'ils veulent me voler mon projet ». Invariant : aucun commit, push, tag ni PR sur ses dépôts sous le compte ou l'adresse de l'employeur, quel que soit le compte `gh` actif. Vérifié : 0 commit de master avec une adresse pro (4 873 gmail, le reste auteurs Kuzu) ; codeparsers 85 gmail + 3 luciformresearch (personnel). push_garde.sh refuse déjà une identité autre que le gmail.
- **Le journal des nouveautés** (docs/nouveautes.md, tenu par chaque session à son atterrissage) : 14 entrées des 13 et 14 nov. (octets des scopes, contenu partagé, tâche = couche, chaînes, flux partagé, équipier, grep, modèle dense local, publications durables, chat, thèmes, boîte, ssh, empreinte, accord). Deux manques relevés à la relecture de Lucie : l'embarqueur à la demande et l'activation distante (embarquement), le verrou de la clé primaire posé ce matin (cœur) — demandés.
- embarquement a poussé essai.js (1b22409b3) et le ticket pour recherche sur le balayage avant index.
- embarquement : essai.js sur master (1b22409b3) après un premier dernier pas arrêté par la garde (code entré : le pointeur codeparsers et code.rs) → rebase, rejeu hors verrou, second dernier pas ; ménage par la garde. Précision donnée : un target existant qui a déjà compilé openssl-sys n'est pas « neuf », on peut bâtir dedans (reprise par RAG3WEAVER_TARGET_REPRENDRE=1) ; après le lot d'archi, plus d'openssl du tout.
- **tty, tty-6 prêt sauf son e2e** (cx-tty-6, dc8c86b5f ; e2e_tty refusé par openssl-sys sur target neuf) : sous tmux et mosh tout tient ; une dégradation corrigée rouge d'abord (Échap attend 500 ms sous tmux, Ctrl-G arrête sans délai) ; la couleur vraie tombe en 256 sous tmux vu par ssh sans casser ; ticket pour l'historique perdu sous mosh ; page docs/tty/14-11-2026/01. Attend le lot d'archi pour rejouer e2e_tty.

## Vers 14 h 2x — 2a′ : l'ancre doit avoir des générations (décision pour Lucie)

- **ED, page de conception de 2a′ (8aa3c3949), avant tout code** : l'ancre réécrit ses lignes `Scope` sur place (uuid = clé, le snapshot efface) ; une couche qui épingle l'ancre à sa base perdrait cet état dès que l'ancre avance → **l'ancre doit elle-même avoir des générations**, un choix de stockage qui touche toutes les lectures de `Scope`. Proposition : lignes immuables une fois publiées ; visibilité par deux colonnes de rang (`_depuis`, `_jusqu_a`) plus `_couche` (le filtre le moins cher) ; uuid par génération pour les lignes neuves seulement (aucune base ne change d'uuid) ; filtre posé surtout à la traduction IR → dialecte, plus une dizaine de lecteurs un par un (relevé E/R/M/N dans la page). À décider avant le code : l'estampille de format, le coût du prédicat (mesuré sur une copie de code-lourd), l'alternative d'une table d'appartenance. **Pour Lucie : « l'ancre garde ses états passés, les lignes ne s'effacent plus qu'à la rétention ».** Arbre principal relit.
- cœur : ligne de nouveautés pour la clé primaire (6f132eb56) ; celle du COMMIT sous index vectoriel manquait aussi, demandée (posée par 0c41f9fe6).
- embarquement : les deux lignes de nouveautés poussées (970176633 : l'embarqueur à la demande ; le fournisseur sur l'autre poste) et le README corrigé (résidence, baux, activation distante — chaque phrase vérifiée contre le code).

## Vers 14 h 2x — Lucie dit oui aux générations de l'ancre

- **Lucie : oui à 2a′** — l'ancre garde ses états passés ; lignes immuables, visibilité par rangs (`_depuis`, `_jusqu_a`) + `_couche` sur les nœuds et les arêtes, uuid inchangés pour les bases existantes, filtre à la traduction IR → dialecte. ED : mesure du coût du prédicat sur une copie de code-lourd d'abord (et la table d'appartenance chiffrée), l'estampille fixée, puis le code rouge d'abord ; arbre principal relit.
- **codeparsers** : les deux lots rebasés sur le codeparsers master d'ED (provenance : e23ed0d / rag3db 70ccddd1b, rejeu court à son tour après archi) ; la partie codeparsers du second lot faite (`provenance-manifestes-2` f413dc6 : options `declared_dependencies` → dependency et `project_packages` → project, comparaison par la tête du nom ; sans options rien ne change ; codeparsers décide parce qu'une arête fondue ne garde qu'un import_path ; témoin 4 cas rouge sans, vert avec ; 298/0 ; un nom absent des manifestes reste `inconnue`). Trou antérieur en ticket : en Python `import requests.adapters` ne produit aucune bibliothèque. Suite : lire les manifestes dans `synchroniser` (code_sync.rs), chemins dits dans /etat.
- **2a′ en trois sous-lots (ED, accord d'arbre principal, validé par c7)** : 2a′-1 la visibilité sans changement de comportement (format estampillé, `_couche`/`_depuis`/`_jusqu_a` sur Scope, File et leurs arêtes pour les bases neuves, l'entité Generation, le prédicat imposé aux lecteurs — toujours vrai au rang 0, la passe prouve qu'aucune réponse ne change ; une base existante se lit comme avant et le dit) ; 2a′-2 l'ancre immuable (snapshot qui retire, uuid par génération pour les neuves, arêtes entrantes recopiées, couche de travail au rang en attente, publication au commit ; mesure bornée des déplacés sur une copie de code-lourd avant le push) ; 2a′-3 l'écriture d'une couche. Puis 2b′.

## Vers 14 h 2x — la reprise après plantage d'un fournisseur

- **embarquement, `reprise-apres-plantage`** (12374929d test, cecbf8853 fix) : A — un fournisseur à la demande tué sous bail entre deux calculs : **vert avant tout code** (la relance unique d'hier), témoin gardé et dit ainsi ; B — la relance qui échoue : **rouge** (le refus ne disait ni le départ ni la relance) → vert, le refus dit les deux temps ; C — une déclaration `Maintenir` dont le fournisseur plante ne relançait rien jusqu'à la réouverture : **décision (a)**, Maintenir relance une fois comme a_demande, refus dit si elle échoue, `relances: n` au /sante. Défaut en ticket : le refus nomme le type de service (« rag3weaver-embeddings »), pas le programme lancé. Passe à luciepc (target-activation repris, 0c41f9fe6).

## Vers 14 h 2x — openssl : le lot d'archi est vert, « à toi »

- **archi, openssl-sys-pour-openssl-4 (c1798ff67)** : passe complète sur un **target neuf sans aucune variable OPENSSL ni PKG_CONFIG_PATH** (unset) sous openssl 4.1.0 : check_features 8/8 (toutes les combinaisons compilées de zéro — le témoin qui compte), script vert, lib 2365/1 (le seul rouge : L3 nommé, rejoué vert seul), navigateur 43/43, e2e_hooks. Douze paquets partis du Cargo.lock, ticket fermé. **« À toi »**, puis recherche-lit-le-catalogue et les thèmes dans le même tour ; après : codeparsers, lucivy, tty-6.
- **recherche, le plafond d'instantané codé** (branche plafond-d-instantane, b0c214cfb) : RAG3WEAVER_SNAPSHOT_MAX_MS disparaît, l'instantané tient tout l'appel ; une ouverture refusée rompt l'instantané et l'appel entier se rejoue au plus deux fois (`instantane_renouvele: n`), au-delà refus dit ; un pool plein attend 5 s puis refuse avec la raison ; compteurs `refused_full` et `broken_snapshots` ; deux témoins rouges d'abord, quatre verts dont L3 ; ticket fermé dans le lot. Deux passes en file au portable derrière archi (luciepc sans creux).
- **luciepc à 15 h : charge 33 sur 24 cœurs**, 4/4 places lourdes (la passe réelle de 96, la passe d'embarquement, sans-residu de lucivy, concurrence_test du cœur), une mesure d'ED en attente (bloque l'entrée des lourds suivants, c'est la règle), codeparsers derrière. **Quatre lourds à -j8 sur 24 cœurs = 32 fils pour 22 disponibles** : c'est la cause probable des deux rouges d'environnement du jour (le crash physique, le délai de la carte de progression). Proposition à Lucie : lourds-max de luciepc à 3.

## Vers 14 h 4x — rag3weaver ne dépend plus d'openssl au bâti

- **archi a poussé** : master = **2a1f52c21** (correctif 015172c00 : hf-hub sans ses features par défaut, le reqwest des dev-deps en rustls ; témoin 05077b8d3 : le lock ne tire ni openssl ni native-tls). Rejeu court sur un target neuf sans variable (F8 8/8, lib ciblée 43/0, navigateur 43/43), garde à jour, trois contrôles. **Toutes les sessions peuvent rebâtir à neuf en rebasant sur 2a1f52c21**, sans OPENSSL_* ni PKG_CONFIG_PATH. Archi enchaîne recherche-lit-le-catalogue puis les thèmes.

## Vers 14 h 5x — la mémoire de Lucie archivée pour de vrai ; les sources suivies, une abstraction unique

- **96, passe réelle** : 51/52 à 14 h 48, 0 erreur, 112 fiches (93 créées) ; copie vérifiée identique à l'instantané (diff vide, aucune écriture pendant la fenêtre) ; **« rendu » à G à 14 h 50, backend rouvert à 14 h 51 min 28 s** (base rendue 77 Mo, lib 0c41f9fe6, recall_fiches ok, 0 erreur). La mémoire de Lucie a été indisponible de 13 h 45 min 38 s à 14 h 51 min 28 s. 96 fait ses comptes et la page (d).
- **Lucie, en discussion** : « tout doit être paramétrable — quelqu'un qui veut tout son arbre git indexé, quelqu'un qui veut plusieurs worktrees chargés, quelqu'un qui veut quelques worktrees plus telle branche racine par worktree ; trois paramètres d'un même truc générique, pas trois implémentations ; une abstraction où des choses pas prévues marchent, pas trois modes stricts ». Proposition de c7 : **une source suivie** = un sélecteur git (branche, motif de refs, worktree, commit, plage — tout ce que git nomme) + trois réglages (parente déclarée ou inférée ; histoire : tous, N, la pointe, les cités ; couche de travail oui/non) ; tout se dérive (génération par commit résolu, couche par source, « connu non indexé » pour le reste, rétention par source ou défaut). Les trois cas = trois déclarations ; les « branches suivies » d'hier sont ce même objet avec un sélecteur limité au nom de branche ; à généraliser avant 2a′-3. Questions posées à Lucie : les défauts quand rien n'est déclaré (main entière + le worktree courant ?), le nom dans le manifeste.

## Vers 14 h 5x — fenêtre fermée : la mémoire de Lucie est archivée

- **96 : fenêtre fermée** (G a rouvert à 14 h 51 min 28 s). La passe : 52 documents, 432 sections, 0 erreur, 61,9 min ; 374 sections rangées et **58 jugées none** (contre 18 au troisième blanc : l'effet de « le chantier, pas le moment » sur les sections de journal — le rapport d'orchestration du 7 passe de 60 à 32 rangées) ; **95 fiches créées, toutes proposed** (40 à une section, 24 d'un même document, 31 de plusieurs) ; 22 sections sous des fiches existantes ; 0 refus de nom daté. Contrôle du retour : base et ~/memoire-longue identiques à l'instantané (aucune écriture pendant la fenêtre). L'hôte : recall_fiches 116 fiches ; le dossier fiches/ en compte 114 — **écart de 2 non expliqué**, dans la page. Filets gardés au portable (avant-archivage-14-nov, remplacee-archivage-14-nov). Restent 4 paires de noms voisins manquées parmi plus de 100 fiches et des doublons de sens. Page (d) en écriture.
- **lucivy** : montée 4.3.7 reportée sur une branche neuve posée sur 0741a940f (après openssl), Cargo.lock cohérent (`cargo metadata --locked`), nouveauté réinsérée ; passe à luciepc depuis 15 h 02 en file. Lot TempDir : PR lucivy #32 verte (lucivy-core 228/0, lucivy-cpp 19/0, TMPDIR vide après, témoin statique 85 endroits), avec un correctif produit (le binding C++ laissait un cache à chaque échec d'ouverture) → version au mot de Lucie.
- Lucie : « pourquoi pas une déduction du parent commun le plus récent depuis les branches actives ? » → oui : **la racine implicite d'un dépôt est le merge-base de ses sources suivies**, la parente d'une source non déclarée est la source dont le parent commun est le plus récent, la racine avance avec les sources (une génération de plus, les anciennes restent pour les lectures épinglées) ; cas limites dits (une seule source ; historiques orphelins). Défaut sans déclaration : les worktrees présents, pointe + couche de travail, racine calculée ; main n'a de place que déclarée.

## Vers 14 h 5x — la page de la première passe ; un arrêt qui laisse les fichiers en retard

- **96, écart 116/114 départagé** : deux fiches de la passe (dernier document) étaient en base sans fichier, quatre autres fichiers périmés ; l'hôte les a réécrits à la réouverture (fiches/ = 116). **Cause : l'arrêt du backend n'écrit pas les fichiers de ses dernières écritures** (ticket docs/memoire/tickets/2026-11-14-un-arret-du-backend-laisse-les-fichiers-de-ses-dernieres-ecritures-en-retard.md). Page (ee4ef20e2) : docs/memoire/14-11-2026/01-la-premiere-passe-de-l-archiviste.md + annexe des 95 fiches pour Lucie ; les « about » pauvres relevés. Lots donnés : l'arrêt qui laisse les fichiers en retard (intégrité du miroir, rouge d'abord) ; la recherche par nom qui manque des paires parmi >100 fiches (un plafond silencieux ?) ; les « about » pauvres (garde).
- Lucie : « le parent peut être zéro code ; un scope commun à deux états est détecté et actif sur les deux » → oui : c'est 1a, le partage est par contenu (une version citée par toutes les générations qui la contiennent), la racine et les parentes ne sont que des réglages (vide / merge-base / déclarée ; aucune / inférée / déclarée).

## Vers 14 h 5x — la page des sources suivies

- **Page écrite pour Lucie, ED et arbre principal** : `02-les-sources-suivies-une-abstraction-pour-les-couches.md` — le partage par contenu comme socle ; une source suivie = sélecteur git + trois réglages ; la racine en réglage (vide, calculée, déclarée) ; le tableau des cas d'usage (dont un non prévu) ; le défaut sans déclaration ; ce qui existe et ce qui se généralise avant 2a′-3 ; les questions ouvertes (le nom, les sélecteurs, le coût d'une racine qui avance). Envoyée à ED (rien ne change pour 2a′-1 et 2a′-2) et à arbre principal (la généralisation de la déclaration est son lot après 1b).

## Vers 14 h 5x — le correctif (a) tient sous charge ; une troisième famille de plantage

- **cœur, DdlEveryMinute sous profil sur 9b0e73c9c, 8/10** : **counts-match-commits 0 sur 8** sous jusqu'à 1 327 refus du pool par tour (3/10 avant) ; annulations sous le verrou 362, 91, 492, 691, 396 par tour. Autres rouges : known_red (index-creation, checkpoint-holds-writers, create-rel-table-holds-writers, catalog-as-expected). **Tour 6 : SIGSEGV après 1 min 40, sans aucun refus du pool** — pile : point de reprise demandé par un COMMIT → NodeTable::checkpoint → Column::checkpointSegment → StringChunkData::write → DictionaryChunk::appendString → _Hash_bytes : une chaîne illisible dans le dictionnaire d'une colonne pendant son point de reprise. **Troisième famille** (ni clé primaire, ni annulation) ; le SIGSEGV de la nuit sur f41c0ba5b ne se relit plus (binaire rebâti). Ticket. Décision : « à toi » au juge rejoué vert, le SIGSEGV nommé sous ticket dans le push ; SingleWriterCrash ×36/×36 puis le juge, vers 17 h ; la lib d'après au hash du cœur ; le dictionnaire = lot n° 1 ensuite.

## Vers 15 h 0x — la charge de luciepc : borner par une enveloppe CPU, pas baisser les lourds

- Lucie, sur la proposition « lourds-max 3 » : « faut voir si on sait régler pour éviter les rouges malgré le partage ; c'est à cause de lucivy ? on pourrait impl une variante tokio async… ». Réponse : pas lucivy en premier (quatre lourds : 96, embarquement, lucivy, le cœur, chacun -j8 + ses fils de test) ; l'async ne réduit pas un travail de calcul. **Décision : une enveloppe CPU par lourd dans `poste lourd`** (CPUQuota = (cœurs − 2) / lourds-max, et RAYON_NUM_THREADS / CARGO_BUILD_JOBS / RUST_TEST_THREADS dérivés), déclaratif à côté de `lourds-max` ; **mesure d'abord** par `systemd-cgtop` de ce que chaque lourd consomme. Lot à G.

## Vers 15 h 0x — trois verts dans la file

- **tty-6 vert** (cx-tty-6-2, 4583eff8e, sur 0741a940f) : crate 39/39 et 10/10, e2e_tty 4/4, catalogue 12/12, target neuf sans variable ; **706 crates compilés sans un seul openssl-sys** (seul `openssl-probe`, pur Rust, pour les certificats de rustls) — témoin du correctif d'archi.
- **recherche, surveillant-2 vert** (a36f707dd, au portable) : lib 2364/2364, e2e_lectures, e2e_code, invariant_des_vecteurs 12/12, F8, navigateur 43/43 ; le plafond suit sur la même place.
- **codeparsers, provenance vert** (51f97a8c1 sur 0741a940f, pointeur e23ed0d ; F8, code:: 42/0, e2e_code 53/53, postgres 36/36) ; le second lot (manifestes, f3def09ce / f413dc6) vert dans ses essais : `src/manifestes.rs` lit Cargo.toml/lock, package.json, pyproject, requirements*, vcpkg.json et les workspaces ; `synchroniser` les note (`manifests_read`) et les passe à codeparsers ; témoin e2e rouge sans, vert avec ; un vrai défaut corrigé (le `]` des extras pyproject perdait PyYAML). Limite admise si dite : un manifeste édité seul n'est relu qu'à la synchro (ticket pour la relecture au changement).
- **File** : archi (recherche-lit-le-catalogue, thèmes) → codeparsers → lucivy (4.3.7) → tty-6 → recherche (surveillant, plafond) → 96 (ses lots) → cœur (C++, vers 17 h).

## Vers 15 h 0x — /tmp purgé ; les machines virtuelles à ressources exactes (graine)

- **hotfixs : /tmp purgé aux deux postes** sur le mot direct de Lucie dans sa session (elle n'était pas bloquée par un refus : elle attendait ce mot, à raison, pour un effacement de masse) : portable 6 762 entrées, **1,78 Gio** (Sairen 1 022 Mio secrets/ compris effacés sans être lus, tests .tmp 345, profils Chrome 251, rag3weaver_cache 128, lucivy Codex 75) ; luciepc 8 947 entrées, **2,64 Gio** (profils Chrome 1 897, Chromium 277, tests .tmp 212, bulk 196, cache 101). Chaque entrée revérifiée avant (fd, cwd, root) : 0 tenue, 0 erreur. Resté : claude-1000/, jest_rs/, le Chrome de Lucie, le moins de 24 h, les tests lucivy du jour. Compte rendu purge-tmp-14-nov (3177ee344), « à toi ».
- **Lucie** : « j'envisageais des machines virtuelles avec telle ressource allouée exacte (à part le GPU) ; cette culture de travail pourrait servir au projet ». Réponse : le cgroup de `systemd-run` (CPUQuota, MemoryMax, IOWeight) est la machine virtuelle légère — c'est le lot donné à G ; une vraie VM coûte la mémoire en double et ne partage pas le GPU. Pour le produit : un bac à sable par tâche de l'atelier (nspawn, bubblewrap) quand l'agent exécute des commandes — graine, dans la vision.
- hotfixs : purge-tmp-14-nov sur master (d3d3f2ff9). Lot en attendant la copie migrée : **la troisième famille de plantage** (le dictionnaire d'une colonne de chaînes au point de reprise, core 3980144) — lire le core, témoin réduit rouge d'abord, en liaison avec le cœur qui finit la perte de Note.
- **Lucie** : « on peut dans une certaine mesure se servir de la tablette aussi, qui est puissante ; juste faut pas que le processeur soit monopolisé ni que je ne puisse plus te parler si l'écran freeze ». → Un troisième poste pour les lourds, sous la même enveloppe (cœurs libres, mémoire, entrée-sortie) que celle que G pose ; à préciser : son nom, son système, ssh, cœurs et mémoire, GPU.

## Vers 15 h 0x — la migration 1b tuée dans son enveloppe de 32 Gio

- Lucie a vu deux notifications « Pénurie de mémoire évitée ». Journal du noyau : à 15 h 07 min 42 s, **`rag3weaver-back` (pid 183835) tué dans le scope `run-p183606-i25062946` (`wt-1b-copie.sh`, la migration 1b d'arbre principal sur la copie du dépôt)**, anon-rss 32,4 Gio pour MemoryMax 32 Gio, après 3 min 21 s ; « Memory cgroup out of memory » — l'enveloppe a joué son rôle, le poste n'a pas souffert (121 Gio, 87 disponibles). **Une migration d'une base de 7 Go qui tient 32 Gio en mémoire charge tout d'un coup** : demandé à arbre principal de lire où va la mémoire, de rendre la migration incrémentale (par fichier ou paquet de Scope), avec un témoin qui borne la mémoire au plus gros lot, avant toute relance ; l'enveloppe ne se relève pas.
- Le nom d'hôte du portable est `lucie-tablette` : c'est la « tablette » de Lucie (121 Gio, lourds-max 1), déjà le poste des sessions et de son IDE.
- **cœur, DdlEveryMinute ×10 sous profil sur 9b0e73c9c, fini 15 h 08 : 0 counts-match-commits sur 10** (3/10 sans) sous jusqu'à 1 327 refus du pool ; le SIGSEGV du dictionnaire au tour 6 (ticket, hotfixs) ; les autres rouges known_red. Annulation d'un COMMIT refusé sous le verrou : 2 478 mesures, médiane 0,0 ms, p99 0,3 ms, max 5 875 ms — une seule, l'annulation du `CREATE_VECTOR_INDEX` de 240 à 535 s dont le COMMIT forcé a été refusé : pas un coût neuf (`rollback` tenait déjà le verrou pendant toute l'annulation ; le correctif avance seulement, sans l'entre-deux). SingleWriterCrash ×36/×36 puis le juge, vers 17 h.

## Vers 15 h 1x — le coût du prédicat de visibilité mesuré ; 2a′-1 part

- **ED, mesure sur la copie de code-lourd** (82 855 Scope, 7 278 File, 151 744 CONSUMES ; seule sous poste mesure ; page 03 § 6, c56976b7c) : nœuds ×1,05 (outline 7,0 → 7,4 ms) à ×1,2 ; le compte ×10 mais 0,2 → 1,9 ms ; **un saut : nœuds seuls ×1,24, arête seule ×0,95, mais nœuds + arête dans la même clause ×7,3 (35 ms) — le plan de la conjonction dérape** ; impact récursif ×2,2 (7 → 16 ms) ; la table d'appartenance ×1,2–1,35 plus 646 ms et 82 855 arêtes par génération publiée. **Décision : les deux colonnes confirmées, ×2,2 sur impact accepté, la clause de l'arête écrite à part** (sonde pour une forme ≤ ×1,3, sinon ticket planificateur au cœur avec les EXPLAIN). Estampille `code-scope-v1/content/generations`. 2a′-1 commence.
- **hotfixs, le core du dictionnaire lu** (envoyé au cœur) : SEGV_ACCERR au début d'une page, au bout d'une arène malloc de 64 Mio ; la chaîne fautive lue dans `segmentCheckpointState.chunkData` (Column::checkpointSegment → StringChunkData::write) ; deux lectures : un index de dictionnaire qui dépasse son tampon, ou un tampon libéré puis réutilisé ; Release sans -g, pas de locales. **Le témoin réduit est en pause : un filtre de sécurité a interrompu hotfixs pendant l'écriture du fichier de test** ; fichier partiel non commité, rien bâti ni poussé ; hotfixs demande le mot de Lucie (autre forme, ou rendre le lot au cœur avec la lecture). Je ne fais pas reprendre le fichier par une autre session : c'est à Lucie.

## Vers 15 h 1x — l'enveloppe CPU par lourd

- **G, la mesure** (15 h 03 → 15 h 13, toutes les 10 s, cpu.stat/pids/memory par portée `run-*.scope`) : luciepc charge 4,3 à 16,4 (les 33 de 15 h étaient des pointes cumulées) ; somme des portées ≤ 13,5 cœurs ; chaque lourd ≤ 5,5 cœurs en moyenne mais check_features à 8 cœurs en pointe, reprise/passe.sh 783 tâches et 15 Gio. **Portable (32 cœurs, lourds-max 1) : trois portées en même temps** — les sous-portées des confineurs et passes composées ne passent pas par la file ; wt-verrou-passe.sh 28,5 cœurs en pointe ; wt-1b-copie.sh à son plafond MemoryMax. **Le lot** (`poste-enveloppe-cpu`) : CPUQuota = (cœurs − coeurs-libres) / lourds-max par lourd (luciepc 500 %, portable 3 000 %), `coeurs-libres` fichier du dossier des verrous (2 par défaut), POSTE_FILS et RAYON_NUM_THREADS/CARGO_BUILD_JOBS/RUST_TEST_THREADS ramenés à la part ; témoin cas 8 : quatre lourds factices, 16 386 ms de CPU en 4 s (borne 18 400), **rouge sur master 66 448 ms**. Cas 6c rouge préexistant, dit. Décisions : 500 % ; fichiers `coeurs-libres` explicites (2, 2) ; tickets pour 6c et pour les sous-portées hors file (imbriquer sous la portée du parent) ; « à toi » avec la page et la nouveauté.
- lucivy, 15 h 15 : le témoin rouge **vu sous 4.3.5** (« filtré 2,2637, vue seule 0,0465 ») ; sur la montée (d3595e110, 4.3.7) le témoin passe, lib ciblée fts_handle 13/0, F8 8/8 ; e2e_code en cours, e2e_lectures ensuite. (Mes messages de la dernière heure disaient « 16 h » : il est 15 h 15 ; les en-têtes du rapport sont justes.)

## Vers 15 h 1x — la forme de clause qui tient

- **ED** : `usages` avec le prédicat complet sur la copie de code-lourd — A une clause 38,1 ms ; B WITH avant le saut 25,3 ; **C le bout filtré dans le MATCH, l'arête et l'autre bout après un WITH : 6,2 ms (×1,28)** ; D l'arête dans le motif 35,8. Les plans : en A le filtre de l'arête est poussé dans un SCAN_NODE_TABLE + SEMI_MASKER de toute la table puis HASH_JOIN ; en C il s'applique après la jointure sur les 780 lignes du saut. **Ticket planificateur pour le cœur** (526ed3aad, les quatre formes et les deux plans en fichiers) ; 2a′-1 traduit un Hop sous la forme C ; témoins 4 et 0 rouges d'abord.

## Vers 15 h 1x — l'enveloppe CPU est posée aux deux postes

- **G a poussé 3a654c4d9** : `scripts/poste` (CPUQuota par lourd = (cœurs − coeurs-libres) / lourds-max, POSTE_FILS et les fils des outils ramenés à la part), `test_poste.sh` cas 8 (28 OK, seul 6c rouge préexistant ticketé), la nouveauté, la page `docs/optimiseur/14-11-2026/02-l-enveloppe-cpu-des-lourds.md`, les scripts de mesure, deux tickets (6c ; sous-portées hors file, forme cible `--slice`/`Delegate=`). **`poste` recopié aux deux postes** (md5 f72d9c7a, l'ancien gardé), **`coeurs-libres` = 2** posé ; luciepc : 500 % et 5 fils par lourd ; portable : 3 000 % et 30 fils. Les lourds déjà en cours gardent leur portée sans quota jusqu'à leur fin.
- **arbre principal, la cause de la mémoire de 1b** : `rederiver_tout` met ~68 600 occurrences en file pour un seul drain (DeriveNode garde tout le lot : lignes, morceaux, texte), et le transfert des vecteurs rend tous les morceaux neufs avec leur vecteur en une requête (> 150 000 × 768 flottants en CypherValue). Correctif : lots de 2 000 drainés et écrits au fil de l'eau, transfert par `WHERE v._uuid IN $lot`, journal par lot (étape, lignes tenues, RSS), témoin par la structure (le plus gros lot tenu ≤ le lot demandé). Règle pour la page 29 : rien ne tient en mémoire plus qu'un lot. La relance sous 32 Gio est la preuve.
- G : lot suivant, les sous-portées hors file (les confineurs et passes composées imbriqués sous la portée du lourd qui les lance, `--slice` ou `Delegate=`, héritage du quota et de MemoryMax, `poste --qui` en arbre) ; le cas 6c reste au ticket.
- **G corrige son relevé** : au portable les trois portées n'étaient pas simultanées (deux-passes 15 h 03-04, 1b-copie 15 h 04-07, verrou-passe 15 h 07-12 : une à la fois, la file à une place a fait son travail) ; seul `poste` lance `systemd-run`, aucune sous-portée hors file nulle part. Dit et retiré ; le ticket se ferme « non reproduit » avec la commande par heure ; pas de lot. Suite de G : le cas 6c (un tenant tué par signal garde le verrou — vrai trou du poste), puis la lib d'après au push du cœur.
- **Faute de c7** : « à toi » donné à G pour l'enveloppe du poste pendant le tour d'archi — `scripts/poste` est appelé par run_e2e.sh, donc **du code pour la garde de tous** ; la garde a arrêté le push d'archi, qui rebase (recherche-lit-le-catalogue-4, 1c6cfacf1) et rejoue court. Règle : un lot d'outillage de passe (poste, run_e2e, check_features, cargo_magasin, la garde) prend sa place dans la file comme un lot Rust ; un seul « à toi » ouvert à la fois, et c'est la troisième fois aujourd'hui que je l'oublie.

## Vers 15 h 2x — la montée en lucivy 4.3.7 est verte

- **lucivy** : lucivy-4-3-7-2 (d3595e110) vert sur 0c41f9fe6, target neuf : témoin rouge sous 4.3.5 (« filtré 2,2637, vue seule 0,0465 »), vert sous 4.3.7 ; fts_handle 13/0 ; F8 8/8 ; e2e_code 38/0 ; e2e_lectures 29/0 (un premier 28/1 sans RAG3WEAVER_EMBED_SERVICE, le test exige bge-m3 ; rejoué avec 7878,7879 sans changer le code). En file après archi et codeparsers.
- **archi a poussé recherche-lit-le-catalogue** : master = 6981de5ec (lock_for_read borné à READ_WAIT, la raison dite au-delà ; rejeu court 37/0 et 43/43, garde à jour). Les thèmes (ide-themes-route-5, 2299e0052, avec la nouveauté openssl) en rejeu court, même tour.
- **G, le cas 6c diagnostiqué et corrigé** (branche tenant-signal, ab7c44fb5, non poussé) : il parle de `poste push`, pas d'un lourd, et l'attendu est l'inverse de ce que j'avais écrit — un tenant de push qui reçoit TERM **garde** le verrou jusqu'à la fin de sa commande (deux pushes ne se croisent pas). Cause : `poste push` prend le verrou, lit la CI (jusqu'à 20 s, b0c0de81e du 18 oct.) et n'installait son `trap` qu'après : un TERM pendant la lecture tuait le script, verrou rendu, commande jamais lancée, sans un mot (témoin vert avec POSTE_CI=0, rouge avec la CI, prouvé sur master). Correctif : le trap posé dès la prise du verrou, message qui dit le signal ; test_poste.sh 29/29 ; ticket fermé. En file (outillage de passe = code), puis la recopie.

## 15 h 3x — trois lots verts arrivent en file, la provenance des 415 prouvée

- **codeparsers** a rejoué le juge des 415 avec les manifestes de l'arbre jugé (20 manifestes,
  923 dépendances, 9 paquets du dépôt), sans index ni base : les **241 inconnues tombent à 0**,
  240 passent `dependency` (les paquets npm des `package.json`), 2 `project`, 77 `builtin` ; les
  arêtes fondues `builtin,inconnue` (31) deviennent `builtin,dependency` (30) et `builtin,project`
  (1). Rien ne reste inconnue dans l'échantillon (tout TS/JS ; le cas PyYAML n'y est pas). Chiffres
  au ticket de l'oracle, master da14106a3.
- **96** : le lot miroir est VERT au rejeu (master d776ae37d, lib 0c41f9fe6). Le rouge d'abord :
  une fiche née juste avant l'arrêt n'avait pas de fichier (Debounce 50 ms, `stop_within` cassait
  la boucle). Le correctif : `Backend::stop_reactors` régénère les fichiers composés depuis la base
  dès que le miroir est arrêté ; couvre shutdown, `/backend/close` et le rechargement ; règle écrite
  « la base est la vérité, les fichiers la suivent jusqu'à l'arrêt compris ». e2e_lectures 27/27
  joué puisque l'arrêt est à tous. Ticket clos.
- **embarquement** : le lot (1), reprise après plantage, est VERT sur 17afd3820 à luciepc : `Maintenir`
  se relance une fois, le relancé dit `relances: 1` au `/sante`, le refus dit « parti, relancé une
  fois, rouvrez » ; le compte vit au dossier de coordination (par déclaration, survit à la
  réouverture). Deux relances au même instant peuvent compter double : accepté, **au ticket côté
  docs au push**, pas dans le code (la tête testée ne bouge pas pour un commentaire).
- **ED** : 2a′-1 en cours sur generations-de-l-ancre, rien ne bloque, témoins 4 et 0 d'abord.
- **La file**, un seul « à toi » ouvert, celui d'archi (thèmes, rejeu court) : codeparsers
  (provenance-imports-3, puis son lot des manifestes), lucivy (4.3.7), tty-6, recherche (surveillant,
  puis plafond), G (cas 6c), embarquement, 96, puis le cœur à son juge vert. À chacun : worktree
  prêt, rebase et passe courte au mot seulement.

## 15 h 3x — les thèmes sur master, la file passe à codeparsers

- **archi a poussé 061669ce5** : `/ide/theme/<nom>.json` sert chaque `templates/themes/<nom>.json` depuis
  les gabarits embarqués, `/ide/themes.json` donne la liste, un nom inconnu rend 404 ; le menu de
  l'en-tête montre les cinq thèmes (dont cyberpunk, cyborg, alien), le choix tient au rechargement.
  Rejeu court sur 2299e0052 (lib ciblée 13/0, navigateur 44/44), garde à jour, trois contrôles lus.
  La ligne de nouveautés openssl est dedans. Archi prend la jauge du contexte puis quatre mineurs.
- **« À toi » à codeparsers** pour provenance-imports-3 seul ; son lot des manifestes attend un second
  mot. Puis lucivy, tty-6, recherche, G, embarquement, 96, le cœur.
- **lucivy** a livré `index_report()` (PR #33 sur #32, sans tag) : par segment, par champ (p50/p90/
  p99/max, dix plus gros, doublons exacts), octets par famille, rapport index/texte ; il retrouve la
  mesure du matin sur Lucivy_Scope (4,59 Go pour 932 Mo, ×4,9, 11,9 % supprimés, 843,7 Mo de
  doublons). Commande relayée à recherche (exclusions, octets des scopes) et hotfixs (purge des
  orphelins) : le gain se prouve avec, sur une copie de la table. Fusion #32 puis #33 à la CI verte,
  sans tag.
- **ED** : rouge du témoin 4 vu (generations-de-l-ancre-2, 7072aa733 : `_couche` absente, estampille
  `content`). Bornes tranchées : les arêtes avec 2a′-2 (un Hop part d'uuids visibles et ne filtre que
  le nœud atteint) ; postgres hors du lot par `sait_les_generations` du dialecte, e2e_postgres rejoué
  et un ticket pour la forme postgres.
- **hotfixs** : le témoin du dictionnaire n'est plus en pause, Lucie a dit « go » dans sa fenêtre ;
  trois cas dans string_rewrite_test.cpp (chaine-au-point-de-reprise, c1a8f8171), sous ASAN dans le
  build du cœur. Appris en passant : un ADD COLUMN d'une autre connexion est accepté pendant une
  transaction d'écriture ouverte — ticket moteur demandé si c'est un défaut d'isolement.

## 15 h 4x — la provenance des imports sur master, second mot à codeparsers

- **codeparsers a poussé 8b3aa1041** : le nœud `Library` dit d'où vient la bibliothèque (builtin,
  dependency, project, inconnue) et l'arête de liaison porte `provenances`. Pointeur du sous-module
  e23ed0d6e, en avance rapide depuis b9029dc d'ED, vérifié sur master. Rebase 51f97a8c1 → 2209f2fa0
  (provenance-imports-4) ; `--code-apporte` depuis la base testée donnait 14 chemins de code, d'où
  le rejeu court sous le verrou (lib 0c41f9fe6 : check_features 8/8, `code::` 42/0, e2e_code 53/53,
  e2e_postgres 36/36) ; au dernier pas master avait encore bougé (85e46fd74, docs), rien de code,
  rebase sans rejeu, garde à jour, trois contrôles lus.
- **hotfixs a poussé 85e46fd74** hors file (docs seules) : lecture du core 3980144, témoin réduit
  vert 4/4 sous ASAN (le plantage n'est pas dans ces formes), et le ticket neuf « un ADD COLUMN
  passe sous une transaction d'écriture ouverte » : l'ALTER d'une connexion B aboutit avant le
  COMMIT de A, A garde son catalogue à trois colonnes, valide, et ses lignes prennent le défaut ;
  PostgreSQL ferait attendre l'ALTER (ACCESS EXCLUSIVE). Gravité au cœur. Le témoin
  (chaine-au-point-de-reprise) atterrit en non-régression, en file après 96, avec un commentaire
  qui dit ce qu'il chassait sans le trouver.
- **Second « à toi » à codeparsers** : le lot des manifestes. Puis lucivy, tty-6, recherche, G,
  embarquement, 96, hotfixs (le témoin), le cœur.

## 15 h 5x — la provenance par les manifestes sur master, le tour à lucivy

- **codeparsers a poussé e34da698d** : `manifestes.rs` lit Cargo.toml, Cargo.lock, package.json,
  pyproject (un correctif : une liste pyproject ne se fermait pas sur le `]` des extras,
  `requests[socks]`), la provenance `dependency` et `project` en sort, `/etat` dit la limite des
  manifestes à côté de `manifests_read`. Pointeur du sous-module f413dc63ac vérifié. Passe complète
  (lib 0c41f9fe6 : check_features 8/8, lib 2 374/0, e2e_code 54/54, e2e_postgres 36/36,
  e2e_code_sync 18/18, e2e_usages 19/19), dernier pas sans rejeu (rien de code depuis la base),
  garde à jour, trois contrôles lus. Les 415 jugés ce matin tiennent (241 → 0 inconnue). Ménage à
  luciepc autorisé, journaux archivés d'abord ; ensuite ses tickets (import Python pointé, builtins
  en fichiers de données).
- **« À toi » à lucivy** pour la montée 4.3.7. Puis tty-6, recherche (surveillant puis plafond, un
  tour), G (6c), embarquement, 96 (miroir puis nomex, un tour), hotfixs (le témoin), le cœur.
- **lucivy** a fusionné #32 et l'outil de graisse (#34, cinq commits reportés sur main, fdf20d4), sans
  tag. Lucie a tranché dans sa fenêtre : fusion dans main dès la CI verte sans lui repasser, une
  version seulement quand ce qui s'accumule apporte assez, ou pour un correctif obligatoire. Écrit
  en mémoire.
- **recherche** : plafond-d-instantane vert sur toute la passe au portable (9b7ff76e8 ; lib entière
  2 367/2 367, e2e_lectures 27/27, e2e_code 36/36, navigateur 43/43) ; un crochet mort sans
  `rag3db-native` corrigé pour check_features ; un faux rouge de son moniteur (il lisait le
  passe.code d'avant) dit pour ce qu'il est. Ses deux lots dans un même tour ; elle passe aux
  exclusions 1 et 5 dans le manifeste.
- **96** : nomex vert — `archive_subject` cherchait la voisine avec les options des candidates
  (`limit 8`) : elle n'y était que deux fois sur huit, classée jusqu'au 38e rang sur 116. Il lit
  maintenant toutes les fiches sans classement, nom exact d'abord, sinon la plus proche en mots. Le
  témoin a dû mettre 119 leurres plus proches que la voisine pour rougir. Miroir puis nomex dans un
  tour ; le lot 3 (les about), empilé, suit s'il est vert.
- **hotfixs** : le témoin du dictionnaire (chaine-au-point-de-reprise-2, 9ded42107) vert en Release
  sur la tête rebasée, en file après 96.

## 16 h 1x — la file passe en pile, et son premier atterrissage

- **Lucie** : « rien de commençable en parallèle, même si invalidable de suite ? si on définit des
  contrats ça peut se faire non ? ». Oui : la file est devenue une **pile** (page 03 du jour) —
  chacun rebase son lot sur la tête du lot d'avant, tous rejouent en même temps, les hashes sont
  figés, et au « à toi » rien ne se rejoue si tout a atterri comme prévu. Dix lots étaient en file ;
  en vingt minutes, neuf têtes de pile sont publiées (lucivy → tty → recherche → G → embarquement →
  96 → hotfixs → ED ; archi attend ED), et les rejeux tournent, quatre lourds à luciepc.
- **La garde ne savait pas la pile** (vu par recherche) : après `rebase --onto`, HEAD ne descend
  plus du commit testé, et la base du rejeu ferait voir le code des lots d'avant. G écrit le mode
  `PILE_AVANT` en lot à part (rang 11 ; sa tête de pile était déjà la base d'embarquement), posé
  d'abord dans la copie du cache, qui fait autorité.
- **lucivy a poussé 4.3.7 (c99b2f05e)**, premier atterrissage de la pile, contenu identique à la base
  testée cd6d02907 ; tty peut suivre. Leçon : un push refusé laissait le worktree rebasé.
- **codeparsers** : la conception « du nom importé au paquet » est sur master (cb74f585c) ; forme
  validée (TOML pour les fichiers de règles de codeparsers, JSON pour la table déclarée du manifeste
  rag3weaver ; défaut compilé plus dossier remplaçable par langue entière ; table dans le manifeste
  du backend, un fichier du dépôt indexé en ticket). Lucie : « et alias aussi non ? » — deux sortes,
  l'alias du nom lié qui ne change pas la provenance, et l'alias de résolution (tsconfig paths,
  vite/webpack, go.mod replace, Cargo package=/patch, PSR-4) lu dans les configs du dépôt, avant la
  règle par le nom. Le lot 1 (import pointé) entre au rang 12 à son vert.
- **arbre principal** : 1b par lots sur la copie — la dérivation passe (40 lots, 179 s, RSS 2,3 →
  11,7 Gio puis stable), le transfert meurt au lot 42 000 / 72 652 sur « buffer pool is full » (8 Gio),
  après 61 transactions validées sans point de reprise explicite : ticket moteur à la relance avec
  `CHECKPOINT` par lot, qui départage « pages validées non évincées » de « la jointure du transfert ».
- **hotfixs** : le témoin du dictionnaire ne reproduit pas le plantage (vert 4/4 sous ASAN) ; l'ADD
  COLUMN sous écriture ouverte en ticket moteur ; le cœur donne son verdict vers 17 h 15.

## 16 h 4x — huit lots atterris par la pile en vingt minutes, 2a′-1 sur master

- **La pile a tenu** : tty (ce4a3140a, 16 h 25), recherche (a48b535d1, dix commits en un push), G
  (89f46c87d, cas 6c clos), embarquement (67dabc121, la reprise après plantage), 96 (c350dc9c3, trois
  lots en un push : le miroir régénéré à l'arrêt, la recherche par nom sur toutes les fiches, l'about à
  six mots), hotfixs (42c84cb77, le témoin du dictionnaire en non-régression), **ED (78c8997de, 2a′-1 :
  Generation, `_couche`, `_depuis/_jusqu_a`, estampille generations, le prédicat de visibilité dans les
  lecteurs, témoin de non-fuite)**. Huit « pile tenue », tous les range-diff en `=`, **zéro rejeu au
  dernier pas**. Archi a le « à toi » ; le cœur à son juge vers 17 h 15.
- **La garde a appris la pile en route** : G (mode `PILE_AVANT`, relu par recherche qui a trouvé le
  `--onto` manquant, posé au cache des deux postes à 16 h 22) ; puis six corrections de forme vues aux
  pushs réels (base lu trop tôt, journaux de lib, empreinte vide, sens de la dérive, range-diff imprimé,
  `nproc` sous quota) dans garde-pile, rang 12, relecture d'ensemble avant son tour. Règle apprise :
  en mode pile **on ne rebase pas à la main**, la garde le fait.
- **La CI GitHub est rouge depuis 16 h 30** (ticket ce525a014, G) : `dtolnay/rust-toolchain@stable` a
  pris une chaîne plus récente que les postes (portable 1.98.1, luciepc 1.98.0, par rustup) et déprécie
  `fetch_update` (read_pool.rs:261, lot du plafond). Recherche corrige en une ligne au rang 11. Le fond
  est à Lucie : épingler (rust-toolchain.toml 1.98.1, la CI et les postes compilent la même chose, la
  montée devient un lot) ou flotter (la CI rougit sans lot fautif à chaque dépréciation). Dans les deux
  cas luciepc doit rejoindre 1.98.1.
- **Le compte gh actif était revenu sur le compte pro** : remis sur L-Defraiteur ; la clé ssh répond
  L-Defraiteur, aucun push n'était concerné.
- **1b** (arbre principal) : le refus « buffer pool is full » n'était pas le moteur mais la requête de
  transfert (jointure de chaque morceau neuf à toutes les occurrences d'une version) ; un représentant
  par version ; CHECKPOINT par lot retiré (mesuré sans effet) ; ticket souhait pour le cœur (le tri
  refuse au lieu de déborder).
- **Lots lancés** : embarquement, le groupe bge-m3/relecteur/OCR à la demande avec des **activations
  nommées au manifeste** (forme B, un bloc inline vaut une activation nommée d'après son modèle) ; 96,
  le journal des chantiers en une fiche par chantier (156 sections, copie, fin vers 17 h 15) ; tty-7
  (journal de séance, repli sous mosh) prêt, rang 15 ; lucivy, profil léger et relevé des fils (sa
  mesure retirée de `poste mesure` pour laisser passer la pile) ; codeparsers, la règle « du nom
  importé au paquet » validée (TOML/JSON, défaut compilé + dossier, table au manifeste), lot 1 rang 13.
- Faute de c7 : un ticket demandé à 96 sur une ligne ✗ d'e2e_postgres citée sans son test ; 96 a lu le
  code, le ✗ est asserté (la falaise du plancher) ; pas de ticket.

## 17 h 1x — une faute de c7 : la mémoire de Lucie fermée derrière une mesure

- **Le journal des chantiers sur copie** (96, 16 h 44 → 17 h 11) : 156 sections, 56 fiches proposées
  (37 à une section, 19 thématiques — les 19 conseils du harnais en une, codeparsers en une : Lucie
  scinde au survol), 2 rejointes, 0 refus ; le miroir corrigé vérifié (172 fiches, 172 fichiers ; il a
  écrit à la fermeture les deux fichiers que l'ancien défaut avait laissés manquer). « Va » donné pour
  la vraie passe, en-tête du journal écarté.
- **La faute** : dans la même minute, « feu » à la mesure d'ED (`poste mesure`, qui exclut tout lourd)
  et « va » à la passe de 96 (sous lourd, après fermeture du backend memoire-longue par G à 17 h 15).
  La passe de 96 est restée 4e en file, la mémoire de Lucie fermée pour rien. Choix : G rouvre tout
  de suite, l'instantané avant-chantiers-14-nov (116 fiches, sommes vérifiées aux deux postes) est
  gardé, la fenêtre se refait après la mesure. Règle en mémoire : lire `poste --qui` avant tout « va »
  qui ferme une base.
- **La pile** : recherche (la ligne pour la CI, rouge reproduit sous rustc 1.100.0) vers 17 h 15, le cœur
  reposé dessus (27cfe33a8) à son juge vers 17 h 30, puis tty-7 (après la mesure), les quatre mineurs
  d'archi (verts), garde-pile, codeparsers, les exclusions.
- **2a′-2** (ED) : la migration versions → générations manquait à 2a′-1 (une base à versions restait
  « sans couches ») : écrite, rouge puis vert, estampille en dernier ; la mesure de la clause sans
  filtre tourne sur une copie de code-rag3db-depot migrée par 1b (79 662 scopes), (a)/(b)/(c), A/B/C,
  seuil 20 % ou 50 ms avant tout drapeau.
- **Archi** sans lot jusqu'à sa base : « aucun client HTTP sans délai » (cinq clients de production, et
  le test de la lib qui parle au service s'il partage le mécanisme ; un délai nommé, réglable).

## 17 h 4x — la mesure de 2a′-2 tient le poste ; 1b vert ; ce qui attend Lucie

- **Lucie** : « qu'est-ce qui prend si longtemps par recherche ? faut se concentrer sur optim ça, ça fait
  partie du lot : si on apporte la feature faut faire le max pour qu'elle devienne viable » ; « et
  filtré ça prend combien ? faudrait voir filtré vs non filtré ». La mesure unique d'ED (partie à
  17 h 42, seule sur le poste, ~1 h 10) : référence R = la même base avant 1b, A/B/C à versions, durées
  par nœud lues sur NodeRun, trois formes (sans filtre, par chemin, par langage) sur R et A, 10 × 5.
  Premier chiffre : **6,8 s par search_code sans filtre** sur la base à versions, dont 0,8 s de masque ;
  migration 20 ms. 2a′-2 commence par rendre la recherche viable ; banc prépare le juge de parité.
- **1b vert sur la copie réelle** (page 03 de la pile) ; 0 orpheline (hotfixs) ; 1 291 morceaux à
  arêtes multiples, antérieur, ticket ; le sort de `Scope_Chunk` après 1b demandé.
- **La pile** : dix atterris ; la ligne pour la CI (recherche, passe de 25 min faute d'un journal
  court, son script apprend `SUITES=`) vers 18 h 10 ; le cœur en file derrière la mesure (26 essais
  sans correctif à compter : un rouge suffit) ; tty-7, les mineurs, garde-pile, codeparsers (import
  pointé vert), les exclusions, embarquement (activations nommées vertes), 1b, DB_THREADS, les clients
  sans délai : après la mesure. Toutes les sessions ont un lot.
- **Ce qui attend Lucie** : la chaîne Rust (épingler ou flotter) ; le classeur Laya dans le rappel
  (banc) ; le drop-in d'enveloppe pour l'hôte (page 03 d'optimiseur : cinq pools du moteur à 32 fils,
  le poids protège l'écran) ; soixante copies `check-lib.conf.avant-*` à effacer.
- Faute de c7 : une heure de section écrite sans la lire (« 17 h 5x » à 17 h 44), corrigée.

## 18 h 0x — la mesure a parlé : le lexical fait ×8 sur une base à versions

- **La mesure unique d'ED** (R = la même base avant 1b, A = à versions ; 10 requêtes × 5 tours ;
  durées par nœud lues sur NodeRun ; arrêtée de l'extérieur à 18 h 03 min 47 s pendant B, cause
  demandée à G) : sans filtre **965 ms → 7 032 ms**, dont le nœud lexical **874 → 6 928 ms** ; par
  chemin (12,4 %) 247 → 2 154 ; par langage (8,1 %) 229 → 1 371 : **le filtre réduit sans contourner**.
  Les autres étages pèsent pareil des deux côtés (resolve ~50 ms, crochet links 30 → 47, fusion ~0).
  Le drapeau de la clause ajouterait 1,2 s (racines par la clause 1 198 ms contre 19) : la forme de
  repli par (couche, rang) est acquise. Migration 18 ms. Le dense à 0 ms en R est à vérifier (R n'est
  référence que pour le lexical).
- **La piste** : le masque des versions passe à lucivy un `allowed_ids` de 72 652 uuid **à chaque
  requête**, puis sa résolution en offsets. C'est la question de Lucie du matin (« lucivy elle-même
  peut pas permettre ces vues filtrées ? ») : **une vue lucivy nommée**, l'ensemble admis posé une
  fois par (couche, rang) à la publication (bitset côté lucivy), citée par son nom, invalidée à la
  génération suivante. ED départage l'intérieur du nœud ; lucivy mesure `allowed_ids` à 72 652
  contre une vue préposée ; banc tient la parité des réponses.
- **Le poste libéré** : la chaîne de la pile reprend — recherche (CI) → cœur (juge) → tty-7 (sur
  27cfe33a8) → mineurs → clients (archi) → garde-pile (G) → import pointé (codeparsers) →
  exclusions (recherche) → activations (embarquement, vrai jeu sur 7971/7891 en parallèle) → 1b
  (arbre principal) → fils du moteur (G) ; chacun dit sa tête au suivant. La fenêtre de 96 (journal
  des chantiers) s'ouvre avec G. Hotfixs joue le témoin des arêtes en double au portable.
- **Décidé** : l'ancien index (`Scope_Chunk`, `Scope_CHUNKED_FROM`, `Lucivy_Scope` 4,0 Gio contre
  1,2 pour `Lucivy_ScopeContent`) reste après 1b pour le retour, plus lu par le produit ; un pas
  « retrait » à part, après la bascule et une validation (durée à Lucie), rend ~5 Go par base et
  emporte les 2 205 arêtes en double. Codeparsers : règles d'import Python en données vertes (302/0).

## 18 h 2x — douze atterris ; le ×8 départagé ; le correctif (a) sur master, la lib d'après en bâti

- **Atterris par la pile** : la ligne pour la CI (recherche, a646bf215 : `try_update`, la CI se rejoue
  dessus) et **le correctif (a) de la perte de Note** (cœur, 373b5477b, juge known_red vert à 18 h 17,
  RefusedCommitRollback 3/3 rouge → 5/5 vert, DdlEveryMinute 3/10 → 0/10 ; SingleWriterCrash n'était pas
  son témoin et le dit). Douze lots, zéro rejeu au dernier pas. **G bâtit la lib d'après à 373b5477b**
  (lancé 18 h 20 min 53 s ; ABI déclarée par le cœur : deux membres privés non virtuels de plus dans
  transaction_manager.h, rien d'autre) et basculera par la procédure sans attendre Lucie, après 96.
- **Le ×8 de la recherche à versions, départagé** : ED, à l'intérieur du nœud lexical — racines 20 ms,
  masque 902 ms, **résolution de `_uuid IN [72 652]` en offsets côté moteur 5 001 ms**, lucivy ~1 s ;
  lucivy, de son côté : son filtre coûte **6 à 10 ms** par requête, mêmes hits (hors de cause, page
  cebda6144) ; arbre principal, par le binaire du produit sur la copie migrée : 7,5 s en bm25, 25 s en
  bm25+vector (réponses justes, 21/24 mêmes premiers). Le masque se calcule deux fois par appel
  (lexical et dense). **Partage** : arbre principal, le court-circuit « sans filtre ni vue, rien à
  masquer » (lot à part, témoin qui borne les lignes lues, **avant 1b** ; la bascule des vraies bases
  attend) ; ED, la fuite de RSS et la vue publiée par (couche, rang) pour le cas filtré ou sous vue ;
  un ticket moteur pour les 5 s de l'`IN` ; banc, la parité ; lucivy, pas de vue maintenant.
- **Un second défaut** : la mesure d'ED n'a pas été arrêtée par quelqu'un — l'oom-killer de sa portée
  (`POSTE_MEM_MAX=32G`) a tué la sonde à 33 Gio de RSS après ~250 search_code sur la base à versions
  (la référence a tenu). Pente par appel à mesurer ; ticket à la courbe ; pas de 40G avant de savoir.
- **La chaîne** : tty-7 (cce8970af, à toi à son vert) → mineurs (c14a389c4) → clients (a4936c7a3) →
  garde-pile-2 (40f6b63a5, relue par recherche ; « un dossier de journaux neuf par passe » en
  procédure) → import pointé (9b4d82175, passe en cours) → exclusions → activations → sans-filtre →
  1b → fils du moteur (rouge « 24 » lu, vert lu). Mémoire fermée 18 h 20 → ~18 h 50 (96, journal des
  chantiers, instantané avant-chantiers-14-nov-2).

## 18 h 3x — les quatre mots de Lucie ; tty-7 atterri ; le profil léger tranché

- **Lucie** : la chaîne Rust → « oui épinglons en une version, à voir si on prend la dernière en date
  d'abord » (lot à G : rust-toolchain.toml + CI ; 1.98.1 maintenant ou la dernière stable, coût des deux
  demandé) ; Laya → « oui bien sûr, faut expérimenter, et pas seulement pour le rappel : aussi pour
  choisir une fiche ou en créer une nouvelle » (lot à banc, sur les décisions de l'archiviste) ; le
  drop-in de l'hôte → « ça l'a pas refait depuis un moment, possible que c'était les e2e qui
  recompilaient tout » (pas posé, la page reste prête) ; les soixante copies → « oui les supprimer,
  voire un mécanisme pour que ça n'arrive plus » (G : effacer, et la sauvegarde hors du dossier
  d'unités).
- **tty-7 atterri** (0de3884b9, 8/8 en `=`, treize lots par la pile) : le journal de séance et le repli
  sous mosh. Archi a « à toi » pour les mineurs puis les clients, à condition que le SIGSEGV à la sortie
  du binaire de lib (après 89/0) soit un rouge d'environnement connu — il rejoue trois fois.
- **Le profil léger (lucivy)** : `positions:false` tient le budget (4,39 → 1,44 Go, −67 %) mais pas
  search_code (×7 à ×16 sur les requêtes courtes, ×150 sur quatre mots, l'archiviste 180 ms → 63 s) :
  **on garde les positions** ; le budget vient des versions partagées (ScopeContent 1,2 Gio contre 4,0)
  et des exclusions ; ticket lucivy pour le chemin sans positions. Les fils sous quota : 5 à 9
  d'ordonnanceur, jusqu'à 19 `docstore-compressor` brefs, 26 au plus.
- **Les arêtes en double** : le témoin d'hotfixs est **vert sous e93710f36 aussi** — l'hypothèse de la
  clé primaire est affaiblie, pas réfutée ; il lit la forme exacte des clés ratées dans le ticket du
  cœur et instrumente le chemin COPY avant d'élargir. Sous 2a′-1, une édition efface encore (les
  orphelines existent sur une base vivante ; 2a′-2 passe au retrait par `_jusqu_a`) : le court-circuit
  d'arbre principal garde son comptage des orphelines ; on ne retire pas une version à l'édition.

## 18 h 4x — la passe du journal posée ; une fenêtre trouée ; les décisions du soir

- **Le journal des chantiers est dans la mémoire de Lucie** (96, 18 h 20 → 18 h 40, 152 sections, 0 erreur) :
  57 fiches proposées (20 thématiques, 37 à un chantier), 8 sections rejointes sur 5 fiches existantes,
  0 refus, miroir 173 = 173 ; posée à 18 h 43, rouverte par G. **La fenêtre a été trouée à 18 h 35** :
  Lucie essayait le terminal (run agent-1a1db20d720-9091, tâche essai-tty), son juge a écrit par l'hôte
  7970 (une Source, une fiche, deux réécritures) et **tout appel rouvre un backend fermé**. Rien de
  perdu : l'état vivant copié dans `ecrit-pendant-la-fenetre-14-nov/`, écrasé par la passe, les
  écritures du juge rejouées. **Une annonce n'est pas un verrou** : ticket et lot à archi — `/backend/close`
  avec une raison refuse toute réouverture en la disant jusqu'à `/backend/open`.
- **Lucie** : la chaîne Rust épinglée à **1.98.1**, posée quand la pile est vide ce soir (G, lot prêt
  d950d8790 : rust-toolchain.toml + workflow ; luciepc 1.98.0 rebâtira ses 17 targets actifs au
  premier cargo) ; la montée à 1.100.0 en lot de nuit (ticket 4d85569f5, ~340 Gio rebâtis). Laya : à
  banc, sur les décisions de l'archiviste et la porte du rappel. Drop-in de l'hôte : pas posé. Les
  copies : 67 + 63 effacées, l'installeur et bascule-pose sauvegardent hors du dossier d'unités
  (bc3adc7f3, témoin rouge d'abord, hors pile à bon droit : `--code-apporte` vide) ; un
  `check-lib.conf.pret` du 25 oct. reste, hors du mot.
- **tty-8** au mot de Lucie (le terminal « trop prototype ») : prose en couleur par défaut, Markdown en
  ligne, un éditeur de ligne complet.
- **Codeparsers** : la passe de l'import pointé rougissait sur l'environnement (la lib 0c41f9fe6 plus
  vieille que le `src/` de la pile, qui porte le correctif (a) ; postgres arrêté au portable) : rejouée
  sous `RAG3WEAVER_MOTEUR_ANCIEN=1` dit (le lot ne touche pas le moteur) et `docker start rag3weaver-pg`.
  La lib d'après (373b5477b) couvrira la pile à sa pose.

## 19 h 0x — la chaîne complète jusqu'au rang 18 ; un core mal rattaché

- **La chaîne** après tty-7 : mineurs (archi, c14a389c4 ; un SIGSEGV à la sortie du binaire de lib après
  89/0, rejoué vert seul deux fois ; navigateur en fond, pushs à son vert) → clients (a4936c7a3) →
  garde-pile-2 (40f6b63a5, vert) → import pointé (9b4d82175, vert, MOTEUR_ANCIEN dit) → **tty-8**
  (696cbf5a3, quinze commits : éditeur de ligne, prose en couleur du terminal, Markdown, l'AltGr perdu
  depuis tty-6 ; rejeu en cours) → exclusions (bb0cea68d, `workspace.index_exclude`, passe en cours)
  → activations (a7d39e4e6 ; vrai jeu parti 18 h 46 sur 7971/7891) → sans-filtre (arbre principal) →
  1b → fils du moteur → chaîne Rust 1.98.1. Codeparsers : règles Rust en données (305/0), en-têtes de
  provenance, 18 Go rendus, un seul target.
- **Faute de c7** : j'ai rattaché le core d'archi (2513521) au core du dictionnaire du cœur sur la seule
  signature SEGV_ACCERR ; hotfixs l'a lu : le fil mort est `index-suivi:56b`, un fil **rag3weaver** non
  joint à la sortie du processus. Ticket à part (produit, Pour : archi), l'occurrence sortie du ticket
  du dictionnaire. Règle en mémoire : le fil, le module, le moment, pas le signal.
- **96** : les gestes du survol conçus (confirmer par la transition `accept`, scinder avec apports
  déplacés et l'ancienne retirée, rejoindre avec une Ref `alias` ; pas de champ neuf), rouge de (1) en
  cours ; M8 chez Lucie après ce lot, par fenêtre et instantané, une fois le verrou de fenêtre de l'hôte
  atterri (ticket docs/memoire/tickets/2026-11-14-un-backend-ferme-par-l-hote-se-rouvre…, Pour : archi).
- **Hotfixs** : le premier témoin des arêtes en double était hors de la fenêtre du raté (les clés déjà
  reprises ; le raté du cœur vise les clés qu'un point de reprise déplace vers le disque) ; second cas
  en jeu (liens neufs reversés pendant des CHECKPOINT enchaînés), rouge attendu sous e93710f36.
- **Banc** : les 79 662 `_uuid` de Scope communs à R et à la base migrée ; parité des réponses en cours.

## 19 h 1x — dix-sept atterris ; la recherche à versions revient à 12 s pour 24 requêtes

- **Le court-circuit du masque** (arbre principal, « sans filtre ni vue et sans version orpheline, rien à
  masquer ») : sur la copie migrée, le jeu de 24 requêtes passe de **406 s à 12 s** (0,3 à 0,8 s par
  requête en bm25 comme en bm25+vector, contre 7,5 et 25 s), **réponses identiques 24/24**. Piles posées :
  masque-sans-filtre-pile b0ecb3bb3 puis migration-1b-pile 0ed318ad7 (139 suites), passe complète en file.
  Reste pour la bascule des vraies bases : la forme filtrée (×8,7 par chemin → la vue par (couche, rang)
  d'ED) et la fuite de RSS.
- **Atterris** : les quatre mineurs (d4f0b5a74) et les cinq clients avec délai (69b2f5474) d'archi ;
  garde-pile-2 (8565a06bf : le mode pile sur master, posé au cache des deux postes, f9521fb3) ;
  l'import Python pointé (b45056943). Dix-sept lots par la pile, zéro rejeu au dernier pas.
- **Le groupe à la demande** prouvé en vrai (embarquement, 18 h 46 → 19 h 06, hôte isolé 7971,
  fournisseur 7891) : rien à l'ouverture, activé pendant l'appel, relecture dans le même processus, baux
  puis éviction à 60 s, réactivation en 6 s ; **2 Gio de marge au pic sur card2** → seuil 13 Gio libres
  en mémoire ; l'index 1 à 904 s (bge-m3 dense + creux, carte pleine) à mesurer à part.
- **Les arêtes en double** : l'hypothèse du raté de la clé primaire **réfutée** par hotfixs (le second
  témoin vert sous e93710f36 ; le plan de la relecture des paires est SCAN + HASH_JOIN, pas l'index) ;
  piste déterministe (sept vagues identiques = une arête par réindexation) : réindexer trois fois un
  fichier inchangé. **Le core d'archi** : un fil `index-suivi` de rag3weaver non joint à la sortie
  (ticket 7aac92bfb, Pour : archi, lanceur `suivre_le_premier_index`), pas le dictionnaire — faute de c7
  corrigée.
- **Banc** : parité R / base migrée à 86 % des dix premiers, scores qui bougent (admis) ; un soupçon
  « même contenu, autre occurrence » à qualifier par le hash du contenu (ticket pour arbre principal
  si confirmé). **Archi** : la fermeture qui tient (rouge posé ; une fermeture tient avec une raison,
  seul `/backend/open` la lève). **Codeparsers** : règles d'import rebasées (308/0), Go en cours.
- Luciepc saturé (huit lourds pour quatre places) : le verdict de la lib 373b5477b attend sa place, la
  bascule viendra plus tard dans la soirée.

## 19 h 3x — dix-huit atterris ; un vrai rouge sur master à départager ; la purge des orphelins chiffrée

- **tty-8 atterri** (b3f58c30e, 15/15 ; la CI de master revenue au vert). Dix-huit lots par la pile.
- **Un vrai rouge sur master**, vu par archi dans la passe de la fermeture qui tient (son lot est vert) :
  `e2e_demon_hote::une_base_a_rouvrir_ne_lache_que_son_backend` (point de reprise qu'on fait échouer, puis
  la réouverture doit relire le validé) rouge sous 0c41f9fe6 + MOTEUR_ANCIEN=1, joué seul, vert avant ce
  soir. ED prouve par le code que 2a′-1 n'y est pour rien (manifeste carnet, `generations = None`).
  Suspects : le miroir de 96 (`stop_reactors` régénère depuis la base à la fermeture), le plafond de
  recherche (read_pool : renouvellement, pool plein qui refuse), le correctif (a) du cœur. Archi joue le
  test à 42c84cb77, c350dc9c3, a646bf215 ; 96 et recherche lisent leur code. **Rien ne se pousse sur un
  rouge non compris** ; les lots dont les passes ne jouent pas cette suite continuent.
- **Les orphelins du plein texte** (hotfixs, sur copie) : 8 809 Scope et 124 File effacés en 921 ms ; ils
  remontaient en masse dans les recherches (« commit » : 8 651 hits dont 5 822 orphelins ; « return »
  33 930 → 25 518) et pesaient **2,78 Go, 68 % de la table pour 10 % des documents** (78 Ko par orphelin,
  les minifiés réindexés à chaque rebâti) ; compactage après purge 4,27 → 1,30 Go, mêmes hits. Sur les
  vraies bases par la règle de Lucie, en fenêtre tenue dès que la fermeture qui tient d'archi est sur
  master ; le correctif de `DeleteRecordNode` à recherche.
- **ED** : l'`IN` du moteur est un balayage (SCAN + LIST_CONTAINS, 5 008 ms) là où `UNWIND … MATCH {_uuid}`
  prend la clé (113 ms) : ticket moteur pour le planificateur, la réécriture à recherche ; le moteur ne
  retient rien (plateau sur 30 tours) ; **le dense à versions prend ~20 s et plusieurs Gio par appel**
  (16 → 29 Gio en six appels), suspect `compile_filter_for_vector_in` : ED départage.
- **Banc** : le soupçon « même contenu, autre occurrence » réfuté par le hash ; un ticket pour les copies
  d'un même contenu dans les dix premiers (29 places sur 289 sur la base migrée) ; parité au bit près du
  court-circuit en file. **Codeparsers** : toutes les règles d'import en données (311/0). **96** : la trace
  d'archive pour Laya après confirmer_fiche.

## 19 h 5x — la chaîne complète jusqu'au rang 22 ; le goulot de luciepc levé ; le terminal à réessayer

- **La chaîne est posée de bout en bout** : exclusions (bb0cea68d) → activations (a7d39e4e6) →
  sans-filtre (b0ecb3bb3) → 1b (0ed318ad7) → fils du moteur (9b26b27cb) → chaîne Rust 1.98.1 (G) →
  pointeur des règles d'import (codeparsers, 28ed5f4b3 : toutes les langues en données, `toml` au lock,
  ticket de la casse Go). Chaque passe en file ou en cours ; les pushs à leur vert, dans l'ordre.
- **Le goulot** : le concurrence_test DdlEveryMinute **sous ASan** du cœur (sa chasse du dictionnaire,
  lancé 18 h 22, 40 min pour un tour) tenait une place et la mesure du dense d'ED attendait derrière en
  bloquant la pile ; arrêté par le cœur à ma demande (le dictionnaire sous ASan la nuit, poste libre).
  **La lib 373b5477b** : e2e_code vert sous elle (36/36), e2e_lectures en cours, bascule ensuite.
- **Le rouge de l'hôte** : toujours à départager — 96 (contre-essai avec/sans régénération) et la
  bissection d'archi (2a1f52c21..42c84cb77) ; recherche hors de cause par lecture (Surveillant en tests
  seuls, plafond sans copie) ; le message exact : « 1 référence à l'instance précédente tenue après 10 s ».
- **tty** a rejoué l'essai de Lucie contre l'hôte 7970 avec tty-8 : couleurs, saisie, repli sous mosh
  marchent ; page pour elle `docs/tty/14-11-2026/04-l-essai-en-une-minute.md`. Cinq défauts du terminal
  → tty-9. **Un défaut de l'équipier** : à « un titre, une liste, un mot en gras », il répond par des
  comptes rendus et des step_done, jamais par le contenu → ticket pour archi.
- **Hotfixs** : le script de la fenêtre de purge passe sur copie (9 701 Scope orphelins dans l'instantané
  de 13 h 40, 4,59 → 1,32 Go) avec le contrôle de rang corrigé (« les 20 premiers d'avant dans les 100
  d'après »). **Lucivy** : dictionnaire par segment 2 % seulement, les 19/20 sont des égalités à la coupe.
  **ED** : les trois tickets des mesures sur master (127bcae937), le premier dit « 1b n'atterrit sur aucune
  vraie base tant que ce ticket n'est pas fermé ».

## 19 h 5x — la fuite du dense chiffrée ; la pile se repose d'un cran ; le rouge de l'hôte date du 13

- **La fuite du dense** (ED, poste mesure seul) : le dense seul, base à versions, sans filtre : **8,4 Gio au
  premier appel puis ~3,7 Gio par appel, linéaire**, tué à 33,2 Gio au sixième ; la requête `IN` seule est
  plate (+20 Mio sur 30 tours) : c'est la recherche vectorielle filtrée (`compile_filter_for_vector_in`,
  le HNSW sous un filtre de 72 652 uuid) ou ce qui garde son résultat ; ~11 s des 17 s par appel. La forme
  filtrée la garde aussi : le court-circuit ne la couvre pas. **Prochaine mesure, qui décide de l'urgence :
  la même série sur R** (base d'avant) — si R fuit, les hôtes de Lucie fuient aujourd'hui. Puis la requête
  vectorielle seule, N fois ; le cœur prévenu. Le plan de la forme par clé lu : UNWIND → HASH_JOIN sur
  `_uuid` (au ticket moteur).
- **La pile se repose d'un cran** : le lot des exclusions de recherche (bb0cea68d) casse le jeu
  sans-feature (`exclusion.rs:45` appelle codeparsers sans `cfg(feature = "code")`, E0433), vu par
  codeparsers au rang 22 et non corrigé par lui (pas son code). Recherche ajoute un commit (cfg et repli),
  embarquement → arbre principal ×2 → G → codeparsers se reposent et rejouent leur court.
- **Le rouge de l'hôte date du 13** (dernier vert 8 h 01 sous 29a7bc52a) : 96 et recherche hors de cause ;
  archi départage lib contre Rust par deux jeux croisés ; **e2e_demon_hote entre dans le verdict d'une lib**
  (procédure de5ab41df, G ; le verdict de 373b5477b = trois suites).
- **Le cœur** prend l'`IN` sur la clé primaire en deux lots : (B) un ensemble haché pour un `IN` sur liste
  constante (le N² de LIST_CONTAINS disparaît), puis (A) l'accès par clé au planificateur ; témoin C++ sans
  rag3weaver. Le dictionnaire sous ASan la nuit (son banc arrêté, qui tenait une place depuis 18 h 22).
- **Tty** : l'essai de Lucie rejoué avec tty-8, page « l'essai en une minute » (3c9f3fa44) ; cinq défauts
  → tty-9 ; **l'équipier rend des comptes rendus au lieu du contenu** → ticket pour archi (f9fb0b8d2).

## 20 h 2x — le rouge de l'hôte compris ; les hôtes de Lucie ne fuient pas ; deux rouges de base au dernier maillon

- **Le rouge de l'hôte, bissecté par archi et lu par arbre principal** : 1e9bf5089e / 2f6c34643 (raccord
  physique du 13) font attendre la réouverture sur `Weak<Database>` **et** `Weak<ReopenState>` ; le
  crochet de test `FailedCheckpointHook` tient un `Arc<ReopenState>` pour toute la durée du test, donc
  après le point de reprise échoué la réouverture attend 10 s et refuse (« 1 référence tenue »). Aucun
  chemin produit ; correctif = `Weak` + `upgrade` dans le crochet, en lot à part en bout de chaîne ; la
  règle du 13 reste. G avait lu la l. 280 comme une perte de donnée validée et suspendu la bascule : la
  l. 280 est l'assertion de surface, l'aide `fiche()` avale l'erreur ; les jeux croisés avec le jeu
  instrumenté d'archi sous les deux libs disent le reste ; le verdict de la lib tiendra avec ce rouge nommé.
- **R est plat** (ED : base d'avant les versions, dense connecté, +5 Mio par appel, 284 ms médiane) : **les
  hôtes de Lucie ne fuient pas aujourd'hui** ; la fuite (3,7 Gio par appel) est propre au chemin filtré
  des versions — un Cypher d'un Mo en clair, `PROJECT_GRAPH_CYPHER`, `QUERY_VECTOR_INDEX`, `DROP` ; la
  sonde isolée (V1 cycle entier, V2 projection + retrait seuls, V3 sans filtre) départage moteur et
  rag3weaver.
- **Deux rouges de base d'affilée au dernier maillon** (codeparsers, rang 22, jeu sans-feature) :
  `exclusion.rs:45` sans `cfg(feature = "code")` (recherche, commit ajouté f63d1bf4e : sans parseur, pas de
  règle « fichier de code ») puis `catalog.rs:2802` deux méthodes mortes sans feature (1b, 23bf41b2c,
  commit ajouté en cours). La chaîne au-dessus se repose à chaque fois. **Règle (codeparsers)** : chaque
  maillon joue check_features avant de donner sa tête. tty-9 (quatre commits, cinq défauts de l'essai)
  inséré après les exclusions, vert.
- **Lucivy** : page et deux tickets (5ddd33d51 ; le chemin sans positions, un ulp d'écart en parse) ; PR #35
  des tests de mesure ; l'API de maintenance (`maintain` : purge d'ids, compactage, rapport ;
  `live_node_ids`) en cours pour le script de fenêtre. **Le cœur** : (B) l'ensemble haché pour un `IN`
  sur liste constante, puis (A) le planificateur ; le dictionnaire sous ASan la nuit.

## 20 h 4x — le verdict de la lib tient, la bascule part ; la fuite tient à la taille du Cypher ; dix-neuf atterris

- **373b5477b** : e2e_code 36/36, e2e_lectures 27/27, e2e_demon_hote 12/13 avec le seul rouge connu (le
  jeu instrumenté d'archi dit mot pour mot le même texte sous les deux libs : Rust du 13, ticket). ABI
  sans symbole changé, lib aux deux postes, binaire 4b43c3afc prêt. **Fenêtre de bascule donnée à G**
  après la recopie de la garde : annonce, instantanés, deux sens, binaire, `courant` et `lib-des-passes`
  aux deux postes, réveil, témoins ; la lib épinglée des passes devient 373b5477b au prochain lot.
- **La fuite tient à la taille du Cypher projeté** (ED) : R filtré par chemin ou langage projette aussi
  un graphe par requête (un Cypher court) et **ne fuit pas** (+5 Mio par appel) ; **les hôtes de Lucie ne
  sont pas touchés aujourd'hui, même en dense filtré**. La projection d'un Cypher de 2,9 Mo (72 652 uuid
  en clair) puis son retrait fuit 3,58 Gio par tour sans requête : variante (b) du cœur, l'analyse ou un
  cache de la requête. Règle rag3weaver en mémoire : jamais une grande liste en clair dans un Cypher ;
  la forme = un graphe projeté posé une fois par (couche, rang) et cité par nom (la vue de 2a′-2).
- **Le cœur** : (B) vert — un ensemble haché pour un `IN` sur liste constante, le rapport IN/UNWIND de
  97 à 0,69 (2 010 → 15 ms), 73 e2e verts, rang 27 ; il attaque la fuite de la projection (cas C++ à
  quatre variantes).
- **La garde** : garde-pile-3 (G, relue par recherche, 33 verts) ajoute `PILE_TESTE` — une tête déjà
  reposée à la main après un conflit de `docs/nouveautes.md` (tout le monde écrit en tête du même
  fichier : à repenser), posée au cache des deux postes (f9521fb3 → d9a6c083), rang 28 pour master.
  Embarquement pousse avec, depuis 8105b7f02.
- **La pile** : dix-neuf atterris (tty-9 373794303 : la garde a remis le worktree au premier refus, en
  vrai). Chaîne : activations (prêt) → sans-filtre → 1b → fils du moteur (vert) → pointeur des règles →
  fermeture qui tient (7c0cb6f6c, check_features 8/8 avant la tête) → crochet en `Weak` → réécriture du
  `IN` → gestes de 96 (confirmer_fiche vert) → (B) du cœur → garde-pile-3 → chaîne Rust.
- **Tty** : README du crate (96e41c072) ; **96** : confirmer_fiche vert, un ticket sur les ForEachNode
  imbriqués ; **lucivy** : PR #35, l'API de maintenance en cours.

## 20 h 5x — la bascule en cours ; vingt atterris ; la vue en prédicat, sans cache

- **La bascule vers 373b5477b est partie à 20 h 52** (G) : hôtes arrêtés aux deux postes, instantanés,
  contrôle des deux sens, binaire 4b43c3afc, `courant` et `lib-des-passes` ; réveil annoncé vers 21 h 10
  (portable) et 21 h 25 (luciepc). Les passes épinglent leur lib par son nom ; les prochains lots
  épinglent 373b5477b (mémoire à jour).
- **Vingt atterris** : les activations nommées et le groupe à la demande (441f28e08), poussés avec
  `PILE_TESTE` après un conflit de `docs/nouveautes.md` reposé à la main — garde-pile-3 (G, relue par
  recherche, 33 verts) posée au cache des deux postes (d9a6c083) en douze minutes ; le fichier des
  nouveautés, où tout le monde écrit en tête, est à repenser. Chaîne : sans-filtre et 1b (arbre principal,
  reposés sur 441f28e08, passe sous 373b5477b) → fils du moteur (vert) → pointeur des règles → fermeture
  qui tient (rejeu) → gestes de 96 (vert, rang 24) → crochet en `Weak` (vert sur master, e2e_demon_hote
  13/13, rang 25) → réécriture du `IN` (26) → (B) du cœur (27, vert, le rapport IN/UNWIND de 97 à 0,69)
  → garde-pile-3 (28) → chaîne Rust 1.98.1.
- **La vue de 2a′-2** (ED, page 05) : le masque devient un **prédicat** de quelques centaines d'octets
  (une version est admise si une de ses occurrences est visible), pas une liste — et ED a vu avant
  d'écrire qu'un ensemble posé une fois par (couche, rang) mentirait sous 2a′-1, le rang ne bougeant pas
  pendant l'ingest : **sans cache pour ce lot**, une requête courte par recherche (lexical et dense), le
  cache viendra avec l'écrivain de 2a′-2 ; témoins : plus de résolution par liste, aucune projection de
  plus de 4 Kio, `inline_params` qui refuse une grande liste ; parité de banc, RSS plat.
- **Le cœur** sur le pic de 3,6 Gio : pas une rétention — la liaison d'une liste littérale est copiée
  par évaluateur, le pic est rendu, la pente d'un appel à l'autre vient des arènes glibc ; ticket
  reformulé et gardé ; G mesurera `MALLOC_ARENA_MAX` sur un hôte isolé. **Banc** : parité du
  court-circuit verte au bit près (30/30), et le passage aux générations ne change aucune réponse.
- **Embarquement** : le dense porté sur `Tenue<C>` (remaniement à comportement égal, dit sans rouge
  inventé), la mesure des 904 s à 3 h, card2 sans autre charge. **Mes propres pushs de docs** ont fait
  refuser un push de pile au dernier instant (c4242ab52) : la garde a remis le worktree, le second essai
  est passé ; je groupe mes docs quand un push est en vol.

## 21 h 1x — le portable réveillé sous 373b5477b ; la chaîne verte jusqu'au rang 24

- **La bascule** : hôtes arrêtés à 20 h 52 aux deux postes, instantanés `avant-373b5477b`, contrôle des
  deux sens (plus long que les 15 min de la procédure : 20 min par base) ; **le portable est réveillé à
  21 h 13** sous le binaire 03ab44a0 lié à 373b5477b, cinq backends rouverts sans refus, search_code,
  recall_fiches et l'IDE répondent ; luciepc suit (code-principal puis code-lourd), puis `courant` aux
  deux postes ensemble.
- **La chaîne** : sans-filtre et 1b (arbre principal, passe complète sous 373b5477b en cours) → fils du
  moteur (G, 1e7926159, check_features après la fenêtre) → pointeur des règles (codeparsers, vert, pas de
  rejeu : rien de code entre be641a3c5 et 1e7926159) → fermeture qui tient (archi, prêt) → gestes de 96
  (eeefcc493, rejeu) → crochet en `Weak` (arbre principal, d1578cd56, e2e_demon_hote 14/14 sous
  373b5477b : le rouge se ferme là) → réécriture du `IN` (recherche, base d1578cd56) → (B) l'`IN` haché
  (cœur) → garde-pile-3 (G) → dense sur `Tenue<C>` (embarquement, vert sous 373b5477b) → chaîne Rust.
- **Mesures du plein texte** : un index neuf du dépôt pèse 771 Mo (4,59 Go ce matin) par le correctif des
  octets d'ED, 754 avec les exclusions ; le dictionnaire garde 94 Mo de textes morts après purge et pèse
  420 Mo pour 171 de texte même neuf (lucivy : la fusion lâchera les ids sans document vivant, puis une
  compaction filtrée du dictionnaire ; conception validée). Lucivy main : l'API de maintenance
  (`maintain`, `live_node_ids`, garde à la moitié), PR #35 et #36 ; **4.3.8 posé à Lucie**.
- **Laya** (banc) : protocole sur master, modèle déjà à luciepc, jeu synthétique de 286 cas, CPU d'abord.
  **Hotfixs** : le pas « retrait de l'ancien index » en préparation sur copie. **ED** : le masque en
  prédicat écrit, trois témoins rouges puis verts, mesures en file.

## 21 h 2x — un écart à luciepc dans la bascule ; les deux index d'une table de relations ne s'accordent plus

- **Le contrôle des deux sens** sur l'instantané de **code-lourd** (état sous 0c41f9fe6) trouve un écart :
  INHERITS_FROM 650/658 (1 d'un seul côté, 11 vues deux fois, 13 aux bouts différents), QUERIES 20 093
  (5/6/9), LOCKS 544/546 (0/3/5) ; dix-neuf tables égales ; le portable sans écart. **Décision de c7 :
  (b)**, la bascule continue sous 373b5477b dès que code-principal est contrôlé — l'écart est antérieur à
  la lib neuve, petit, et de la famille que le correctif (a) vise ; revenir luciepc seul laisserait deux
  libs. Nuance écrite dans la procédure : « un écart antérieur, petit, ticketé et de la famille corrigée ne
  bloque pas ; la base se répare dans la fenêtre suivante ».
- **La lecture d'hotfixs** change la nature du défaut : des relations d'identifiant 0 « vues deux fois »
  (des cases à zéro dans un des deux index, souvent depuis le nœud 11:0), des identifiants consécutifs
  (29:357 → 360) aux sources décalées d'un sens à l'autre — **les deux index d'une table de relations ne
  s'accordent plus**, un décalage de CSR plutôt qu'une arête perdue. Une réparation par Cypher n'est pas
  sûre ; hotfixs reconstruit les trois tables **sur copie** (relevé des deux sens, DROP/CREATE au même
  schéma, repose des arêtes d'accord, les ~24 en désaccord redites par le code, contrôle égal sur 22
  tables). Ticket moteur pour le cœur, après la fuite de la projection.
- **G** : son contrôle des deux sens tournait hors du verrou (charge 13 à 17 pendant la mesure d'ED) ;
  il passe sous `poste lourd` dans la procédure. **ED** : le prédicat ÷6 dense compris, filtré ×1,3 à ×1,74
  de R, sans-filtre au court-circuit ; mémoire plate à 20 Gio (MALLOC_ARENA_MAX=2 en mesure), B à l'OOM :
  deux tickets (c3f2ebd6b), rang 30. **Lucivy** : le dictionnaire est le vocabulaire (5,6 M entrées, 46 M
  clés de suffixes), pas les segments ; les leviers changent ce que `contains` trouve — décision à Lucie,
  pas ce soir ; la pièce 1 (la fusion lâche les ids morts) écrite. **Hotfixs** : le retrait de l'ancien
  index répété vert sur copie (4,28 Go rendus, 8/8 identiques ; un DROP ne rend pas ses pages au disque :
  ticket souhait) ; le piège du manifeste de copie qui pointe hors de la copie, en tête de sa page.

## 21 h 3x — 1b et le court-circuit sur master ; code-principal diverge aussi, la bascule continue

- **Arbre principal a poussé 194a4e572** : le court-circuit du masque (sans filtre ni vue ni orpheline, la
  recherche à versions ne lit que les k résultats ; 406 s → 12 s sur 24 requêtes) **et 1b**, la migration
  par lots bornés avec un représentant par version, `rag3weaver-backend migration versions --base
  <chemin>` hors hôte sous un embarqueur qui refuse tout appel — 11 commits, un seul push, 11/11 en `=`,
  passe complète sous 373b5477b (lib 2398/0, dix suites e2e sans sauté). Vingt-deux lots par la pile.
  La bascule des vraies bases vers les versions attend encore : la mémoire du dense à versions (le
  prédicat d'ED, rang 30) et les deux défauts de la vue.
- **La bascule de la lib** : code-principal diverge aussi au contrôle des deux sens (QUERIES 7/8, IMPLEMENTS
  3/8/9 et 6 `line` à deux valeurs selon le sens, LOCKS 0/12/9 et 16 `line`), même date que code-lourd
  (14 h 25, sous 0c41f9fe6). Décision (b) maintenue pour les deux : `courant` et `lib-des-passes` aux deux
  postes ensemble, réveil de luciepc sous 373b5477b ; un seul ticket, deux bases, Pour : hotfixs, qui
  reconstruit les six tables sur copie (relevé des deux sens, DROP/CREATE, repose des arêtes d'accord, le
  reste redit par le code). La piste du cœur : sous l'ancienne lib, l'annulation d'un COMMIT refusé
  tronquait chaque index séparément — la famille que 373b5477b ferme.
- **La chaîne** : G (fils du moteur) → codeparsers → archi (fermeture) → 96 (gestes) → crochet → `IN` →
  (B) → garde-pile-3 → dense sur `Tenue<C>` → prédicat → chaîne Rust 1.98.1.

## 21 h 4x — vingt-six atterris ; la lib courante aux deux postes ; la mémoire des recherches plafonnée

- **Atterris depuis 21 h 30** : le court-circuit et 1b (194a4e572, un push de 11 commits), les fils du
  moteur (77b9e5f18), le pointeur des règles d'import (7215d18ad0 : sept langues en données, plus aucune
  liste dans le code), la fermeture qui tient (1cbf1cf3a), confirmer_fiche (10b7a0edb). Vingt-six lots par
  la pile, zéro rejeu au dernier pas ; deux pushs refusés au premier essai par des docs arrivées entre
  fetch et push, remis par la garde et repassés. Le crochet en `Weak` pousse (il ferme le rouge de l'hôte).
- **La bascule** : `courant` et `lib-des-passes` à 373b5477b aux deux postes à 21 h 33, luciepc réveillé à
  21 h 33 (code-lourd et code-principal rouverts, search_code ok) ; page 04 de la pose ; les deux écarts en
  un ticket pour hotfixs (reconstruction des six tables sur copie) ; `controle-des-deux-sens.sh` passe sous
  `poste lourd` ; la nuance au § 8.
- **La mémoire des recherches à versions** : `MALLOC_ARENA_MAX=2` plafonne le RSS à **9,8 Gio** au lieu de
  20,6 après 50 recherches, sans coût (ED) ; la pente venait des arènes de glibc. Proposition écrite par G
  dans sa page 03 (la ligne dans `rag3weaver-daemon.env`, que la pose écrit déjà, aucune unité touchée) ;
  mesure sur l'hôte réel cette nuit, puis la question à Lucie avec les deux chiffres.
- **Le cœur** : le pliage des listes littérales vert (le pic de 3 524 → 192 Mio : un évaluateur et un
  ValueVector de 2 048 places par élément), au rang 27 avec l'`IN` haché ; un rouge antérieur et
  indépendant (`DropNodeTableReclaim`) en ticket ; **`e2e_test` sans filtre dit « PASSED 0 tests »** — en
  mémoire. **Lucivy** : un second défaut trouvé par la pièce 1 — la compaction ne réécrivait jamais un shard
  réduit à un seul segment, ses supprimés y restaient pour toujours ; corrigé, passe en cours. **Archi** :
  le fil `index-suivi` joint à la fin de l'hôte (le SIGSEGV à la sortie expliqué), passe en cours.
  **Codeparsers** : le plan Go validé (tables par langue, `go.mod` lu), C# par la documentation.

## 21 h 5x — vingt-sept atterris, le rouge de l'hôte fermé ; les sources suivies conçues

- **Le crochet en `Weak`** (a76f36a08d) ferme le rouge de l'hôte qui dormait depuis le 13 ; vingt-sept lots
  par la pile. Recherche pousse la réécriture du `IN` (rouge 1,10 s → vert 0,28 s pour 30 000 uuids), puis le
  cœur (l'`IN` haché et le pliage des listes, deux pushs), G (garde-pile-3), embarquement (le dense sur
  `Tenue<C>`, vrai jeu vert sur 7892), ED (le prédicat, parité au bit près 30/30 par banc), G (chaîne Rust).
- **Les sources suivies** (arbre principal) : conception courte validée — le champ `tracked` (root
  computed/empty/declared ; sources à sélecteur préfixé branch/refs/worktree/commit/tasks avec parent,
  history, work), le défaut = le worktree du backend et les tâches, l'ancienne clé lue, la résolution cachée
  par le hash des refs, le flux multi-env nommé, le dépôt id par le plus vieux commit ; témoins un par ligne
  du tableau de la page 02. Il déclare, résout et dit ; l'indexation par objets git vient après.
- **Hotfixs** : les six tables des deux bases de luciepc relevées et jugées (le sens avant est le juste),
  reconstruction finale sur copies sous le lourd ; les identifiants sont des positions CSR, pas des dates ;
  un instantané du 13 à 20 h 36 dira depuis quand. **Banc** : la parité du prédicat verte 30/30 ; Laya
  synthétique rejoué à question fixe (22 % en zéro-coup, hasard 17 %). **96** : la trace d'archive commence
  (resté au repos après son push, dit tel quel).

## 21 h 5x — vingt-huit atterris ; MALLOC_ARENA_MAX à l'hôte réel ne confirme pas la sonde

- **Le `IN`** de recherche est sur master (9fd946ec3, 30 000 uuids 1,10 s → 0,28 s, tout en « = »). Le cœur
  a le « à toi » pour ses deux pushs du rang 27 (a56af398d puis 9d90edbb6, suites C++ rejouées sur les
  têtes reposées) ; puis G, embarquement, ED (parité 30/30 au bit près : prêt au rang 30), G.
- **Arbre principal** : deux formes de la résolution changées à la relecture d'ED et validées — l'identifiant
  d'une couche est le nom complet de la ref (clés courtes relues et réécrites avec témoin ; renommage =
  une couche finit, une commence) ; la couche de travail est au worktree, pas à la branche (détaché =
  génération épinglée sur son commit). Garde demandée : relire une clé courte ne perd rien quand une
  étiquette homonyme existe.
- **G, MALLOC_ARENA_MAX=2 sur l'hôte réel** (binaire 10b7a0edb, 373b5477b, hôte à part 7996, 50
  search_code) : RSS 6 005 → 5 083 Mio au 50e (−0,9 Gio, −15 %), pente ÷4, mais médiane 460 → 793 ms
  (+70 %), la série « avec » jouée en second sur un poste à load 24. La paire inverse tourne (13 min). Les
  20,6 → 9,8 Gio d'ED venaient d'un hôte qui projette 2,9 Mo de Cypher ; celui de master court-circuite.
  Rien à Lucie avant les deux paires ; si la lenteur suit le réglage, la ligne reste une note.

## 22 h 2x — MALLOC_ARENA_MAX=2 : la lenteur suit le réglage, la ligne reste une note

- **G, paire inverse faite** (hôte à part 7996, binaire 10b7a0edb, 373b5477b, 50 search_code) : dans les deux
  ordres, moins d'1 Gio gagné au 50e appel (6 005 → 5 083 et 6 044 → 5 028 Mio, −16 %) et des appels 70 à
  77 % plus lents (460 → 793 ms, 436 → 771 ms). Le poste n'y est pour rien (load 13 et 6). **La ligne
  n'entre pas dans l'env** ; la page 03 de G (bdec3dd4f) garde les deux paires et la conclusion « sous un hôte
  qui projette, à mesurer ». Rien à Lucie. Le ticket de la fuite dense d'ED reste ouvert ; l'hôte de mesure
  est arrêté, la place mesure rendue.
- **La pile** : le cœur rejoue ses suites C++ sur ses têtes reposées (a56af398d, 9d90edbb6) ; derrière,
  recherche (orphelins, 27 ter), archi (index-suivi, 27 quater), G (garde-pile-3, 28), codeparsers
  (provenance par langue + imports Go par tree-sitter, 28 bis), embarquement (dense), ED (prédicat), G
  (chaîne Rust). Banc : le `IN` au bit près 30/30, les trois optimisations du soir jugées identiques.
- **Arbre principal** (sources suivies) : déclaration, résolution, racine, parente, flux, fonctionnalité
  finie, identifiant du dépôt et migration des clés — tous rouges puis verts (10 tests). Décisions dans la
  page 02. Reste : cache, branchement (le `_couche` des générations se décide avec ED), témoins, passe.
- **Correction d'ED à 22 h 2x** : l'écart entre sa série (20,6 → 9,8 Gio) et celle de G n'était pas la
  projection du Cypher de 2,9 Mo — sa sonde tournait sur vue-publiee, masque déjà en prédicat, **sans le
  court-circuit** d'arbre principal : le prédicat rejouait la jointure sur 72 652 versions à chaque appel sans
  filtre. Page 03 de G corrigée (777413893). La conclusion tient : réglage non posé. Reste ouvert : le cas
  filtré sur une base à versions, plat vers 20 Gio dans la sonde d'ED, à mesurer sur l'hôte réel après
  vue-publiee.
- **Hotfixs, 22 h 3x** : une première reconstruction des deux bases de luciepc donnait « égales » au contrôle
  de G, mais `usages`/`impact` par le binaire avant et après montraient une relation vraie perdue
  (lockIndexForWrite → acquireLock, l. 1173). Cause : le relevé — le sien comme l'outil de G — tirait ses
  sources d'une seule lecture non ancrée, dans un seul sens. Corrigé (tranches d'offsets ancrées par
  étiquette, sens prouvé par EXPLAIN) ; la chaîne rejoue sur copies neuves. Le reste tenait : lignes des impl
  justes à 284cc00c0, fantômes des .md d'offset 0 disparus, héritages mal rattachés tombés. **« Aucun écart »
  ne prouve pas qu'il ne manque rien** : demandé un compte par table dans le relevé et le ticket.
- **ED, 2a′-2 conçu** (page 06, 987bdd49e) : deux rangs vivants par couche (n publié, w = n+1 en attente,
  la couche de travail) ; l'écrivain ligne par ligne à six cas (réécrire ce qui est né à w, retirer par
  `_jusqu_a = w`, ligne neuve à uuid hash(clé, couche, w), effacer au retour à l'identique) ; publication
  au commit par une ligne `_generation` puis n := w ; cache à clé (couche, rang, époque si w, filtre,
  cellule), borné, qui ferme côté lexical l'état B ; dense en post-filtre (k′ = 4k, repli à la projection
  courte). Tranché : arbre sale → `arbre_propre: false` sur la génération avec le nombre de chemins sales,
  2a′-3 rend le propre exact ; ordre : mesure des déplacés (50 commits de code-lourd), mémoire du filtré,
  écrivain, cache, post-filtre dense mesuré avec parité et rappel en chiffres.
- **Codeparsers, 22 h 4x** : le 28 bis fait trois commits (d7832ad7d, replace de go.mod compris). Relevé des
  trous des sept langues contre les manifestes : Python (nom importé ≠ nom distribué : PyYAML → yaml,
  Pillow → PIL… ; requirements PEP 508 `@` et `-e` perdus, requirements/ non reconnu ; setup.py, setup.cfg,
  Pipfile, PEP 735 non lus), JS/TS (alias `#…`, `paths`, `@/` comptés inconnue ou paquet), C# (aucun
  .csproj lu, espace de noms comparé au paquet entier), Go (go.work), C/C++ (conan, CMake ; tiret des ports
  vcpkg), partout (une dépendance d'une autre langue ne compte plus — à dire dans les nouveautés). Ordre
  donné : requirements, renommages Python en données + champ `imports` déclaré du manifeste, .csproj +
  préfixe C#, alias de résolution, go.work, conan/CMake ; un lot par trou, rouge d'abord.
- **lucivy, pièce 1 verte** (dictionary-compaction) : rouge sur main (un segment seul gardait ses
  200 supprimés), vert (réécrit en un tour, .gmap 631 ids = l'index neuf, hits et scores au bit près). Coût à
  luciepc sous le lourd : indexation +0,6 à +1,8 % (sous le bruit), compaction +12 % (le pré-passage qui
  relit les postings ; 17,7 Mo rendus). Décision : garder tel quel, PR, fusion à la CI verte, pas de tag ;
  la réduction (a) en ticket souhait ; pièce 2 ensuite.
- **ED, la mesure des déplacés** (sonde sur les 50 derniers commits de master, arêtes lues sur une copie
  de R) : 44 849 arêtes recopiées, 86 % par les seuls déplacés ; médiane 0 sur les 50 (docs), 1 500 parmi les
  commits de code, maximum 12 444 (cbb529c7e). À la lettre du seuil d'arbre principal, la position reste
  dans l'identité. Tranché : **elle reste pour 2a′-2**, avec le compte des recopies à chaque publication dans
  l'état et 20 000 comme seuil de réouverture ; **elle sort de l'identité avec 2a′-3**, en une seule
  migration avec celle des objets git — la mesure en est la justification écrite.

## 22 h 5x — vingt-neuf atterris ; archi a le « à toi »

- **Le cœur a poussé ses deux** : l'`IN` haché (e5e626803) et le pliage des listes littérales (1d5580d74), tout
  en « = », C++ seul. Archi (index-suivi, 58b7a8eab sur 9d90edbb6) a le « à toi » ; puis G (garde-pile-3
  79d15a5fb, check-rel-ancre 0f9a149ed), codeparsers (498799f34, cinq commits), 96 (trace, base donnée),
  embarquement (dense), ED (prédicat), arbre principal (suivies, verdict de la passe à 23 h 19), G (chaîne
  Rust). Les orphelins de recherche sont sortis de la file (témoin rouge, 6 documents pour 5 Scope ; parité
  de banc verte au bit près, comptes à relire par les colonnes de version).
- **Décisions de l'heure** : la position reste dans l'identité pour 2a′-2 et en sort avec 2a′-3 (mesure d'ED :
  86 % des recopies pour un numéro de ligne) ; l'arbre sale au commit par `arbre_propre: false` ; la pièce 1
  de lucivy gardée telle quelle (+12 % sur la compaction, 17,7 Mo rendus) ; la table de pipreqs pour les
  renommages Python avec un champ `imports` déclarable ; l'outil des deux sens joue sur une copie jetable de
  la base réelle, hors de tout jeu automatique.
- **Hotfixs, 23 h 3x** : les deux reconstructions vertes sur copie avec le relevé ancré, comptes au ticket
  (677b5297a) — code-principal QUERIES 7 → 0, IMPLEMENTS 6 → 0 (3 copies exactes ôtées), LOCKS 11 → 0 ;
  code-lourd INHERITS_FROM 9 → 0, QUERIES 5 → 0, LOCKS 2 → 0 ; contrôle de G égal, relevé ancré égal ;
  usages/impact par le binaire confirment (acquireLock garde lockIndexForWrite). Le portable complet et
  égal sur toutes ses tables. La réparation attend un hôte de luciepc avec la fermeture qui tient : G dit
  quand il pose un binaire ; sinon demain matin.
- **Banc, 23 h 3x** : le trou de Symbol comblé (page 02 § 4, e635ffee8) : après le retrait d'arbre.rs, 3 Symbol
  (relation_label, le_libelle_est_en_casse_de_phrase, la_forme_du_graphe_de_dependances) restent sans aucune
  relation — rendez-vous par nom, hors session. Ticket pour ED : rétention sans règle ; la règle va dans
  l'écrivain de 2a′-2 (fermé à la publication quand plus rien ne le relie, rouvert si le nom revient), rien à
  toucher sous la forme sans générations.
- **G, 23 h 3x** : le binaire 10b7a0edb (porte 1cbf1cf3a, bâti contre 373b5477b, md5 ffb6d2e6, essai 7991
  vert aux deux postes) se pose sur l'hôte de luciepc vers 1 h 15 : verdict à trois suites lancé à 23 h 30
  (en file derrière 4/4 lourds), annonce aux sessions au vert, pose quand aucune passe ne lit l'hôte. La
  fenêtre d'hotfixs suit. Le portable attend.
- **La fenêtre de luciepc (vers 1 h 15, hotfixs)** : une seule fermeture et un seul instantané par base pour
  deux gestes — la réparation des deux index (reparer-les-deux-sens.sh) puis la purge des orphelins du plein
  texte (fenetre-purge.sh ; code-lourd 103 Scope, code-principal 102) ; code-lourd puis code-principal ;
  relevé ancré jugé entre les deux (un écart arrête la base, RETOUR.sh, pas de purge) ; les trois gardes de la
  purge sur la base réparée ; banc de recherche par l'hôte avant et après ; comptes aux deux tickets.
- **Le cœur, 23 h 3x** : RefusedCommitRelDirections dit déjà — avant le correctif (f41c0ba5b) la relation
  validée de B est perdue à chaque tour, **dans les deux sens** ; après (7c92562b1) vert au tour 1. La fenêtre
  corrigée en 373b5477b perdait des relations validées, mais le témoin ne reproduit pas encore la forme d'un
  seul côté de code-lourd : le cœur cherche l'asymétrie (une troncature dans un sens seul) avant de conclure
  à la même cause. Son témoin retiré de la file de luciepc pour laisser passer 96.
- **G, 23 h 35** : le verdict de 10b7a0edb ne suffit pas (e2e_demon_hote 13/14 : le rouge connu de la l. 280,
  le crochet en Weak dc5bc9853 a atterri après lui) ; G bâtit le binaire de la tête de master (porte
  dc5bc9853 et 1cbf1cf3a), essai aux deux postes, verdict à trois suites, 14/14 attendu ; pose vers 1 h.
- **lucivy, pièce 2 verte** (PR #38, 943ada6) : sur la copie réelle de Lucivy_Scope, purge des 8 809 orphelins
  puis compaction : dictionnaire 513,8 → 400,7 Mo, textes 8,15 M → 6,31 M, **index entier 4,27 → 1,16 Go**
  (1,30 avec la pièce 1 seule), sous l'index neuf d'hier (420 Mo de dictionnaire, hypothèse : plusieurs
  générations). Témoin textes 631 = 631, octets +2,25 à +3,65 % (marge 5 % justifiée dans le test), hits et
  scores au bit près ; coût dans le bruit. Fusion à la CI verte ; **4.3.8 attend le mot de Lucie** pour que
  rag3weaver pointe dessus et que la compaction joue sur les vraies bases.
- **Le cœur, 23 h 3x** : RefusedCommitRelDirections rouge 2/2 sur f41c0ba5b, vert 3/3 sur 7c92562b1 (15 à 25 s
  par tour) ; au ticket 94c431abf8. Établi : sous une lib d'avant 373b5477b, la fenêtre d'annulation perd des
  relations validées (des deux côtés). Non établi : la forme d'un seul côté et les « vues deux fois » ; prochain
  témoin = une troncature d'un seul sens (point de reprise entre les deux versements d'abord). Le témoin entre
  avec le prochain, en un lot.
- **Le cœur, 23 h 3x** : la piste du point de reprise est exclue par le code (un écrivain refusé reste ACTIF
  jusqu'à son annulation, le point de reprise attend tout écrivain actif) ; reste **un refus entre le versement
  du sens avant et celui du sens arrière** (RelTable::commit : l'enregistrement d'annulation ne couvre que le
  premier sens → le sens avant tronqué avec la relation de B, le sens arrière la garde — la forme de
  code-lourd, sous pool plein). Validé : un point de faute de test dans le moteur (TestDiePoint, sans effet en
  service) qui fait lever après le sens avant ; témoin rouge sur f41c0ba5b, vert sur master, plus l'assertion
  que l'annulation sous le verrou rend les deux sens.

## 23 h 4x — trente-sept atterris ; la pile reposée jusqu'à la garde des sous-modules

- **Atterris depuis 23 h** : index-suivi (archi), garde-pile-3 et l'outil des deux sens ancré (G), la provenance
  par langue (codeparsers), la trace (96), le dense sur `Tenue<C>` (embarquement), le prédicat (ED, 28 commits),
  les sources suivies (arbre principal, 19 commits). Chaque fois « = » et « pile tenue », identité gmail,
  aucune attribution. **Reposés et en rejeu** : le dual du creux (870bb4a5e, « à toi » à son vert), les
  renommages Python (4a10a5939), la chaîne Rust 1.98.1 (b1990e6e2) et la garde des sous-modules (8a2b85f81,
  40 à 60 min de bâti à froid). Derrière : les orphelins (recherche, cause trouvée : aucun FlushNode après
  DeleteRecordNode), l'équipier (archi), .NET + using C# (codeparsers), la base postgres par passe (G), la
  troncature d'un sens (cœur).
- **Trois défauts d'outillage trouvés par la pile elle-même**, tous en ticket pour G : la garde du target ne
  voit pas un sous-module qui change de commit (corrigé, en pile) ; deux passes partagent la base postgres de
  test (une base par passe, en branche) ; controle-du-worktree.sh compte le retard en hashes et refuse une
  base de pile non atterrie (RETARD_MAX=300 dit pour la passe de codeparsers ; --cherry-pick à écrire).
- **Les mesures de la soirée** : lucivy pièce 2 (index 4,27 → 1,16 Go sur la copie réelle, 4.3.8 attend
  Lucie) ; les déplacés d'ED (86 % des recopies, position gardée pour 2a′-2, sortie avec 2a′-3) ;
  MALLOC_ARENA_MAX écarté ; les trois parités du soir et celle de la pile d'ED (avec orphelines) au bit près.
- **La fenêtre de luciepc** vers 1 h : G pose un binaire de la tête de master (verdict à trois suites en
  cours), hotfixs répare les deux index puis purge les orphelins du plein texte, une fermeture et un
  instantané par base, relevé ancré jugé entre les deux.
- **G, 23 h 5x — l'inventaire des scripts du cache** (ticket bd175de35) : `poste` au cache en retard sur master
  aux deux postes (sans le `nproc --all` de garde-pile-2 : un lourd sous quota étranger calcule ses parts sur
  le quota) → recopie validée, test_poste.sh d'abord ; `passe_epinglee.sh` dans aucun dépôt et différent d'un
  poste à l'autre (57 appelants à luciepc, cité par push-d-un-lot.md) → entre le premier dans tools/poste/,
  diff des deux versions, base luciepc ; puis lib_seule.sh et rag3db_libs.py. Le contrôle du worktree recopié
  (sha 94d5c09b6c85b737, témoin 2 vert).
- **Hotfixs, 23 h 55** : la répétition de reparer-les-deux-sens.sh sur copie neuve de code-lourd verte de bout
  en bout (23 h 35 → 23 h 55 : instantané, relevé ancré, jugement, reconstruction, contrôle de G égal, relevé
  ancré après à 0 d'un seul sens sur les trois tables) ; le contrôle de G prend ~20 min par base : compter une
  heure de fenêtre pour les deux bases avec la purge et les bancs. Prêt pour le « posé » de G.
