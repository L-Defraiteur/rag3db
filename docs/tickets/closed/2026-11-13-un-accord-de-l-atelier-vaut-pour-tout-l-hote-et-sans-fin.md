# Un accord de l'atelier vaut pour tout l'hôte, toutes conversations, et sans fin

- **État** : corrigé (13 novembre 2026, branche atelier-accord-borne, archi). Témoin
  `a_grant_belongs_to_its_conversation_and_expires`, rouge puis vert.
- **Famille** : produit
- **Gravité** : réponse fausse — un accord donné à une conversation ouvre le chemin aux autres (y compris en écriture sur un dépôt tenu par l'utilisatrice), et jusqu'au redémarrage de l'hôte
- **Atteignable en service** : oui, dès qu'un accord est donné sur une carte d'accord et qu'une autre conversation d'atelier vise le même chemin
- **Touche rag3weaver** : oui (`extension/rag3weaver/src/daemon_host/atelier_fichiers.rs` : `Accords.ouverts` 41, 79-80, `accorde` 188-191, `atelier_peut_ecrire` 179-185)
- **Pour** : rag3weaver archi (l'atelier qui lit et écrit, son auteur)
- **Signalé par** : everything declarative, relecture de l'atelier sur origin/master 73c7f15e1, à la demande de l'orchestration (13 novembre 2026) ; lignes de origin/master à cette date

## Ce que c'est

`Accords.ouverts` est une liste de l'hôte : `accorde(chemin, mode)` regarde tous les accords
ouverts, sans conversation ni backend ni échéance. La conception
(`extension/rag3weaver/docs/09-novembre-2026-atelier-lire-ecrire/01-conception.md`, ligne 76) dit
que la réponse « ouvre ce chemin à la RootPolicy de la conversation ».

## Le cas

La conversation A demande l'écriture de `~/git_workspaces/rag3db/x` ; l'utilisatrice accorde. La
conversation B (autre atelier ou autre session) écrit ensuite sous ce chemin sans rien demander.
Et une demande laissée en attente se décide des heures plus tard, puis relance une conversation
passée à autre chose.

## Le témoin à écrire

Deux conversations d'atelier ; un accord à A ; B est refusée sur le même chemin. Une demande plus
vieille que son échéance est refusée à la décision.

## Ce qui le fermerait

Un accord porté par sa conversation (et son backend), avec une échéance ou une fin de session ; le
témoin vert.
