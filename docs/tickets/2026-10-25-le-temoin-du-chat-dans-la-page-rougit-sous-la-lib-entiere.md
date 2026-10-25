# Le témoin du chat dans la page rougit sous la lib entière

- **État** : ouvert, le 25 octobre 2026 (vu par la session « recherche », à luciepc).
- **Gravité** : test instable.
- **Atteignable en service** : non (le test).
- **Touche rag3weaver** : oui (`src/daemon_host/chat.rs`, `src/daemon_host/navigateur.rs`).

## Ce que c'est

`daemon_host::chat::tests::le_chat_se_mene_et_s_arrete_dans_la_page` a rougi une fois dans la lib
entière (1757 passés, 1 échec), sur la branche `replis-signaux-2` (`b42418abc`), qui ne touche pas
au chat. La condition attendue n'a pas été vue en 20 s (`Navigateur::attendre`), alors que la vue
montrait déjà « Proposé : une note de l'agent. ».

Seul, il passe trois fois sur trois en 1,5 s ; la lib entière rejouée juste après passe aussi
(1758 passés).

## La recette

La lib entière à luciepc, sous `rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres`. Une
fois sur deux passes vues le 25 octobre 2026.

## Le témoin

Le test lui-même. Le journal du rouge :
`~/.cache/rag3weaver-build/replis-vert/lib.log` à luciepc.

## Pour le fermer

Lire laquelle des quatre parties de la condition manquait au délai : la note « fini », le lien de
la proposition, l'outil `propose` ou le dernier `pre`. L'attente devrait le dire. Puis attendre la
bonne chose, ou borner le délai sur un signal de la page plutôt que sur 20 s.
