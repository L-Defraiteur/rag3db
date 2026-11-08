# Des tests de la lib rougissent sous charge

- **État** : ouvert (7 novembre 2026), pour everything declarative — correctif dans le lot du
  témoin du chat, après les vues 5, 6, 7 et 3b de l'IDE en React.
- **Gravité** : faux rouge (des témoins qui attendent une durée ou tolèrent une perte bornée, et
  que la charge de la lib entière dépasse).
- **Atteignable en service** : non (les témoins) ; ce qu'ils couvrent reste vert seul.
- **Touche rag3weaver** : oui (`src/commande.rs`, `src/daemon_host/taches/tests.rs`,
  `src/daemon_host/chat/tests.rs`, `src/daemon_host/navigateur.rs`).
- **Famille** : produit

## Ce que c'est

Dans la lib entière (1 946 tests en parallèle, 270 s à luciepc, jeu large) ou dans le lot des
témoins navigateur, certains tests rougissent **une fois sur quelques passes**, et passent seuls,
sur master comme sur le lot qui s'y pose. Ils attendent une durée fixe ou tolèrent une perte
bornée, et la charge dépasse la borne.

## Les témoins et leurs chiffres (luciepc, lib 04063ee06 épinglée, 7 novembre 2026)

| témoin | où il rougit | ce qu'il attend | seul |
|---|---|---|---|
| `commande::tests::en_fond_un_journal_au_dela_du_plafond_garde_la_fin_et_le_dit` | la lib entière (1 rouge sur 2 passes) | `fin.octets_stdout` entre 300 000 et 330 000 (`commande.rs:1512`) : la troncature perd « au plus quelques Ko », plus sous charge | 2/2 verts master 2d3ecad1e, 2/2 verts lot |
| `daemon_host::taches::tests::la_base_principale_n_est_jamais_ecrite_par_la_tache` | la lib entière (1 rouge sur 2 passes) | « la tâche n'a pas indexé en 60 s » (`taches/tests.rs:92`) : une durée | 2/2 verts master, 2/2 verts lot |
| `backend::tests::a_failed_batch_is_undone_without_poisoning_the_catalog` | la lib entière, ce poste, 7 nov. 21 h 33 (lib 17a094d12, sources 7cd11e5fa, `MOTEUR_ANCIEN=1`, target rebâti à neuf) | — (vu une fois, cause non lue) | vert seul sous 17a094d12 et sous 7cd11e5fa ; lib entière seule ensuite : 1988 verts |
| `backend::tests::stopping_cuts_a_long_reaction_and_the_restart_resumes_it` | la même passe | l'arrêt d'une réaction longue (voir les tickets fermés du 11 et du 19 octobre) | vert seul sous les deux libs |
| `daemon_host::chat::tests::le_chat_se_mene_et_s_arrete_dans_la_page` | l'étape navigateur, à la première exécution après un bâti | voir [son ticket](2026-11-07-le-temoin-du-chat-dans-la-page-rougit-a-la-premiere-execution-apres-un-bati.md) | 5/7 lot, 5/5 master |
| `le_chat_se_mene_et_s_arrete_dans_la_page`, le tour lent | la lib entière (25 octobre) | voir [le ticket du 25 octobre](2026-10-25-le-temoin-du-chat-rougit-encore-sur-le-tour-lent.md), même famille | — |
| `dataflow::reactor::tests::each_runs_per_ring_and_debounce_waits_for_quiet` | la lib entière, le portable, 8 nov. 9 h 50 (lib dadbf0041, base 0768770d7) | « réveillé 29.979054ms après le dernier envoi » (`reactor.rs:636`) : une durée ; forme attendue, une horloge injectée, comme la sonde de débit (fermée le 8 novembre) ; pour everything declarative | vert dans la lib entière juste avant |
| le témoin des vues de l'IDE | le lot des témoins navigateur | voir [son ticket](2026-10-25-le-temoin-des-vues-de-l-ide-rougit-sous-le-lot.md) ; le harnais attend l'appel en vol depuis bb0f4d9c0 | 2/2 seul |
| `e2e_lectures::le_creux_se_surligne_par_le_chemin_de_l_hote` | e2e_lectures, le portable, 8 nov. 14 h 37 (lib cbc06c3e1, `MOTEUR_ANCIEN=1`, lot ide-symbole-cite) | « le service creux n'a pas répondu en 1000 ms » (`e2e_lectures.rs:1350`) : le tunnel 7979 (le service de luciepc) répondait en 2,1 s, 7980 et 7981 en 0,02 s ; relevé par archi | 24/24 au passage suivant, 7979 à 0,02 s |
| `daemon_host::taches::tests::la_base_principale_n_est_jamais_ecrite_par_la_tache` | la lib entière, luciepc, 8 nov. vers 15 h (lib cbc06c3e1, `MOTEUR_ANCIEN=1`, lot codeparsers/locales-js, charge 33 à 36 sur 24 cœurs, 911 s, 45 tests au-delà de 60 s) | « la tâche n'a pas indexé en 60 s » : une durée (la file de 7878, c7) ; relevé par codeparsers | vert seul, 6,96 s, même binaire, charge 33 |
| `watch::tests::a_quiet_scan_of_the_engine_tree_costs_less_than_a_listing` | la même passe | `left: 1, right: 0` (`watch/tests.rs:341`) ; la montre sur `../..` (7 464 fichiers) démarre en 82 s : une durée | vert seul, 2,60 s, même binaire, charge 32 |

