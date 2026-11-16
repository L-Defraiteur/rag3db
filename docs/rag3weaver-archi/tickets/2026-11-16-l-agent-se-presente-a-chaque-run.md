# « Je suis Algareth » à chaque run

- **État** : ouvert, le 16 novembre 2026 ; à faire par Codex (passation d'archi).
- **Famille** : produit
- **Pour** : archi (passation à Codex)
- **Gravité** : gêne : la présentation se répète à chaque pas.
- **Atteignable en service** : oui, toute tâche nommée.
- **Touche rag3weaver** : oui
- **Signalé par** : c7, campagne D6 du 16 novembre 2026 (S5)

## Ce que c'est

L'invite d'un pas dit « In the first answer of a task, introduce yourself by this name, once »
(`chat.rs:709`). Chaque pas repart d'un fil neuf : chaque pas se croit le premier.

## Ce qu'il faut faire

Ne dire la phrase de présentation qu'au **premier pas d'une tâche** (aucune ligne de l'équipier dans
la boîte), et sinon seulement « Your name is <nom>. ». Le nom reste dans l'invite.

## Comment le prouver

Rouge : faux modèle, une tâche nommée, deux pas ; l'invite du pas 2 n'a plus « introduce yourself ».
Passe comme le plafond.

## Taille et blocage

1 h. Aucun.

