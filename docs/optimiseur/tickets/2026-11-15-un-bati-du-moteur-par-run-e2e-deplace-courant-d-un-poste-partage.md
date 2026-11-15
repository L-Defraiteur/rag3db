# Un bâti du moteur par run_e2e.sh déplace `courant` d'un poste partagé

- **État** : ouvert (15 novembre 2026, 4 h 30) ; corrigé sur la branche `publier-sans-courant`, en pile derrière ED.
- **Famille** : produit
- **Gravité** : une passe qui lit `courant` joue une autre lib que celle du poste, sans le savoir.
- **Atteignable en service** : non pour l'hôte (son binaire nomme sa lib par RUNPATH) ; oui pour toute passe sans `RAG3DB_LIB`.
- **Touche rag3weaver** : oui ; `scripts/rag3db_libs.py` (`publier`) et `run_e2e.sh`.
- **Pour** : optimiseur (G).
- **Ouvert le** : 15 novembre 2026

## Ce qui s'est passé

Le 15 novembre à 0 h 53, puis à 1 h 00, la preuve postgres de G (`pg-deux.sh`) a lancé `./run_e2e.sh --build-only` dans
`rag3db-fils-pile-G` à luciepc. Ce bâti du moteur publie la lib au magasin, puis la pose en `courant` : « `courant` la
désigne désormais » (run_e2e.sh). `courant` a donc désigné **9d90edbb6** (le moteur de master, plus neuf que la lib des
passes) de 0 h 53 à 4 h 15. preparer-binaire s'est arrêté sur « courant diffère à luciepc », c'est ainsi que c'est vu.
G l'a remis sur 373b5477b à 4 h 15.

**Ce qui n'a rien vu** : l'hôte de luciepc (son binaire fac67a208 a son RUNPATH sur le dossier nommé 373b5477b), les
outils de la fenêtre d'hotfixs (RUNPATH 373b5477b, ou sans librag3db), toute passe qui nomme sa lib (passe_epinglee,
`RAG3DB_LIB`), lib-des-passes. Le cœur et hotfixs le confirment. Aucun processus vivant ne chargeait 9d90edbb6 à 4 h 20.

## La correction

La règle de `bascule-de-lib.md` § 1 : seule la bascule pose `courant`. `rag3db_libs.py publier` ne le pose plus que sous
`--poser-courant`. `run_e2e.sh --build` joue la lib qu'il vient de bâtir par son identifiant, et laisse `courant`.
Témoin `test_rag3db_libs.py` : sur l'outil de master, deux FAIL (la publication sans option déplace `courant`, et
`--poser-courant` n'existe pas) ; sur la branche, 8 OK, sans saut.
