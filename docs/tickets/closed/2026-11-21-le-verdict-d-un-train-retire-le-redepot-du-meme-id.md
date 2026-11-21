# Le verdict d'un train retire de la file le redépôt du même id

- **État** : corrigé (21 novembre 2026, lot pusher-verdict-own-entry : chaque pas retient les lignes de file qu'il a
  prises, et un verdict n'agit que sur l'entrée identique ; un rouge sur une entrée remplacée est dit « rouge d'un dépôt
  retiré », sans retirer le redépôt ; un vert ne pousse pas l'ancien train ; témoin rouge d'abord dans test_pusher.sh,
  cas 47). Écrit par G à la demande de c7.
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : le pousseur)
- **Gravité** : moyenne. Un lot redéposé disparaît de la file sans que son auteur le sache ; sur une fin verte, l'ancien
  contenu est poussé à la place du nouveau.
- **Atteignable en service** : non, le pousseur seulement
- **Touche rag3weaver** : non (`tools/poste/pusher.sh`)

## Ce que c'est

Le 21 novembre 2026, `e2e-equipier-faux-modele` est pris à 11 h 20 min 19 s (train pusher-20261121-112021-175). La
session archi le retire à 11 h 20 min 39 s, puis le redépose sous le même id à 11 h 20 min 54 s, avec une autre suite de
témoin. L'ancien train finit rouge à 11 h 21 min 22 s : `red()` retire de la file l'entrée de cet id, qui est déjà le
redépôt. Le redépôt disparaît.

Le correctif du dequeue pendant une passe (lot pusher-dequeue-during-pass, en service depuis 11 h 06) ne couvrait pas ce
cas : il empêche de pousser un lot retiré, mais ne vérifie que la présence de l'id dans la file, et ne touche pas aux
rouges. Rejoué en témoin, une fin verte de l'ancien train poussait même l'ancien contenu et retirait le redépôt.

## Le correctif

- à la prise d'un train (et du créneau 2), le pas retient la ligne de file de chaque lot pris ;
- `red()` n'agit que si la file porte encore cette ligne même. Sinon, le rouge est dit au journal (« rouge d'un dépôt
  retiré ») et dans la boîte de l'auteur, et la famille du redépôt n'est pas touchée ;
- `left_queue`, avant la garde et dans `PRE_PUSH`, compare la ligne entière : un lot redéposé compte comme retiré, et
  son ancien train n'est pas poussé.
