# Le témoin du chat dans la page rougit à la première exécution après un bâti

- **État** : ouvert (7 novembre 2026), pour everything declarative — correctif dans un petit lot à
  part, après les vues 5, 6, 7 et 3b de l'IDE en React.
- **Gravité** : faux rouge (le témoin), et peut-être une réponse fausse de la page derrière (un
  tour effacé de l'écran par une relecture tardive — hypothèse, à prouver à froid).
- **Atteignable en service** : la page, peut-être : un tour envoyé pendant qu'une relecture de
  l'état du chat est en vol pourrait disparaître de l'écran jusqu'au prochain rechargement ; le
  témoin, non.
- **Touche rag3weaver** : oui (`src/daemon_host/chat/tests.rs`, `src/daemon_host/navigateur.rs`,
  `ui/ide/src/views/Chat.tsx`).
- **Famille** : produit

## Ce que c'est

`daemon_host::chat::tests::le_chat_se_mene_et_s_arrete_dans_la_page` (témoin navigateur, feature
`navigateur`) rougit à son dernier pas — « le tour qui propose flux fermé, le lien de sa
proposition » — **à la première exécution après un bâti**, sous charge : la vue montre l'état
« fini » et trois tours, le quatrième (« écris une autre note ») n'apparaît pas ; aucun appel en
vol, aucune erreur à la console. Les exécutions suivantes du même binaire passent. Chez 96, le
même témoin a rougi le matin du 7 novembre sur un autre pas (« flux fermé jamais vu »), lui aussi à
la première exécution après un bâti.

## Le témoin et ses chiffres (luciepc, lib 04063ee06 épinglée, 7 novembre 2026)

| code | exécutions | rouges | où |
|---|---|---|---|
| master cb5e103fb + vue 5 (be865293a) | 7 | 2 | la passe entière (étape navigateur, 11/12), puis la première exécution isolée après le bâti |
| master cb5e103fb seul | 5 | 0 | isolé, dont une première exécution après bâti |

Ici même (le portable, lib 04063ee06 épinglée, 7 novembre 2026 à 15 h 55, session 73) : master
554f3055e + whois (gabarit memory seul, aucun code du chat ni de la page), la passe entière rougit
à l'étape navigateur (11/12) sur **un autre pas**, celui du tour lent (« Arrêter » et ses jetons
jamais vus en 120 s, le pas du ticket du 25 octobre `2026-10-25-le-temoin-du-chat-rougit-encore-sur-le-tour-lent`),
à la première exécution après le bâti ; rejoué seul aussitôt : vert en 1,34 s.

La vue 5 ne touche ni le chat ni le flux (des routes dans `main.tsx`, « taches » dans
`VUES_PORTEES`) et le bundle commité est celui du code (`build.mjs --check` vert) : ce rouge n'est
pas une cause de la vue 5, il est celui du témoin — et sans doute aussi celui de master, qui ne
l'a pas montré en 5 exécutions.

## La cause (hypothèse lue dans le code)

Le témoin ferme le flux (`window.__ide.flux.close()`), envoie un tour, puis attend sa relecture en
appelant `window.__ide.flux.onopen()` **à chaque évaluation de sa condition** (`attendre`, toutes
les quelques dizaines de millisecondes) : chaque appel relit l'état de l'hôte et celui du chat
(`reload()` dans `Chat.tsx`). Plusieurs relectures sont donc en vol quand le pas suivant envoie le
quatrième tour ; une relecture partie avant ce tour, revenue après sa réponse, remplace les tours
rendus par un état qui ne le contient pas (`setItems(fromTurns(s.turns))`, dès qu'aucun tour n'est
« live ») : l'état dit « fini », le tour a disparu. À froid (premier bâti, caches vides), les
réponses sont plus lentes et la course se gagne du mauvais côté. Le flux de la page ne se rouvre
pas de lui-même après `close()` ; la relecture tardive vient bien du témoin.

## Ce qu'il faut pour le fermer

- **Le témoin** (c7, 7 novembre) : il attend l'ouverture du flux par son repère (comme le harnais
  attend l'appel en vol, bb0f4d9c0), jamais une durée, et dit les secondes ; il appelle `onopen()`
  une fois, pas à chaque évaluation ; rouge d'abord à froid (première exécution après un bâti).
- **La page** : une relecture partie avant qu'un tour ait fini n'écrase pas ce tour (un compteur
  d'époque : la relecture n'applique son état que si aucune réponse de tour n'est arrivée depuis
  son départ) — si l'hypothèse se prouve.
- **D'ici là** : une passe où ce seul témoin rougit se compte en nommant ce ticket, et le témoin
  se rejoue une fois seul avant de pousser.
