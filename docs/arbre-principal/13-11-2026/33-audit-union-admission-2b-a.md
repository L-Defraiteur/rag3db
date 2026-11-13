# Union de l’admission A sur la garde intégrée

Base master fraîche `9feb7cdbadef6a60568dd4fc447bafddf7986942`, branche neuve `cx-arbre-couches-2b-union-r1`, WT privé `cx-wt-couches-2b-union`. Aucun changement de l’arbre principal de Lucie ni des ressources Recherche.

Application portée `06e2086f8` : trois cherry-picks sans conflit de 54142/917463/affa6. Le range-diff joint donne trois `=`. Les neuf fichiers produit du delta A sont identiques mode/OID à `affa6d03b5a57dd99810b6853783dc1a267f76b9` ; les trois chemins documentaires A également. Hors ces douze chemins, 8074 entrées ou absences de l’univers master/base sont préservées intégralement. Aucun delta de garde, daemon_host, run_e2e, gabarit, lock, gitlink ou SDK ajouté. Audit mécanisé : `2b-a-union-audit.json`, `2b-a-union-range-diff.txt`. Les nouveaux rapports/audits de ce commit sont documentaires et extérieurs aux douze chemins portés.

Preuves historiques A CPU `33a64da41c0d68a4fd0653c71167fc8887e41f7d` déclarées suffisantes par C7 : 15 exacts, lib1662/0/6, 315 fichiers manifestés identiques deux postes, 187,03025 s sur1800. Elles restent rattachées APPaffa6/pilotec75 ; aucun nouveau runtime de cette union n’est prétendu.

Parent et enfant 2b restent ouverts : admission A fixture commune uniquement, B chargeur durable/snapshot composé et C accès outils non qualifiés. Le nouveau couple source/pilote devra recevoir RO puis GO exact C7 avant tout lancement. Ni GO master ni retrait.
