# Relevé de l'avis de pause du bus entre sessions — le mercredi 18 novembre 2026

C'est la suite de `16-11-2026/02-releve-de-la-trame-du-bus.md`. Lucie a autorisé directement,
dans la session d'archi, cette seconde entrée témoin : temporaire, retirée après. Claude Code est
en version 2.1.322.

## Le montage

- **La socket.** Le témoin `temoin.py` (pid 1748426) écoute sur
  `/run/user/1000/cc-socks/1748426.sock`. Il journalise et ne répond rien.
- **L'entrée.** `~/.claude/sessions/1748426.json` a la même forme que la première, avec deux
  différences :
  - `name` vaut « algareth-temoin-2 » ;
  - **`peerFeatures` vaut `["notify_idle"]`**, et `status` « busy » au départ.
- **Le retrait.** L'entrée et la socket sont retirées à 14 h 26 min 46 s, et le processus arrêté.

## Ce qui est vu

1. **L'abonnement passe par `peerFeatures`.** c7 envoie avec `notify_when_idle: true`, et son
   outil répond : « Subscribed — you will get one notice here when "algareth-temoin-2" is next idle
   (or exits), provided that session runs in the same permission class as this one (or is one this
   session spawned) or asserts none ». Le 16, avec `peerFeatures` vide, l'abonnement avait été
   refusé.
2. **L'abonnement est une trame de contrôle envoyée au receveur**, sur sa propre connexion,
   juste après le message (14 h 18 min 47 s, 178 octets) :

   ```json
   {"type":"control","action":"notify_when_idle","from":"uds:/run/user/1000/cc-socks/<pid émetteur>.sock","from_mode":"prompting","msgV":1,"msg_id":"<uuid>"}
   ```

   C'est donc le receveur qui tient l'abonnement et qui doit envoyer l'avis vers `from`.
3. **Le registre seul ne déclenche rien.** Le `status` de l'entrée est passé à « idle » à
   14 h 20 min 02 s. c7 n'a reçu aucun avis, ni « [Cross-session idle notice] » ni notice de
   livraison.
4. **Le relevé de l'avis lui-même n'a rien donné.** Avec l'accord de c7, le témoin lui a envoyé la
   même trame de contrôle, avec sa socket en `from` (14 h 21 min 29 s). c7 a fini son tour juste
   après, puis en a commencé un autre. En cinq minutes, le témoin n'a reçu que des sondes vides
   (47 connexions en tout). Trois explications possibles, non départagées :
   - Claude Code refuse un abonnement qui vient d'une socket sans fichier `.key` ;
   - il le refuse parce que la « classe de permission » diffère ;
   - il n'émet l'avis qu'à une autre transition que la fin d'un tour.

## Pour le mode pair d'Algareth

- **Recevoir un abonnement** : lire les trames `type: "control"`, `action: "notify_when_idle"`, et
  garder `from` et `msg_id`.
- **Déclarer `notify_idle`** dans `peerFeatures` **seulement si** Algareth envoie l'avis à sa
  prochaine pause (ou à sa sortie).
- **La forme de l'avis** reste à relever. Quand une vraie session Claude Code en enverra un à une
  socket d'Algareth, il suffira de le journaliser. D'ici là, Algareth peut ne pas déclarer
  `notify_idle` : l'émetteur dit alors franchement « nothing was subscribed ».

Le journal brut est hors du dépôt (il porte le texte des messages).
