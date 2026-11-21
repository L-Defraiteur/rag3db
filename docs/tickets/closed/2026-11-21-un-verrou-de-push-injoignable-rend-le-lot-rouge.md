# Un verrou de push injoignable rend le lot rouge, avec une raison vide

- **État** : corrigé (22 novembre 2026, lot pusher-lock-unreachable : `poste push` qui ne joint pas le verrou relayé rend
  l'état « verrou injoignable » ; le pousseur attend le verrou (PUSHER_LOCK_WAIT, 30 min), repousse le même train sans
  seconde passe, ou laisse les lots en tête de file, jamais rendus ; témoin rouge d'abord, test_pusher.sh cas 48). Écrit
  par G à la demande de c7, pour embarquement.
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : le pousseur)
- **Gravité** : moyenne. Un lot vert rendu à son auteur, et une passe de vingt minutes perdue, pour une panne qui
  n'est pas la sienne.
- **Atteignable en service** : non, le pousseur seulement
- **Touche rag3weaver** : non (`tools/poste/pusher.sh`)

## Ce que c'est

Le verrou de push sur master vit à la tablette : le `poste` de luciepc le prend par ssh (`poste-push-hote`). Le 21
novembre 2026, pendant le redémarrage de la tablette, `emb-reuse-by-hash` a été rejoué VERT à 15 h 40. Puis `poste push`
n'a pas pu joindre le verrou (« ssh muet ou refusé ») : rien n'a été exécuté, mais le pousseur a lu ce refus comme un
rouge de la garde, sans ligne « ARRÊT », et a rendu le lot rouge avec une raison vide.

## Le correctif

`push_train` reconnaît ce refus (code 5). Le pousseur dit « verrou injoignable » au journal, puis attend que
`poste push --qui` réponde, au plus PUSHER_LOCK_WAIT secondes (1800 par défaut). Revenu, il repousse le même train,
sans rejouer la passe ; toujours injoignable, il laisse les lots en tête de file, sans rien rendre. Le créneau 2 laisse
ses lots en file de même.
