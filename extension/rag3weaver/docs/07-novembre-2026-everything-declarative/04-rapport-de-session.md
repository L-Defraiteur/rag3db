# Rapport de session — everything declarative, 7 novembre 2026

## Ce qui est sur master

| lot | hash | ce qu'il pose |
|---|---|---|
| IDE en React, vue 5 | 43d7489ee | les tâches de l'agent et leur relecture, l'équipier |
| vue 6 | 08d10babf | les bilans de session par fiche |
| vue 7 | 7897d2c96 | l'arbre, le symbole par sections, le graphe, le DAG vivant, les réactions, les propositions |
| vue 3b | a4ffea748 | l'édition dans la page (CodeMirror du magasin embarqué) ; l'ancienne page n'a plus de vue |
| la boîte adressée | b9b47ee12 | id stable par ligne, `to`, `reply_to`, le relais MailboxLine sur /ide/flux, pending / seen / list |
| l'étanchéité de l'équipier | 75d8d6d1e | le backend pris à chaque appel d'outil, le repli sur les fichiers, `ChatModel` sur le manifeste préparé, la clé `teammate`, un pas raté qui n'arrête plus |
| la reprise après un arrêt de l'hôte | 4ae8c93a8 | l'équipier interrompu reprend à sa frontière au redémarrage, `teammate.resume_after_restart` |

Pages : 01 (l'équipier, agent de code fonctionnel), 02 (une session, plusieurs backends ; le
vocabulaire git ; le modèle session › tâches › espaces ; la forme de la barre), 03 (l'atelier ouvre
les dépôts). Tickets ouverts par moi : le témoin du chat dans la page, les tests sous charge, la
garde de `match` Rust (pris par codeparsers), les sept défauts du chat (portés et fermés par le lot
de hooks-essai), le plafond du juge.

## En file

- **Le plafond du juge** : `juge-plafond-2`, a20a9ebdd — `bilans.max_per_hour`, au mot de c7 après
  codeparsers (Lucie allume `bilans.memoire` ce soir).

## Relu pour d'autres

Le lot du chat de hooks-essai (A, B, la page, premier-jeton : la course de `Flux::publier`),
`tache-hors-depot` (la racine `templates`, le refus de `declare_set`), le pont d'archi (la règle
« seule une ligne adressée lance l'équipier », mon commit repris).

## Ce qui reste, dans l'ordre

1. Le plafond du juge (en file).
2. La première marche de la page 03 : les outils intégrés `attach_repo` / `open_branch` /
   `declare_repo`, un appel qui crée le worktree, le manifeste, l'index, l'événement sur le flux.
3. Le lot d'IDE des noms à plusieurs définitions (choisir avant de dessiner).
4. Tâche jetable (l'effacement à la fusion, les orphelines).
5. Les témoins sous charge (attendre par un repère).
6. Les lots 2 à 6 de la page 01 (commandes déclarées, Claude Code ↔ équipier, sous-équipiers,
   `tool_issues`, la mémoire non bloquante), puis la page de direction visuelle avec le thème de tty.

## Ce que j'ai mal fait, et ce qui le garde

- Un push de la reprise lancé à la main après le seul rejeu d'un témoin, avec des suites sans
  verdict : rejouées après, vertes ; le pas du push refuse désormais sans `l2/verdict`.
- Des scripts de test courts lancés hors `poste lourd` : corrigés.
- Un fichier de témoin laissé indexé dans mon worktree de luciepc par le « rouge d'abord » : mes
  scripts remettent l'index et la copie depuis HEAD.
- Un commit par chemins qui oubliait un fichier neuf (deux fois) : `git add` d'abord, `status` lu après.
