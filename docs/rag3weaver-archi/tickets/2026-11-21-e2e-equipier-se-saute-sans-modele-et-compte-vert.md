# e2e_equipier se saute sans modèle et compte vert

- **État** : ouvert, le 21 novembre 2026 ; pour archi
- **Famille** : produit
- **Pour** : archi
- **Gravité** : moyenne. Une suite qui ne joue rien passe pour verte au pousseur, et un lot qui casse l'équipier de
  bout en bout n'est pas vu par elle.
- **Atteignable en service** : non, c'est la batterie de tests
- **Touche rag3weaver** : oui, `tests/e2e_equipier.rs`
- **Signalé par** : mémoire, relayé par c7, le 21 novembre 2026 (« 2 passed » en 0,00 s)

## Ce qui se passe

La suite a deux tests, tous deux `#[ignore]`, tous deux sur un vrai modèle :
`l_equipier_ecrit_la_doc_d_un_depot_de_test_par_pas_juges` et
`l_equipier_rend_le_texte_demande_et_non_un_compte_rendu`. Sans `GOOGLE_CLOUD_PROJECT` ni
`GOOGLE_APPLICATION_CREDENTIALS`, chacun écrit `SAUTÉ <nom> : …` et sort par `return`. Cargo le compte comme passé :
« 2 passed », en 0,00 s. Seule `RAG3WEAVER_EXIGER_GEMINI=1` en ferait un rouge.

Le comportement de l'équipier est par ailleurs tenu sur faux modèle dans la lib (`daemon_host::teammate_tests` et ses
fichiers, `daemon_host::teammate::tests`). C'est là que vivent les témoins de la nuit : la porte de vérification, les
refus répétés, le run d'un pas, l'arbre des agents. Ce qui manque est le bout en bout sur l'hôte réel, avec le gabarit
`memory`.

## Le pas exact qui bloque

Un saut écrit `SAUTÉ` puis `return` : rien ne le distingue d'un vert dans le résumé de cargo, et le juge de la suite
ne lit pas les lignes `SAUTÉ`.

## Ce qu'il faut faire

1. Garder les deux cas sur le vrai modèle, dits « mesure » (ils mesurent le produit sur Gemini), hors de la suite
   jugée au push : `RAG3WEAVER_EXIGER_GEMINI=1` quand ils sont joués exprès.
2. Donner à la suite un cas joué sur le faux modèle (`tests/common`, `fake_sse`), sur l'hôte réel et le gabarit
   `memory` : la boîte, un pas, step_done, la porte de vérification, la fin. C'est lui qui la rend jugeable.
3. Le juge des suites compte une ligne `SAUTÉ` comme « non jugé », jamais comme vert, et le résumé le dit. Cela vaut
   pour toutes les suites.

## Comment le prouver

Rouge d'abord : la suite jouée sans identifiants rend aujourd'hui « 2 passed ». Après le correctif, elle joue le cas
sur faux modèle (au moins un test qui dure plus qu'un saut), et le résumé dit « 2 non jugés » pour les cas sur Gemini.
