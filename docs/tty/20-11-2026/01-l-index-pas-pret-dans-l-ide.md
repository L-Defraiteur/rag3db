# L'index pas prêt dans l'IDE

Session tty, nuit du 19 au 20 novembre 2026. C'est une tâche de c7, posée sur les deux lots de recherche :
- l'attente des vecteurs : le champ `index` d'une réponse d'outil ;
- la notification « index prêt » du client mince : `ReadyWatches`, `$/rag3weaver/notice`.

Branche `cx-index-pas-pret-1`, sur `recherche-notification-index` (50b892df9e).

## Ce que la page dit

1. **Sous la bulle d'un outil dont la réponse n'est pas prête**, une ligne en clair, lisible sans déplier la bulle. Elle dit, quand l'hôte les donne :
   - l'estimation du texte ;
   - celle des vecteurs, avec le nombre de morceaux dus ;
   - ou la raison de leur absence ;
   - les fichiers en retard ;
   - « l'index s'écrit en ce moment » pendant une écriture.

   L'outil n'est pas refusé, et le tour finit.
2. **Une fois la couche rattrapée**, une ligne de page dans le chat du backend : « l'index de la vue du backend est prêt : texte ». Elle vient une seule fois, même si plusieurs réponses n'étaient pas prêtes pendant la même veille.

## Comment l'hôte le sait

- **L'état suit le reçu.** Le modèle lit l'état dans la ligne « > » du texte. Le tour, lui, ne gardait que `metadata.invocation`. `BackendTools` ajoute maintenant `metadata.index`. Il passe donc dans le `ToolCallFinished` du flux, et dans l'état du chat au rechargement.
- **L'hôte veille comme le client mince.** La boîte d'outils du chat (`OutilsDuChat::appeler`) voit chaque réponse dont `index.ready` vaut false. L'hôte lance alors une veille `ReadyWatches`, réutilisée telle quelle :
  - une veille par backend et par vue ;
  - ses réglages sont ceux de recherche (`search.ready_watch`) ;
  - son texte et sa clé sont partagés avec le client mince (`daemon_client::what_a_watch_says`).

  La sonde est `index_state_now`. Une fois la couche prête, l'hôte publie `IndexReady {view, text}` sur le flux de l'IDE, sujet `index`, que la page reçoit désormais.

## Un défaut de recherche, corrigé ici

`SearchSettings` refusait les champs inconnus et ne connaissait que `index_wait_s`. Un manifeste qui déclarait `search.ready_watch` ne s'ouvrait plus. Le témoin de recherche écrivait un faux manifeste, jamais chargé. La correction déclare `ready_watch` dans `SearchSettings`, avec ses quatre champs typés ; une faute de frappe y est toujours refusée. Rouge f450a06dcf, vert 9fa084ef38. Recherche est prévenue.

## Les témoins, rouges d'abord

| Témoin | Ce qu'il prouve | Rouge | Vert |
|---|---|---|---|
| `backend_tools::tests::the_state_of_an_index_not_ready_goes_with_the_receipt` | l'état dans `metadata.index` | db0e6100e0 | 8c5d837384 |
| `daemon_client::tests::a_manifest_that_declares_the_watch_settings_still_opens` | le manifeste s'ouvre | f450a06dcf | 9fa084ef38 |
| `daemon_host::chat::tests::an_index_not_ready_in_the_chat_is_watched_and_said_ready_once` | deux réponses pas prêtes, un seul `IndexReady` | c344aa15d1 | (veille) |
| `daemon_host::chat::tests::the_chat_page_says_an_index_not_ready_then_ready_once` (navigateur) | la ligne sous la bulle, puis une seule ligne « prêt » | 59c9ca7df7 | (IDE) |

Le banc est le même pour les deux derniers (`BancPasPret`). Ce n'est pas un faux hôte : c'est le vrai chemin, avec un retard noté à la main.
- Un fichier est noté en retard dans la couche du backend, à un débit observé de 20 s par fichier. `usages` répond donc tout de suite, pas prêt.
- Aucune réaction ne touche ce fichier : le retard reste jusqu'à ce que le témoin le lève (`done_in`), et c'est la vraie veille qui le voit.

## Ce qui reste

- **La fenêtre de la vue** : l'avis « index prêt » dans une fenêtre détachée du dépôt. Ce sera un lot à part, sur `ide-fenetres-5` (les fenêtres ne sont pas dans la base de recherche), avec ce lot-ci en second parent.
- La veille de l'hôte ne regarde que la couche du backend (`view: null`). Le chat de l'IDE ne travaille dans aucune vue. Si un jour il en tient une, la clé et le texte la porteront déjà (`what_a_watch_says`).
