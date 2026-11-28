# Les premiers mots d'une conversation neuve sont perdus

- **État** : ouvert le 28 novembre 2026 par la session tty, vu au rejeu de la relance de l'hôte sur un hôte jetable à luciepc (faux modèle local).
- **Famille** : produit
- **Gravité** : défaut — les premiers mots de la première réponse d'une conversation neuve ne s'affichent jamais.
- **Atteignable en service** : oui, à chaque première demande ; peu visible quand le premier mot du modèle tarde (Gemini), net avec un modèle rapide.
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty` : `tui.rs`).

## Ce que c'est

À l'écran du rejeu : « du faux modèle : je réponds lentement, mot après mot. » sans « Réponse 1 », ses deux premiers
mots. La vue écarte toute trame tant qu'elle ne connaît pas le manifeste de sa tâche (`frame_is_the_tasks` : rien
n'est à elle avant qu'elle s'ouvre, le 22 novembre). Ce manifeste n'est connu qu'après la réponse au POST de la demande
puis une lecture de `/ide/teammate` (`connaitre`) ; les jetons émis entre-temps sont jetés.

## Ce qui est attendu

Tant que le manifeste de la tâche n'est pas connu, la vue **garde** les trames au lieu de les jeter, dans un tampon
borné (en nombre de trames et en durée : une tâche qui ne s'ouvre jamais ne le fait pas grossir sans fin). Dès que le
manifeste est connu, elle rejoue, dans l'ordre, celles qui sont à sa tâche, et oublie les autres.

## Le témoin

Des trames de la tâche arrivent avant que le filtre soit posé, puis le filtre ; elles sont rendues entières et dans
l'ordre, celles d'une autre tâche jamais ; le tampon reste borné en nombre et en âge.
