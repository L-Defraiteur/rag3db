# Un outil lent du chat de l'IDE ne dit pas qu'il attend

- **État** : ouvert. Relevé par archi à la relecture de `cx-hote-waiting` (19 nov. 2026), écrit par la session tty ; sans porteur.
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
