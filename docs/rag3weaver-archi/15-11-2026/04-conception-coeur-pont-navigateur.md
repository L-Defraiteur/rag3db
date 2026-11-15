# Le cœur Rust du pont navigateur : la conception fine

15 novembre 2026, 11 h 4x. Lucie a choisi le socle de l'[étude](03-etude-pont-navigateur.md) : un fork de
jackwener/browsermcp dont l'hôte Node est remplacé par du Rust (c7). Cette page conçoit le premier
lot : la bibliothèque seule, jouée sur un faux pont. Ni extension, ni fork distant, ni MCP dans ce
lot : le compte GitHub attend le mot de Lucie. C'est un lot à part de la pile.

Protocole repris de `src/protocol.ts` (révision 5) et de `src/host/{native-messaging,bridge}.ts`, au SHA
`d6597aacb`.

## La conception, en quatre lignes

1. **Un module `pont_navigateur` dans rag3weaver** (pas de feature : la bibliothèque standard et serde ;
   le nom `navigateur` est déjà celui de la feature des témoins de l'IDE). Ses quatre parties :
   - `cadrage` : longueur sur 4 octets en petit-boutiste, puis du JSON UTF-8. Une trame de plus de
     1 Mio vers l'extension est refusée avant l'envoi. Un décodeur incrémental reçoit des morceaux.
   - `protocole` : les types serde des enveloppes (`command` et `ready` vers l'extension ; `hello`,
     `result` et `event` vers l'hôte). `Command` porte `id`, `action`, `session`, `page`,
     `deadlineAt`, et les champs des actions de ce lot (`claim`, `mark`, `closing`, `op`, `code`,
     `world`, `url`, `act`, `cdpMethod`, `chromeMethod`) ; les champs absents ne s'écrivent pas. Les
     champs inconnus venus de l'extension sont tolérés.
   - `pont` : un `Pont` générique sur un lecteur et un écrivain (stdin et stdout en vrai, un tube
     dans les témoins). Un fil lit les trames ; les appels en attente sont rangés par `id`.
     - `hello` répond `ready` avec la version, et note `browserId`, révision et capacités.
     - `envoyer(action, params, délai)` pose `deadlineAt` et attend son `result`.
     - Un délai dépassé rend `timeout`, et le résultat tardif est jeté. Un canal fermé rend
       `extension_disconnected` à chaque appel en attente.
     - Une révision autre que 5 refuse, sans les envoyer, toutes les commandes sauf `ping`
       (`protocol_mismatch`).
   - `baux` : la table des onglets de chaque session, côté hôte, tenue par les résultats et les
     événements. Un bail est `active`, `handoff` ou `released`, avec son origine (`user` ou
     `agent`). `tab_acquired` et un `claim` réussi l'ouvrent ; `tab_released` et `tab_closed` le
     ferment ; `mark: handoff` puis `session-finalize` le passent en `handoff`.
2. **Deux gardes de l'hôte, avant l'envoi**, en attendant le fork de l'extension :
   - **le passage de main tient** : une commande qui nomme une `page` en `handoff` ou `released`
     est refusée par l'hôte (`page_handed_off`, `page_released`) jusqu'à un nouveau `claim`.
     C'est le trou de l'extension amont : une commande à `page` explicite n'y examine pas le bail.
   - **les passe-partout sont fermés par défaut** : `cdp`, `chrome-call`, `chrome-describe`,
     `cookies`, `network-capture-*`, `stream-watch` et `extension-logs` sont refusés
     (`action_closed`). Une `Politique` ouvre une action, et pour `cdp` une liste blanche de
     méthodes ; rien n'est ouvert sans qu'on l'écrive.
3. **Ce qui n'est pas dans ce lot** : le relai Native Messaging vers l'hôte rag3weaver du poste,
   les outils de l'atelier et leurs accords, la façade MCP, le manifeste d'hôte de Chrome, et le
   passage de main comme pas du run. Ils viennent ensuite, sur cette bibliothèque.
4. **Rouge d'abord, sur un faux pont** : une fausse extension dans un fil, qui parle la révision 5
   par un tube, joue :
   - le cadrage : aller-retour, trame découpée en morceaux, trame trop grosse refusée ;
   - `hello` puis `ready` ; une révision 4 qui refuse `exec` mais laisse passer `ping` ;
   - deux commandes servies dans l'ordre inverse, chacune à son appel ; un délai dépassé ; une
     fermeture du canal pendant une attente ;
   - `claim` puis `mark: handoff` et `session-finalize`, puis un `exec` vers cet onglet : **refusé
     sans être envoyé** (la fausse extension compte ce qu'elle reçoit) ; un nouveau `claim` le
     rouvre ;
   - `cdp` et `chrome-call` refusés par défaut sans envoi ; `cdp` sur une méthode listée, envoyé.

   Les témoins sont des tests de la lib (`pont_navigateur::`), sans e2e ni GPU. La passe : ces
   tests, `check_features`, puis la lib entière avant le « vert ».
