# Le témoin du chat rougit encore sous la lib entière, sur le tour lent

- **État** : ouvert, le 25 octobre 2026 (vu par la session « recherche », à luciepc).
- **Gravité** : test instable.
- **Atteignable en service** : non (le test).
- **Touche rag3weaver** : oui (`src/daemon_host/chat.rs`, `src/ide/page.html`).

## Ce que c'est

`daemon_host::chat::tests::le_chat_se_mene_et_s_arrete_dans_la_page` a rougi dans la lib entière
sur une base qui porte déjà le correctif `930cccc96` (ticket fermé
`2026-10-25-le-temoin-du-chat-dans-la-page-rougit-sous-la-lib-entiere`), à une autre étape : le
tour lent. La condition « le bouton d'arrêt visible, et `encore` dans la bulle du tour en cours »
n'a pas été vue en 20 s. La vue montrait « l'agent travaille… », et l'appel `/ide/chat` du tour
lent était encore en vol, sans erreur.

Vu sur la branche `ide-signaux` (`0862110ed`, base `f472e75d4`), qui ne touche pas au chat, sous
`rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm`.

## La recette

La lib entière à luciepc sous ce jeu de features. Une fois sur trois passes de la lib vues ce
jour-là (le premier rouge était l'étape corrigée par `930cccc96`).

## Le témoin

Le test lui-même. Le journal : `~/.cache/rag3weaver-build/ide-vert/lib.log` à luciepc.

## Pour le fermer

Savoir si les jetons du tour lent n'arrivent pas dans la bulle en 20 s sous charge, ou s'ils
arrivent sans que la bulle porte `.tour.en-cours`. Puis attendre le bon signal de la page.
