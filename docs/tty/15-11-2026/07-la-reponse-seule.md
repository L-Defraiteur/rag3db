# tty-16 (premier morceau) : l'historique ne garde que la demande et la réponse

Session tty, 15 novembre 2026, au retour de Lucie sur son second essai : après le compte rendu de l'équipier, « [équipier · étape faite] fait : … » et « [hôte · fini] fait sans plan » étaient des lignes de plomberie, lues comme du bruit. Elle a précisé : « peut-être les étapes faites je les vois défiler de manière stylée à côté du truc comme quoi ça travaille, plutôt que d'avoir l'historique dans mon terminal ; pour un utilisateur comme moi j'ai pas besoin d'avoir les logs de chaque étape ».

## Ce qui change

- **L'historique du terminal ne garde que la demande et la réponse** : les envois de Lucie, les comptes rendus et réponses de l'équipier, ses questions et demandes d'accord, une commande `!` avec son retour, et les notes du terminal lui-même.
- **Le reste défile dans la zone vivante**, sur la ligne du haut, en sourdine, remplacé à mesure :
  - chaque appel d'outil (`» read_file src/a.rs  ⟨1⟩`, puis `· fait (3 ms)`) ;
  - une étape faite (« étape faite : … ») ;
  - les mots de l'hôte (« hôte : tâche ouverte : … ») ;
  - les plans de l'équipier.

  La fin du pas ne fait que changer l'état : la ligne d'état dit « fini : … ».
- **Ce qui s'est passé reste lisible à la demande** : Ctrl-O ouvre la visionneuse des appels (tty-11), avec leurs aperçus et leurs résultats, et `/journal` dit où relire la séance.
- **`--etapes`** (ou `RAG3WEAVER_TTY_ETAPES=1`, ou `/etapes` en cours de séance) remontre tout dans l'historique, comme avant : les appels, leurs aperçus colorés, les étapes et les mots de l'hôte.

L'invite qui fait parler l'équipier à l'utilisatrice, plutôt qu'au « superviseur », est chez archi.

## Les témoins

- Le test `the_teammate_plumbing_stays_out_of_the_history_unless_asked`, vu rouge : une demande, un mot de l'hôte, un `read_file`, un `report` et son compte rendu, un `step_done`, puis la fin du pas. Sans `--etapes`, l'historique garde la demande et le compte rendu seuls ; l'appel défile dans la zone vivante, l'étape faite y reste dite, l'état dit « fini », et les trois appels sont à Ctrl-O. Sous `--etapes`, tout est dans l'historique, aperçu compris.
- Trois anciens tests de la vue, qui vérifiaient l'historique avec ses appels, décrivent maintenant le mode `--etapes` et s'y mettent.
- `/etapes` a son test de lecture, vu rouge. Il a aussi fait voir que « /e » ne se complète plus seul (`effacer`, `etapes`).
- Le harnais : la séance des aperçus joue sous `--etapes`. Le harnais entier est vert sous ssh (`mesures/historique-reponse-seule-ssh.json`).

Le crate passe à 99 tests avec la vue.
