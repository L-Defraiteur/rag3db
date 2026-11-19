# Le témoin du chat rougit encore sous la lib entière, sur le tour lent

- **État** : fermé (19 novembre 2026, session recherche) : le symptôme du tour lent n'est jamais revenu.
- **Ouvert le** : 25 octobre 2026 (vu par la session « recherche », à luciepc).
- **Gravité** : test instable.
- **Atteignable en service** : non (le test).
- **Touche rag3weaver** : oui (`src/daemon_host/chat.rs`, `src/ide/page.html`).
- **Famille** : produit
- **Pour** : recherche (suivi et lot ; toute réattribution passe par l’orchestration).
- **Signalé par** : recherche ; sources et observations dans ce ticket et le [rapport de session](../11-11-2026/01-rapport-de-session.md).

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

## Ce qui le ferme (19 novembre 2026, recherche)

**Le relevé.** À luciepc, 481 journaux de lib ou de témoins navigateur écrits depuis le 26 octobre. Le témoin
`le_chat_se_mene_et_s_arrete_dans_la_page` y est vert dans 107 passes depuis le 15 novembre. Il n'a rougi qu'**une
fois** depuis le 26 octobre (`passe-garde-124920`, 18 novembre 13 h 01), et pas au tour lent : sur la page restée
vide au chargement (« la vue : "" ; en vol : [] ; erreurs : [] »).

**Ce qu'il en reste.** Le symptôme du tour lent (« `encore` dans la bulle du tour en cours », non vu en 20 s) ne
s'est pas reproduit. Le rouge de la page vide est un autre défaut, commun à plusieurs témoins du chat. Il est suivi
dans `docs/recherche/tickets/2026-11-14-le-temoin-du-chat-d-une-tache-rougit-sous-charge.md`.
