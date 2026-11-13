# Passation cœur C++ — week-end, 13 novembre 2026

## Consigne de reprise

Priorité Lucie : limite d’usage, arrêt des nouveaux calculs. Ce rapport est poussé uniquement sur branche NON master `cx-coeur-passation-weekend`. Aucun nouveau GO/runtime/CI rerun, installation ou nettoyage n’est déduit de la passation. **Les anciens délais et créneaux ne se réactivent pas.** C7 rassemble les rapports ; toute reprise lourde exige une recette/source/budget et une coordination fraîches. Arbre de Lucie `/home/lucied/git_workspaces/rag3db` réservé, ne pas y committer ni nettoyer.

Contact C7 : session `01a1cb5c-62b9-7ea0-8b7e-cf38398ece09`. Identité git Lucie Defraiteur <luciedefraiteur@gmail.com>, messages français sans attribution. Un lot/worktree possédé ; nom de branche neuf après rebase, aucun force/amend poussé. Push source sur sa branche dans la minute. Master code seulement après « à toi », poste push, dernier fetch, if VERT et trois contrôles. Aucun droit master accordé par ce rapport.

## Livré

- Format CI clang-format18 exact Ubuntu24.04/18.1.3-1ubuntu1 : exclusions docs/sous-module puis formatage pur253C++ ; contrôle réel2065sources sans sortie, lexique253sansécart. Intégré master `f903fedba18d7c878f87412b7b1721161bc245a4`, après revue C7. Bâti4cibles réussi sur source publique `74ad9c25cd2a63e01abd54f25314992cda6fea60`. Aucune bibliothèque distribuée posée. Docs22–27 sous docs/coeur-cpp/12-11-2026.
- Étape corrective distincte03 sur le paquet conservé : 357 tests passés, deux outils sautés attendus, un désactivé ;359sélectionnés/360XML, zérofailure/error, suite1069,769s. Cas VectorCopyUnderASmallPoolTest.TheIndexReadsItsVectorsWhileTheHeadIsExchanged completed567,327s. Unité1071,981s ; cumul trois unités1965,781s. Ce cumul ne revendique pas le respect d’une ancienne échéance absolue2400s échue.
- Rapport provenance writer CI D17 et comparaison SDK29 intégré DOCS SEULES master `4e925998d4b9f0f1412bef21ce79ba87807ad63c`. Source CI exacte `d17a230b14f60fbac59aa1e4ac10510ba5f3c7e2`, run43407787602/job131826283645 : archive statique36ece6a62ab092dc575a57879acdef0dc25cd7742725825784532cd7a998bc9b, ELFfbd960f8ffbaf2ea810dd292f22deea7450d3238030615e35dfe048f0d60a1d9 annoncés par l’attestation du job. Rapport docs/coeur-cpp/13-11-2026/01-ci-writer-d17-provenance-et-liberation.md.
- SDK29 identifié29a7bc52aaf11e6204f9beafaffb154105417e13 ; garde writer/mutex/set/deleter textuellement identique à D17. Marque premier membre Database, libérée en dernier ; Connection C++ non propriétaire, partage seulement lifecyclebool. Chemin physique open_explicit_config direct, sans REOPEN_WAIT voie normale. Cela n’attribue pas le détenteur ni la cause CI ; ED/Lucivy étudient acteurs et destruction.
- Docs historiques intégrées master8839f717f9b014bc26b8c84766089e1a86404a64 et revue quarantaine docseulee7590eeab475511d49f2aa1b923ff94781965725.

## Conservé / non livré

Worktrees locaux `/home/lucied/git_workspaces/` ci-dessous, tous relus propres, sans ignorés ; SHA des quatre premiers vérifiés aussi par `git ls-remote --heads origin 'cx-coeur*'` lors de la passation. Origin = dépôt partagé habituel ; ne pas publier ces correctifs privés sur master sans nouvelle revue.

| Worktree / branche | HEAD et branche distante identique | État |
|---|---|---|
| cx-coeur-annulation | 0511dece74f94db6937fb03e6ec45f0c58828c5a / origin/cx-coeur-annulation | Correctif privé rollback pool plein, non intégré |
| cx-coeur-diagnostic-saturation | 35ea70d3a4a5cc4917586de02f6bfc862c76a02c / origin/cx-coeur-diagnostic-saturation | Instrumentation privée, non intégrée |
| cx-coeur-revue-quarantaine | ccb01fc1ee91b2a761ede70152a1a0d9589d49fc / origin/cx-coeur-revue-quarantaine | Revue/proofs seules déjà publiées, conserver historique |
| cx-coeur-suivi-11nov, branche cx-coeur-suivi-11nov-5 | 34f8e9203a36c3696ffabd42b92df08d0635533a / origin/cx-coeur-suivi-11nov-5 | Historique docs, derrière master, ne pas réutiliser sans tri |

