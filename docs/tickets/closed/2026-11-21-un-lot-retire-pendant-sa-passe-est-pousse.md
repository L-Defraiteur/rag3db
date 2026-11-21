# Un lot retiré de la file pendant sa passe est poussé quand même

- **État** : corrigé (21 novembre 2026, lot pusher-dequeue-during-pass : avant la garde, puis dans le crochet `PRE_PUSH`
  de la garde, le pousseur relit sa file ; un lot qui l'a quittée arrête le train, qui n'est pas poussé, et le journal
  dit « retiré pendant la passe, non poussé » ; témoins rouges d'abord dans test_pusher.sh et test_push_garde.sh). Écrit
  par G à la demande de c7.
- **Pour** : G (optimiseur)
- **Famille** : produit (l'outillage des passes : le pousseur et sa garde)
- **Gravité** : moyenne. Un lot que son auteur retire atterrit sur master sans son accord.
- **Atteignable en service** : non, le pousseur seulement
- **Touche rag3weaver** : non (`tools/poste/pusher.sh`, `tools/poste/push_garde.sh`)

## Ce que c'est

Le 21 novembre 2026, `arbre-diffs-des-vues` est pris et reposé par le pousseur à 09 h 58. La session arbre le retire
par `pusher.sh dequeue` à 10 h 07 min 41, pour le renommer en anglais. Sa passe tourne encore : elle finit VERT à
10 h 12, et le pousseur pousse le lot à 10 h 13 (5d3e2c9bf9, PR 388). `dequeue` ne faisait que retirer la ligne de la
file : le train déjà pris ne relisait jamais la file avant de pousser. Le créneau 2 la relisait, lui (« n'est plus en
file »), mais sans dire le lot retiré.

Rien n'a été défait sur master : le lot était sain, seulement nommé en français ; arbre le renomme par un lot de suite.

## Le correctif

- après la passe, avant la garde (`replay_and_push_a`, `spec_settle`) : un lot du train qui n'est plus en file arrête
  le train ; chaque lot retiré est dit « retiré pendant la passe, non poussé », les autres restent en file pour le pas
  suivant ;
- pendant la garde : son crochet `PRE_PUSH` (l'ouverture des PR) relit la file et rend 75 ; la garde s'arrête alors
  (« ARRÊT : le pousseur retire le push »), master ne bouge pas, et aucun lot n'est rendu rouge.

Reste une fenêtre de quelques secondes entre `PRE_PUSH` et le `git push` de la garde.
