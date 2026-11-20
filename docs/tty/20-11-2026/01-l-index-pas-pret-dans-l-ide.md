# L'index pas prêt dans l'IDE

Session tty, nuit du 19 au 20 novembre 2026. C'est une tâche de c7, posée sur les deux lots de recherche :
- l'attente des vecteurs : le champ `index` d'une réponse d'outil ;
- la notification « index prêt » du client mince : `ReadyWatches`, `$/rag3weaver/notice`.

Branche `cx-index-pas-pret-2`, posée sur `index-ready-notify` de recherche (2b8c8d4632), qui porte aussi la correction ci-dessous.

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

## Un défaut de recherche, trouvé ici, porté par recherche

`SearchSettings` refusait les champs inconnus et ne connaissait que `index_wait_s`. Un manifeste qui déclarait `search.ready_watch` ne s'ouvrait plus. Le témoin de recherche écrivait un faux manifeste, jamais chargé. La correction déclare `ready_watch` dans `SearchSettings`, avec ses quatre champs typés ; une faute de frappe y est toujours refusée. Je l'ai écrite rouge d'abord ; recherche l'a reprise dans son propre lot (ae38d24ecb rouge, 2b8c8d4632 vert), pour que son lot n'annonce pas un réglage qui empêche le manifeste de s'ouvrir.

## Les témoins, rouges d'abord

| Témoin | Ce qu'il prouve | Rouge | Vert |
|---|---|---|---|
| `backend_tools::tests::the_state_of_an_index_not_ready_goes_with_the_receipt` | l'état dans `metadata.index` | 264c92c353 | 13374db45c |
| `daemon_host::chat::tests::an_index_not_ready_in_the_chat_is_watched_and_said_ready_once` | deux réponses pas prêtes, un seul `IndexReady` | ae3cfd4348 | d89fadb605 |
| `daemon_host::chat::tests::the_chat_page_says_an_index_not_ready_then_ready_once` (navigateur) | la ligne sous la bulle, puis une seule ligne « prêt » | 5a02c4fb6a | ba3e9e7f37 |

Les rouges ont été joués sur la première pose (`cx-index-pas-pret-1`, sur 50b892df9e), le navigateur à luciepc ; la repose sur la tête de recherche ne change aucun fichier.
- Au premier vert du témoin navigateur, une seule assertion tombait : « l'outil n'est pas refusé » attendait `data-ok="true"`. Or la bulle relue depuis l'état du chat, à la fin du tour, ne porte pas `ok`. Le témoin juge maintenant un refus à ce qui le dit : `data-ok="false"`, ou « refusé » dans le titre (ae80d805d9).
- Vert à luciepc sur ae80d805d9 : `daemon_host::chat::tests` (42, navigateur compris), `backend_tools`, `daemon_client::tests`, `ide_par_le_flux` et le contrôle du bundle.

Le banc est le même pour les deux derniers (`BancPasPret`). Ce n'est pas un faux hôte : c'est le vrai chemin, avec un retard noté à la main.
- Un fichier est noté en retard dans la couche du backend, à un débit observé de 20 s par fichier. `usages` répond donc tout de suite, pas prêt.
- Aucune réaction ne touche ce fichier : le retard reste jusqu'à ce que le témoin le lève (`done_in`), et c'est la vraie veille qui le voit.

## Ce qui reste

- **La fenêtre de la vue** : l'avis « index prêt » dans une fenêtre détachée du dépôt. Ce sera un lot à part, sur `ide-fenetres-5` (les fenêtres ne sont pas dans la base de recherche), avec ce lot-ci en second parent.
- La veille de l'hôte ne regarde que la couche du backend (`view: null`). Le chat de l'IDE ne travaille dans aucune vue. Si un jour il en tient une, la clé et le texte la porteront déjà (`what_a_watch_says`).
