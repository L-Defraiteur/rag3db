# La page du chat nomme ce qu'un tour attend

Session tty, 19 novembre 2026, au mot de c7. C'est la moitié page du ticket « Un outil lent du chat de l'IDE ne dit pas qu'il attend ». La moitié hôte est le lot `cx-chat-waiting-2` : le chat émet `Waiting` puis `Resumed` sur le run du tour.

## Ce que fait la page

`ui/ide/src/views/Chat.tsx` :
- un `Waiting` du run suivi (`runRef`) pose une attente : la cause, le nom, le début (`since_ms`) et la fenêtre (`wait_ms`) ;
- `Resumed`, un nouvel appel, la fin d'un appel ou la fin du run la lèvent ;
- pendant l'attente, `#status` dit « propose attend… (N s) », ou « <modèle> attend le fournisseur… (reprise dans N s) » ;
- une minuterie locale d'une seconde ne tourne que pendant l'attente. La page ne demande rien à l'hôte pour elle : `the_ide_asks_the_host_nothing_while_idle_and_only_its_gesture_for_a_turn` le garde.

Le bundle est rebâti et commité (`scripts/ide_bundle.sh`) ; `--check` est vert.

## Le témoin

`daemon_host::chat::tests::la_page_du_chat_nomme_ce_qu_un_tour_attend`, témoin navigateur, joué à luciepc (Chromium n'est que là), lib 7c2b506ec, `RAG3WEAVER_EXIGER_NAVIGATEUR=1`. Le faux modèle appelle `propose` pendant que le test tient la liste des propositions 2,5 s, sous un seuil d'1 s. Le test garde l'hôte pour atteindre son backend : il reprend les quatre lignes de `hote_garde_avec`. Ce qu'il vérifie :
- `#status` dit « propose attend… (N s) » ;
- un `MutationObserver` relève chaque texte du statut : au plus 6 textes d'attente, un par seconde, et l'attente vient avant « fini ». C'est la borne, comme les octets par image du terminal ;
- aucune erreur à la console.

Rouge sans le code de la page (9bb66923fd, avec la page et le bundle de 0aad08fc0c remis par chemins : le statut ne l'a jamais dit en 22 s, et la carte de `propose` dit 2 500 ms), puis vert sur la tête.

Deux premiers essais du témoin étaient faux, et c'est dit :
- le gabarit notebook, à qui le banc donnait un `workspace` sans `source` ;
- puis un `put_note` que le chat refuse tout de suite, puisqu'il propose au lieu d'écrire. Le rouge ne prouvait rien : l'appel n'attendait pas (100 ms).

## Les remarques d'archi sur l'hôte (`cx-chat-waiting-2`)

- **Un fil de veille par appel d'outil du chat.** `say_the_wait` ouvre un fil court qui dort par pas de 100 ms jusqu'à la fin de l'appel : un tour de 60 appels, c'est 60 fils courts. C'est négligeable devant un appel au modèle.
- **La ligne de boîte de l'équipier reste en français** (« « outil » attend depuis N s … »), alors que le modèle la relit à sa frontière : ticket [`docs/tty/tickets/2026-11-19-la-ligne-d-attente-de-la-boite-est-en-francais.md`](../tickets/2026-11-19-la-ligne-d-attente-de-la-boite-est-en-francais.md).
