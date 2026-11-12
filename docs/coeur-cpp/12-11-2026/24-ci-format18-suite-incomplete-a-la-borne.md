# CI format18 : suite interrompue à sa borne

Source : `74ad9c25cd2a63e01abd54f25314992cda6fea60` ; correction de préparation publiée dans `6628fc388`. Aucun changement source, de cible, de drapeaux ou de budget après ce delta.

Le second bâti privé réussit les quatre cibles en 188,291 secondes. Le paquet conservé avant les tests contient 834 fichiers ; leurs tailles et SHA256 ont été revérifiés après arrêt, sans écart. Il reste privé sur luciepc : `~/.cache/rag3db-moteur-notes/ci-format18-74ad-02/package/`. Aucune bibliothèque distribuée n’est posée.

La suite complète sélectionne 359 tests de 72 suites, puis atteint sa borne propre de 540 secondes pendant `VectorCopyUnderASmallPoolTest.TheIndexReadsItsVectorsWhileTheHeadIsExchanged`. Aucun marqueur GTest FAILED n’apparaît, mais aucun XML final n’est produit : résultat **incomplet, non vert**, sans verdict individuel sur le test interrompu. Les lignes RUN/OK de sous-processus ne permettent pas de calculer un total final fiable. Le saut affiché `VectorIndexCrashReopenProbe.OpenInAFreshProcess` est conservé dans le journal, sans transformer la suite inachevée en succès.

Les cinq tests BlobCopyCheckpointTest terminent en 306,721 secondes cumulées. Cette durée explique une part importante du budget consommé ; elle ne prouve ni régression du formatage ni blocage du dernier test. Il faudrait une recette distincte revue pour poursuivre le diagnostic ; aucune augmentation de borne ni relance n’a été faite.

L’unité termine en échec contrôlé : MainPID=0, cgroup vide, aucun processus correspondant restant. Durée seconde unité 731,945 secondes ; première unité 161,855 secondes ; cumul 893,800 secondes inférieur aux 2400 autorisées. Pic mémoire 5795106816 octets, swap nul. Scratch privé conservé pour inspection, sans processus actif. Aucun service, GPU, modèle ou PG touché.

Les [preuves de l’arrêt02](preuves/ci-format18/arret-02/) gardent exception, journal intégral, recette effective, environnement, résultat du bâti et manifeste du paquet. Le premier arrêt EROFS reste documenté séparément. Ticket ouvert : ni transaction_test vert, ni CI globale verte, ni publication master autorisée. Le lot attend revue C7 ; aucun nouveau runtime lancé.
