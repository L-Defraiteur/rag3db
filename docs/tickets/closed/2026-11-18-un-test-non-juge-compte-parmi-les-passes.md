# Un test « non jugé » compte parmi les passés

- **État** : fait (18 novembre 2026, branche not-judged-is-not-passed) : passe_epinglee.sh liste à part les lignes « non jugé : » de chaque journal (« non jugés dans <suite> : N — <raison> », « tests non jugés : N ») ; un non-jugé du témoin ou d'une suite exigée rend la passe rouge ; push_garde.sh refuse une suite exigée dont un test est non jugé ; rouge d'abord aux deux témoins
- **Famille** : produit (outillage : `tools/poste/passe_epinglee.sh`, `tools/poste/push_garde.sh`)
- **Gravité** : faux vert. Un test qui n'a rien jugé passe pour vert.
- **Atteignable en service** : non
- **Touche rag3weaver** : non
- **Pour** : G (optimiseur)
- **Signalé par** : embarquement par c7, 18 novembre 2026

## Ce qu'on voyait

`e2e_lectures::la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel` (`tests/e2e_lectures.rs`, vers la ligne
853) écrit « non jugé : la lib n'a pas la colonne state (format 43 requis), bascule à venir » et rend la main sous
`RAG3WEAVER_MOTEUR_ANCIEN=1`. cargo le compte parmi les 27 passés, et la passe disait « 27 passed » sans rien de plus.

## Le contrat

Un test qui ne peut pas juger écrit une ligne qui commence par `non jugé : ` (en tête de ligne), avec sa raison, puis rend
la main. La passe et la garde lisent cette ligne. Le nom du test n'y est pas aujourd'hui : l'écrire dans la raison aiderait
la lecture.
