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
| le plafond du juge | ff2725f90 | `bilans.max_per_hour` (20), le saut dit au journal et dans la boîte |
| `code-tree add --from` | 0683cc474 | par nom ou par chemin, quel que soit le nom du manifeste (`rag3db.json`) |

Pages : 01 (l'équipier, agent de code fonctionnel), 02 (une session, plusieurs backends ; le
vocabulaire git ; le modèle session › tâches › espaces ; la forme de la barre), 03 (l'atelier ouvre
les dépôts). Tickets ouverts par moi : le témoin du chat dans la page, les tests sous charge, la
garde de `match` Rust (pris par codeparsers), les sept défauts du chat (portés et fermés par le lot
de hooks-essai), le plafond du juge.

## Demain matin (8 novembre)

- Les figements de la lib entière sous charge et le rouge intermittent du réacteur
  (`a_batch_collapses_a_burst_into_one_run`, 1/8) : le réacteur est mon module (ticket de
  codeparsers) ; témoin sous charge d'abord, puis la cause.

## Relu pour d'autres

Le lot du chat de hooks-essai (A, B, la page, premier-jeton : la course de `Flux::publier`),
`tache-hors-depot` (la racine `templates`, le refus de `declare_set`), le pont d'archi (la règle
« seule une ligne adressée lance l'équipier », mon commit repris).

## Ce qui reste, dans l'ordre

1. Les figements sous charge du réacteur (demain matin).
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

## La nuit du 8 novembre (de 1 h à 7 h)

Poussé sur master, dans l'ordre de c7, chaque lot après une passe complète VERT puis un rejeu
court sur le master du moment, sous `poste push`, avec les trois contrôles :

| lot | hash | ce qu'il fait |
|---|---|---|
| compte des suites de run_e2e.sh | 91543d7f0 | `--test e2e` est le binaire, le filtre désigne la suite ; « 1 sur 1 » pour une suite jouée |
| équipier jamais muet | aff3611fb | un pas sans step_done écrit sa réponse ; la request dit d'où part la tâche |
| commandes par la porte déclarée | ad496c462 | run_command dans une tâche à porte, l'approval dans la boîte, le oui l'accorde ; la course de la boîte (un oui posté pendant le dernier pas se perdait) |
| ticket des essais réels | eb154bf5f | les constats de c7 aux essais 3 à 5 |
| état d'attente | b0301a0e3 | une question sans réponse suspend l'équipier, sans appel au modèle, ni réussi ni raté ni dans les 40 |
| bac à sable de cargo | 927ae9680 | la chaîne Rust en lecture, `sandbox.env` avec `{task_dir}` et `{repo_target}`, un target partagé par dépôt sous verrou et plafond, /dev/null ouvert, `taches purge` |

Prêt, VERT, et en attente de la fenêtre de 7 h 30 : le fil par pas (un pas repart d'une session
neuve avec la demande en cours et ses step_done) et l'arrêt d'inactivité retenu par un équipier en
route (branche `equipier-fil-par-pas-2`).

La mesure qui a décidé du fil par pas, prise sur les journaux des runs de l'essai 3 : l'entrée du
premier appel d'un pas montait de 5 275 jetons au pas 1 à 122 242 au pas 10, et le pas 5 en
envoyait environ 950 000 en trois minutes. La mesure d'après se prendra sur un essai de c7 une
fois le lot dans l'hôte de Lucie.

Rouges de la nuit, chacun nommé et départagé :

- un SIGSEGV à la sortie du binaire e2e après e2e_equipier vert (une fois, vert seul) ;
- e2e_juge où Gemini range le `fichier` obligatoire en description_trompeuse plutôt qu'en
  parametre_devrait_etre_optionnel (deux fois sur cinq sur la tête du lot, vert deux fois sur
  master seul et trois fois sur la tête : variance du classement) ;
- deux trous de la même course dans la boîte de l'équipier, vus par la passe du lot des
  commandes sous la charge de la lib entière, corrigés dans le lot avec un témoin rouge sans eux.

## Ce qui reste, après la fenêtre

1. La reprise sur 429/503 : hors de max_failed_steps, une fenêtre de quota avant de reprendre.
2. Le libellé du crochet, et le gabarit `motif-ailleurs` introuvable hors du crate (ticket des
   essais, point 8).
3. Le juge qui note chaque pas comme la tâche entière, la consigne qui ne retient pas la
   clôture, la reprise à dire (ticket des essais, points 4, 5 et 7).
4. Puis la liste d'avant : réacteur sous charge, page 03, noms à plusieurs définitions, tâche
   jetable, témoins sous charge.