Branches historiques locales sans worktree : cx-coeur-ci-format18 = f8529a79564dadfe99b1ea4561bf049a5c158d2e ; cx-coeur-ci-writer-ro = db92f3e5b2ef4a37fcd0d8d408259e1fe56eacb8. Distantes correspondantes vérifiées identiques. Branches distantes d’intégration gardées : origin/cx-coeur-ci-format18-integration-2 = f903fedba18d7c878f87412b7b1721161bc245a4 ; origin/cx-coeur-ci-writer-docs-2 = 4e925998d4b9f0f1412bef21ce79ba87807ad63c ; leurs worktrees et branches locales d’intégration ont été retirés après garde. Autres anciennes distantes cx-coeur-suivi-11nov/-3/-4 et publications docs gardées, sans WIP nouveau. Cette passation possède son propre worktree cx-coeur-passation-weekend, conservé après push pour collecte C7.

Sur luciepc : seul worktree attribué listé par l’inventaire `git worktree list` du dépôt rag3db-colonnes : `/home/lucied/git_workspaces/cx-coeur-archive-0511`, détaché0511dece74f94db6937fb03e6ec45f0c58828c5a, relu propre. Les autres sources natives des unités étaient dans des tmpfs privés disparus à l’arrêt ; aucun worktree02/03 vivant n’est prétendu. Archive0511 à préserver, aucun cleanup implicite.

## Tickets et résultats à ne pas repeindre

1. `docs/coeur-cpp/tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md` : ouvert. Source privée0511. Ciblés6/6×3 et stockage8/8 verts ; ancienne suite transaction359joués/357verts/2outils sautés,1336,857s. Concurrence424cas/25rouges/3206,038s :16known_red,5probabilistic,4horslistes décrits au ticket. Leur présence historique ne prouve pas l’indépendance du correctif. Pas de qualification globale verte.
2. Diagnostic privé35ea : DdlEveryMinute1/1vert en987,526s,984échantillons, aucun premier refus observé. BufferManagerpeak1120Mio contre2051historique rouge ; causalité non établie, aucune optimisationA validée. PKcalls0 jusqu’au dernier échantillon984,064s seulement, pas de métrique terminale extrapolée.
3. `docs/coeur-cpp-hotfixs-tickets/tickets/2026-11-12-verifier-la-quarantaine-d-une-annulation-partielle-pour-les-transactions-deja-ouvertes.md` : ouvert, porteur hotfixs. Témoin privé sur0511 prouve TAinactive/locks0 et refus publicBEGIN/CHECKPOINT ; COMMITTB déjà ouverte atteint factorysentinelle avant Checkpointer. Aucun checkpoint réel/reopen/corruption exécuté. commit/publish peut précéder cette factory : ne pas conclure TB sans publication. **Reprise de la trace privée bloquée par revue automatique interdite : aucun contournement, même sous un autre nom.** Revue doc21, sources privées non intégrées.
4. `docs/coeur-cpp-hotfixs-tickets/tickets/2026-10-25-la-ci-de-master-est-rouge-depuis-le-18-octobre-cassert-dans-geo-et-format-clang.md` : ouvert jusqu’aux vrais jobs CI futurs verts. EROFS01 runner après bâti,0test,161,855s ; timeout02 suite540s sansXMLfinal, unité731,945s : garder ces rouges.03 vert distinct ne réécrit pas01/02. cassert déjà corrigé avant lot, ne pas refaire. CI shell historique séparée, aucun vert global inventé.
5. `docs/orchestration/tickets/2026-11-13-la-ci-refuse-la-reouverture-du-temoin-physique.md` : ouvert, ED/C7/Lucivy pour détentions acteurs ; cœur C++ provenance terminéeRO. CI2257passés/1échec/66ignorés sur D17 ; alreadyopenwriter àtests.rs414 après dropcreate_new, aucune cause retenue encore. Ne pas désactiver garde, ajouter sleep arbitraire ou rouvrir en concurrent. Aucun SDK/build/test/DB/service/rerun lancé pour ce diagnostic.
6. `docs/arbre-principal/tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md` : porteur arbre. RO fournie sur DEFAULTNULL explicite non typé : defaultExpr parsé persisté, replay rebind sans cast ; omission via primitive dialecte produitNULLtypéSTRING. Ne pas confondre avec nouveau refus writerD17. Aucun correctif C++/repro livré par cette session.

