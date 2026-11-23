# Un appel refusé parce que son backend est occupé se disait non transitoire

- **État** : corrigé (23 novembre 2026), branche `cx-memoire-busy-slot-transient`.
- **Famille** : produit
- **Gravité** : un client qui lit le champ `transient` abandonnait un appel qu'il pouvait refaire. Pour Lucie : un
  rappel de mémoire qui échoue pendant la maintenance nocturne du plein texte, sans être retenté.
- **Atteignable en service** : oui. Toute fermeture, réouverture ou ouverture qui tient le backend plus de
  `SLOT_READ_WAIT` (5 s), dont la maintenance du plein texte lancée par l'hôte après 03 h 30.
- **Touche rag3weaver** : oui, `daemon_host.rs` (`lire_le_slot`, `op_response`).
- **Signalé** : le 23 novembre 2026 à 09 h 37, à l'essai d'avant pose, sur une copie de la mémoire longue.

## Ce qu'on voyait

`recall_fiches` rendait `{"ok": false, "transient": false, "error": "… une fermeture ou une réouverture tient ce
backend depuis plus de 5 s : cet appel n'a rien fait, recommencez dans un moment"}`. Le texte disait de recommencer,
le champ disait le contraire.

## La cause

`op_response` ne marquait transitoire qu'une erreur commençant par `REOPENED`. Les refus du slot occupé, eux,
commencent par le chemin du manifeste.

## Le correctif

Les deux refus du slot occupé (fermeture ou réouverture, ouverture) finissent par la constante `SLOT_BUSY`, et
`op_response` marque transitoire toute erreur qui finit ainsi.

## Les témoins (vus rouges d'abord)

`daemon_host/busy_slot_tests.rs` : les deux refus du slot occupé sont transitoires, une autre erreur ne l'est pas.
