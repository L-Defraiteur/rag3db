# Un outil lent du chat de l'IDE ne dit pas qu'il attend

- **État** : fermé le 19 novembre 2026 par la session tty (branche `cx-chat-waiting-2`). Le minuteur de l'équipier est partagé (`teammate_tools::say_the_wait`), et les outils d'un tour du chat sont enveloppés (`chat::DitSonAttente`) : au-delà de `teammate.tool_wait_notice_s`, `Waiting` puis `Resumed` sur le run du tour. Témoin : `daemon_host::chat::tests::a_slow_tool_of_the_chat_says_it_waits_then_resumes`, rouge (763271512d, reposé en 0ac2c9a05e), puis vert ; `daemon_host::` 183 verts sous la lib 7c2b506ec. La page `Chat.tsx`, qui nommerait l'attente, reste à faire par l'IDE.
- **Famille** : produit
- **Gravité** : gêne. L'écran tourne sans dire ce qu'il attend.
- **Atteignable en service** : oui. Un tour du chat de l'IDE dont un outil attend un index ou un slot tenu.
- **Touche rag3weaver** : oui (`src/daemon_host/chat.rs`, `OutilsDuChat`).

## Ce que c'est

`Waiting {run, cause: "tool", what, since_ms}` et `Resumed` (lot `cx-hote-waiting`, page `docs/tty/19-11-2026/05-l-hote-dit-qu-un-tour-attend.md`) ne partent que des pas d'un équipier. Le minuteur est celui de `Etanche::par_le_backend`, au-delà de `teammate.tool_wait_notice_s`. Un tour du chat de l'IDE appelle ses outils par `OutilsDuChat`, sans ce minuteur : un outil lent n'y dit rien, ni dans la boîte ni sur le bus.

## Ce qu'il faut faire

- Le même minuteur autour des appels d'outil d'un tour du chat, avec un seuil déclaré (celui de l'équipier, ou un `chat.tool_wait_notice_s`). Il émet `Waiting` sur le run du tour, puis `Resumed`.
- La page (`Chat.tsx`, qui suit `runRef`) nomme l'attente à côté de son indicateur.

## La preuve

Un test du chat, rouge d'abord : un outil retenu au-delà du seuil donne un `Waiting` puis un `Resumed` sur le run du tour, et un outil rapide n'en donne aucun.
