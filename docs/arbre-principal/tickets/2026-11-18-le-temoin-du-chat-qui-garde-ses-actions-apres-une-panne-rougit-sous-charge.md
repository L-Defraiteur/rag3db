# Le témoin du chat qui garde ses actions après une panne rougit une fois sous charge

- **État** : ouvert (18 novembre 2026), arbre principal, pour le propriétaire du chat de l'IDE.
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

## Ce qu'il faut pour le fermer

Rejouer le témoin en boucle sous charge (des lourds en parallèle) jusqu'à le voir rouge. Puis départager :

- soit le témoin évalue avant que l'action soit affichée, et il doit attendre la condition au lieu de la lire une
  fois ;
- soit l'affichage perd vraiment l'action dans une course, et c'est un défaut du chat.