## La règle du jour (c7, 7 novembre 2026)

Une passe où l'un de ces témoins seul rougit se compte en nommant ce ticket, et le témoin se
rejoue **une fois seul** avant de pousser ; un rouge qui tient seul n'est pas la charge, c'est une
régression, et il se donne à la session du lot qui l'a apportée. Pas de `known_red` pour eux.

## Ce qu'il faut pour le fermer

Le même correctif que pour le témoin du chat : chaque témoin attend sa condition **par un
repère** (le journal tronqué, la tâche indexée, le flux ouvert), jamais une durée, et dit les
secondes quand il abandonne ; la borne de perte du journal se mesure sur ce que la commande a
écrit après la dernière troncature, pas sur un total. Rouge d'abord sous charge (la lib entière),
dans le lot du témoin du chat, après la vue 3b.

**Le binaire de la lib mort sans résumé** (ce poste, 7 nov. 21 h 33, la même passe) : après ces deux rouges, le binaire de tests de la lib s'est arrêté sept minutes après son départ, sans ligne `test result` ni panique au journal ; plusieurs tests étaient « running for over 60 seconds ». La lib entière rejouée seule juste après, au même code et sous la même lib : 1988 verts, 26 s. Non reproduit ; à surveiller : un deuxième cas dirait s'il s'agit d'un arrêt du processus (signal, mémoire) plutôt que d'un faux rouge de durée. Ce que le poste dit de cet instant (le portable, pas luciepc : la passe tournait ici) : le journal du noyau, de 21 h 30 à 21 h 36, ne montre ni arrêt par manque de mémoire, ni signal du noyau, ni vidage mémoire du binaire de tests (`journalctl -k`, `coredumpctl`) ; le seul vidage de la fenêtre est `plasma-discover` (SIGABRT, 21 h 32 m 44 s), sans lien. Sur le poste à ce moment : la passe seule sous `poste lourd` (une place), et la copie par `scp` de la lib 7cd11e5fa depuis luciepc (21 h 33). La charge elle-même n'a pas été relevée : le journal de charge de `run_e2e.sh` ne couvre pas l'étape de la lib. Pour le second cas : relever `uptime` et la mémoire libre au départ et à la fin de l'étape de la lib, et le code de sortie de `cargo test` (un signal se lit dans `$?` au-delà de 128).
