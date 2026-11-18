# Le témoin du chat qui garde ses actions après une panne rougit une fois sous charge

- **État** : fermé le 18 novembre 2026 par `81e569457` (archi, chat-attend-l-outil-4, sur master).
- **Famille** : produit
- **Gravité** : un rouge sans lien avec le lot joué a bloqué un push. Il faut une passe de plus.
- **Atteignable en service** : à départager : un délai du témoin, ou un affichage réellement tardif.
- **Touche rag3weaver** : oui (témoins navigateur).

## Ce que c'est

Le 18 novembre vers 13 h 50, à luciepc, quatre lourds tournaient en même temps. La passe `arbre-sondes` du lot
A1 a donné 44/45 aux témoins navigateur, et le rouge était
`daemon_host::chat::tests::une_erreur_du_modele_garde_les_actions_du_chat_et_permet_la_reprise`
(`src/daemon_host/chat/tests.rs:1870`). L'assertion « une panne n'efface pas la lecture déjà exécutée » a reçu
`false` : l'action `read_file` n'était pas à l'écran quand le témoin l'a lue. Le lot ne touchait que des tests de
mesure.

La même tête, reposée sur 411be25cb2, est repassée à 14 h 03 : 45/45.

**Deuxième occurrence, le même jour** : recherche l'a vu rougir à l'identique, même ligne et même assertion. C'était
le seul rouge de la passe de `view-lag-2` (895a584016), un lot qui ne touche pas le chat.

## Ce qu'il faut pour le fermer

Rejouer le témoin en boucle sous charge (des lourds en parallèle) jusqu'à le voir rouge. Puis départager :

- soit le témoin évalue avant que l'action soit affichée, et il doit attendre la condition au lieu de la lire une
  fois ;
- soit l'affichage perd vraiment l'action dans une course, et c'est un défaut du chat.

## Fermeture (archi, 18 novembre 2026)

Ce n'était pas un délai mais un défaut de la page de l'IDE (`ui/ide/src/views/Chat.tsx`).

- **Le constat.** Quand le modèle tombe juste après un outil, l'hôte garde bien le résultat de
  l'outil dans la session (`["user","assistant","tool(true)"]`). La page, elle, n'appliquait ni
  la fin d'outil du flux, qui arrivait après la réponse en erreur, ni l'état poussé de fin de tour,
  mis de côté parce qu'une requête en erreur ne porte pas de run. La carte restait donc à
  « read_file… ».
- **Le correctif.**
  - Un tour en erreur relit la session de l'hôte, comme le fait un tour réussi.
  - Une fin d'outil tardive se pose sur sa carte.
  - Le témoin attend la carte (attente bornée) et, sur un rouge, imprime ce que l'hôte a gardé et
    ce que la page montre.
- **La preuve.** Sous charge à luciepc (12 copies en parallèle × 3 tours), master donne 3 rouges
  sur 36 ; la branche donne 36/0, deux fois. Rejeu court sur la tête poussée : bundle `--check`,
  check_features, module chat 21/0, navigateur 45/0, lib 3d706f709.
- **Sur master** : `c9b73a704`, `5feb195f9`, `ef4fd7a13`, `81e569457`.
