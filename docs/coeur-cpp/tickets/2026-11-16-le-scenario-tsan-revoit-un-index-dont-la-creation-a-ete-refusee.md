# Le scénario TSan revoit un index dont la création a été refusée

- **État** : ouvert (16 novembre2026), constat dans une passe existante, oracle ou état à départager
- **Famille** : moteur
- **Gravité** : témoin rouge de catalogue après refus, aucune perte de lignes établie par ce relevé
- **Atteignable en service** : à établir ; observation sous TSan et pool200Mio dans une fixture privée
- **Touche rag3weaver** : indirectement, sémantique du CREATE_VECTOR_INDEX refusé
- **Signalé par** : cœur C++, lecture finale de la passe TSan historique C1
- **Pour** : cœur C++, coordination C7 et hotfixs pour l’état des index

## Faits conservés, sans attribution hâtive

Passe déjà lancée, six tours, aucun nouveau runtime de cette reprise. Source64eee2e1f6c3599f5e4a0bcd3369f6202e33eaca annoncée par le contrôleur ; BUILD_ID59ac9c86bc2b3529b590a7462ab7051e3ad26b8a-RelWithDebInfo-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanTRUE-ubsanOFF réellement présent dans les six stdout. [Bilan final et archive](../16-11-2026/10-c1-tsan-six-tours-termines.md).

Tour2, ddl-2.log : CREATE_VECTOR_INDEX dure2417510ms environ puis est refusé pour bufferpoolplein (pic199Mio). Après checkpoint/réouverture, [check: catalog-as-expected] dit doc_vec true alors que expected false ; note/rang/b présents,a absent,Cite présent. GTest1échec dans ce tour. Les comptes après réouverture sont Doc55414/55414,Link196562/196562,Note24351/24351, pas de perte comptée à cette observation. Les autres cinq tours rendent GTest1passed ; tous les six sortent néanmoins66 à cause des avertissementsTSan.

Source du contrôle : twenty_writers_scenario_test.cpp963–979 compare présence du nom dans SHOW_INDEXES avec ddlNamed(scenario,"4 ")->ok. Or le CREATE en deux transactions peut avoir un holderNOTED aprèsT_A avant un refusT_B. La présence seule peut donc rendre l’oracle obsolète ; l’état exact NOTED/READY, la raison et le contrat doivent être lus avant de qualifier cela de défaut moteur. **Aucun état NOTED n’a été effectivement interrogé dans cette fixture après son exécution**, ne pas le déduire du seul nom.

Un symptôme de même libellé figurait dans le ticket fermé2026-11-14-sous-vingt-ecrivains-et-un-pool-plein-des-note-validees-manquent, dont le défaut principal était une perte de Note. Le présent relevé ne prouve pas sa régression et ne le rouvre pas par coïncidence.

## Suite et fin

Relire les deux phases du CREATE et contrat SHOW_INDEXES ; relier log erreurT_B/état au contrôle de présence. Réduire sous fixture privée si nécessaire, uniquement après recette/admission fraîches ; aucun sleep/retagprobabilistic/affaiblissement d’oracle automatique. Si oracle faux : témoin et contrat d’état explicites ; si vrai défaut : rouge réduit/correctif/qualifications. Ne pas fermer C1crash ni ticketoffsets sur cette observation ; pas de baseLucie/service/SDK/modèle/GPU. Aucun run nouveau demandé ou autorisé par ce ticket.