NON JOUÉS : nouvelle suite native après intégration format (Cpp/flags inchangés, exemption C7), runtime diagnostic writer/SDK29, nouvelle comparaison causale saturation, checkpoint réel de la trace quarantaine, nouvelles passes GPU/services/PG. Aucun budget ancien valide à reprendre.

## Preuves stables et ressources

- Dépôt : docs/coeur-cpp/12-11-2026/preuves/ci-format18/ et rapports22–27 ; docs/coeur-cpp/13-11-2026/preuves/{ci-writer-d17-extraits.log,sdk29-garde-comparee.json}.
- luciepc : `~/.cache/rag3db-moteur-notes/ci-format18-74ad-02/package/`834fichiers, manifeste et logs ; répertoire01/02/03preuves conservés sous moteur-notes. Archive arrêt01 SHA5289fbeeb15f873a3347711ab5b7419d69be808e9daad6f968427bf4764161cd, copie locale sous ci-format18-a783. Aucun package distribué installé.
- luciepc : `~/.cache/rag3db-moteur-notes/annulation/` journaux/XML historiques ; `annulation/cx-build-saturation-12nov-02/package/` paquet35ea,834fichiers110723932octets ; `annulation/cx-archive-privee-12nov-02/` archive0511.
- Reçus locaux cleanup hors worktree : `~/.cache/rag3db-moteur-notes/ci-format18-publication/`, `ci-writer-docs-4e925998d/`, `publication-docs-8839f717f/`, `publication-quarantaine-e7590eeab/`. Archives administratives fidèles/ref/reflog et rapports gardés. Dernière garde utilisée writer SHA6f3dfceb17c9703029d2f4f26385f9aa21820c84bc79c518e1338b6d0f36188c ; relire garde actuelle à la reprise, aucun force ni purge des preuves. Rawtransaction.log conserve sa ligne vide finale, exemption localeC7 tracée et SHA fidèle ; ne pas le normaliser.
- État pendant passation : unitécx-coeur-ci-format18-74ad-03 inactive/MainPID0/cgroupvide ; aucun processus du contrôleur03 ou du binairepackage/transaction_test restant observé. Place lourde/push rendue lors des fins de lot ; aucune réservation CPU/GPU ni service appartenant à cette session conservée. Ne pas arrêter les autres lots/services. Points de montage01/02/03 vides retirés par rmdir seulement. Aucun target compilation actif de cette session ; paquets/preuves hors target conservés.

## Trois prochains pas, après reprise explicite

1. Relire ce rapport, C7 et les tickets, puis inventorier sans mutation : `git fetch origin`; `git worktree list --porcelain`; `git ls-remote --heads origin 'cx-coeur*'`. Dans chaque W conservé : `git status --short --ignored`. Sur luciepc : `systemctl --user show cx-coeur-ci-format18-74ad-03.service -p ActiveState -p MainPID -p ControlGroup`. Ne réactiver aucun ancien délai.
2. Pour le CI writer : relire doc01/D17 et diagnostic ED/Lucivy, relever le propriétaire/fin effective de destruction et la CI la plus récente. Confronter garde29 identique à ces faits ; tout runtime éventuel nécessite source/recette/budget frais publié et créneau C7. CommandeRO utile : `git show d17a230b14f60fbac59aa1e4ac10510ba5f3c7e2:src/main/database.cpp` ; aucun build/rerun automatique.
3. Pour0511/35ea/quarantaine : choisir avec C7 le ticket prioritaire et les preuves manquantes avant toute action. Lire les XML/logs conservés, distinguer privés/nonintégrés/rouges. Ne pas rebaser/publier ces sources privées par commodité, ne pas rejouer la trace interdite. Si un lot est réellement terminé, préparer archive fidèle et garde actuelle avant cleanup, préserver preuves et distantes.

## WIP et limites de cet inventaire

Aucun fichier source non poussé n’a été trouvé dans les quatre worktrees conservés ; toutes leurs têtes sont déjà sur origin. La passation est le seul nouveau WIP, poussé sur branche dédiée. Pas d’inventaire exhaustif du poste d’autrui ni d’affirmation que la CI ultérieure serait verte. Arbre principal aperçu avec données d’autres travaux : laissé intégralement intact. Les preuves historiques citées proviennent des rapports conservés ; seules les vérifications d’état, références distantes et absence des processus nommés ont été relues pour cette passation. Aucun nouveau calcul de qualification.
