# Un test de la lib qui parle au service d'embarquement attend un service réel sans délai à lui

- **État** : ouvert (7 novembre 2026), sans porteur. Relevé par rag3weaver archi pendant la passe du 3 ter (c).
- **Famille** : produit
- **Gravité** : passe figée. La lib entière ne rend pas la main, et un verrou tenu par la passe bloque les autres.
- **Atteignable en service** : non, c'est un défaut des témoins. Il ne se voit qu'en passe, pendant que les services réels redémarrent.
- **Touche rag3weaver** : oui, les témoins de la lib, sous `RAG3WEAVER_EMBED_SERVICE`.

## Ce qui s'est passé

Le 7 novembre, la passe complète du 3 ter (c) a lancé la lib entière à 19 h 58:28. C'était
pendant le réveil des hôtes de la bascule 43, et les passes exportent
`RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7979,7980,7981`, les services d'embarquement réels. À 20 h 41,
le binaire de test tenait encore, avec 1 229 fils, à 6,7 % de CPU. Cinq tests étaient arrêtés
depuis plus de 60 s :

- `daemon_host::taches::tests::une_tache_ouverte_survit_a_l_arret_de_l_hote_et_se_fusionne`
- `daemon_host::taches::tests::la_base_principale_n_est_jamais_ecrite_par_la_tache`
- `daemon_host::tests::deux_fenetres_l_une_enregistre_l_autre_voit_le_changement`
- `dataflow::reactor::tests::a_frozen_reaction_does_not_hold_a_bounded_stop`
- `ide::tests::the_relay_does_not_wait_for_the_graph_runtime`

Un sixième avait rougi : `backend::tests::stopping_cuts_a_long_reaction_and_the_restart_resumes_it`.
La même passe, rejouée à 20 h 43, a fait passer la lib entière en 38 s. Il n'y a pas eu d'autre
reproduction. La pile n'a pas été prise : `eu-stack` n'avait pas le droit de ptrace.

## La piste, non prouvée

Un test qui embarque, par un backend à signal vecteur, a pu attendre un service réel en plein
redémarrage, sans délai qui lui soit propre. C'est le témoin qui a tort, pas l'hôte : un témoin
ne doit pas dépendre de l'état d'un service partagé.

## La règle à poser

Tout test qui touche un service, d'embarquement, un hôte ou un port réel :
- porte un délai borné qui lui est propre ;
- dit « service absent » et se saute, ou rougit en le disant, au lieu d'attendre.
À la prochaine reproduction, prendre les piles des fils arrêtés avant toute hypothèse : `eu-stack`
avec le `LD_PRELOAD` ptracer, comme dans la fiche « profiler sans perf ».
