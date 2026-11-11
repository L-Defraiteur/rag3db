# Rapport de session « rag3weaver archi », 11 novembre 2026

Cette page est écrite pour un agent qui reprend le périmètre sans mémoire de nos échanges.
**archi** est le nom de cette session de travail : elle tient l'IDE web de rag3weaver, l'atelier
et les hooks de Claude Code. Elle est orchestrée par une autre session, **c7** (qui distribue les
lots, donne le feu pour pousser et relit), aux côtés d'**ED** (la session « everything
declarative », qui relit le code), de **G** (la session « optimiseur », qui pose les binaires sur
les postes) et de **96** (la session « mémoire »).

Un **lot** est une unité de travail : une branche, ses témoins, sa passe, son push. Un **témoin**
est un test qui prouve une promesse : on l'écrit d'abord, on le voit échouer (« rouge »), puis on
corrige jusqu'à ce qu'il passe (« vert »). Une **passe** est la suite de vérifications jouée avant
un push. Le procédé est décrit dans la connaissance, page `02-knowledge-dump.md`.

## Les lots atterris sur master

Tous sont poussés sur `origin/master` du dépôt rag3db, entre le 9 novembre au matin et le
9 novembre au soir.

| hash | ce que fait le lot | branche d'origine |
|---|---|---|
| `8efd3eeb0` | un délai sur chaque client HTTP des tests de l'hôte ; ferme le ticket du 8 novembre « un test de l'hôte attend son hôte par un client sans délai », ouvre celui des cinq clients de production | `delai-des-temoins-de-l-hote` |
| `d23af9cd2` | l'atelier lit et écrit : `read_file`, `read_scope`, `replace_text`, `replace_lines`, `insert_lines`, `ask_permission`, la carte d'accord, le journal des décisions | `atelier-lire-ecrire` |
| `b3053bd51` | trois lignes pour Lucie (« À essayer dans ton atelier ») en tête de la page de l'atelier | docs |
| `be319fa75` | une commande mise en attente se décide sur la carte d'accord, et la décision relance le tour ; `accorder_ligne` n'accorde plus que la commande exacte (ticket fermé dans le lot) | `atelier-commandes-sur-la-carte` |
| `3e8c87aa5` | la carte d'un chemin relance aussi son tour | `atelier-chemin-relance` |
| `ab8272deb` | la jauge du contexte écrit son pourcentage, suit chaque appel au modèle, et survit au redémarrage de l'hôte | `jauge-du-contexte` |

Les lots plus anciens de cette session (les hooks de Claude Code, le 7 et le 8 novembre) sont
décrits dans `extension/rag3weaver/docs/07-novembre-2026-hooks-claude-code/`.

G a posé `d23af9cd2` sur le portable le 9 novembre à 13 h 42. Les lots suivants
(`be319fa75`, `3e8c87aa5`, `ab8272deb`) ne sont **pas encore posés** chez Lucie. c7 voulait les
regrouper avec le lot de la session « recherche ».

## Le lot en branche, non poussé sur master : l'IDE par le flux

- **La branche** : `origin/ide-par-le-flux`, sur le portable dans le worktree
  `/home/lucied/.cache/rag3weaver-build/wt-flux`.
- **La demande** : Lucie, le 8 novembre au soir, « que ces trucs-là fonctionnent par SSE, pas par
  polling, voire rendre générique quand on a besoin d'un SSE ». SSE (*server-sent events*) est un
  flux HTTP ouvert, par lequel l'hôte pousse des événements à la page.
- **La page** : `extension/rag3weaver/docs/09-novembre-2026-ide-par-le-flux/01-inventaire-et-forme.md`.
  Elle donne l'inventaire de ce que l'IDE demande à l'hôte, et la forme choisie.
- **Les commits de la branche :**
  - `5ef9a7fcf` : le témoin rouge et la page ;
  - `41ad15263` : l'hôte publie l'état sur le sujet `etat` (`Flux::publier_etat`), la page
    s'abonne par `onEtat` (`ui/ide/lib/host.ts`), et la fin d'un tour ne relit plus rien ;
  - `a9c5a3108` (« wip ») : l'usage du tour se compte au compteur du chat au lieu d'attendre le
    journal. **Ce commit n'a pas été joué.**
- **L'état exact :**
  - Le témoin de la branche est vert sur `41ad15263`. C'est
    `daemon_host::ide_par_le_flux::the_ide_asks_the_host_nothing_while_idle_and_only_its_gesture_for_a_turn` :
    une minute d'inactivité donne zéro requête, un tour donne le seul `POST /ide/chat`.
  - Mais la lib entière est rouge deux fois sur `41ad15263`, sur
    `daemon_host::symboles_cites::tests::un_symbole_cite_se_resout_par_l_index_sans_ouvrir_de_depot`.
    Elle prend 100 à 113 s au lieu d'environ 33 s. Seul, ce test est vert cinq fois sur cinq : c'est
    la charge.
  - La cause probable : à chaque fin de tour, `41ad15263` lançait un fil qui relisait le journal
    des runs toutes les 50 ms pendant 5 s, en attendant l'usage du tour. Dans la lib, des
    centaines de tours le font.
  - `a9c5a3108` remplace ce fil par un calcul immédiat, depuis `chat_meter.totals_for_run`. La
    passe qui devait le vérifier a été tuée (code 137) avant de rendre un verdict.
- **Le prochain pas** : rejouer le témoin, puis la passe complète. La lib doit revenir vers 33 s.
  Si elle reste lente, regarder ensuite `publier_l_etat_de_l_hote`, appelé après chaque
  `RepoIndexed` : il refait tout `GET /etat`.

## Reprendre

Un seul worktree a un travail non fini : celui de l'IDE par le flux. Aucun sur luciepc.

- **Où** : sur le portable, `/home/lucied/.cache/rag3weaver-build/wt-flux`.
- **Branche** : `ide-par-le-flux`, suivie par `origin/ide-par-le-flux`.
- **Dernier commit** : `a9c5a3108`, poussé.
- **Base** : `ab8272deb`. Master a avancé depuis : le script de push rebase lui-même.
- **Fait** :
  - le témoin rouge et la page (`5ef9a7fcf`) ;
  - la publication d'états et `onEtat`, pour lesquels le témoin est vert (`41ad15263`) ;
  - l'usage du tour compté sans attendre le journal (`a9c5a3108`, **non joué**).
- **Reste** :
  - jouer le témoin sur `a9c5a3108` ;
  - jouer la passe complète, et vérifier que la lib entière revient vers 33 s et au vert ;
  - pousser au feu de c7.
- **Le target** : `/home/lucied/.cache/rag3weaver-build/flux`, 11 Gio. Il a été bâti par ce
  worktree ; la passe le réutilise.
- **Le dossier de verdict** : `/home/lucied/.cache/rag3weaver-build/flux-passe/`. Il ne
  contient que le journal et le log de la passe tuée : il n'y a pas de verdict.
- **Les commandes**, depuis `/home/lucied/.cache/rag3weaver-build` :
  1. le témoin seul :
     `rm -rf flux-passe; FILTRE=the_ide_asks_the_host_nothing ETAPE=navseul ./poste lourd bash wt-flux-passe.sh; cat flux-passe/verdict`
  2. la passe complète :
     `rm -rf flux-passe; ETAPE=complete ./poste lourd bash wt-flux-passe.sh; cat flux-passe/verdict flux-passe/journal`.
     Regarder la durée de la lib entière dans `flux-passe/lib.log` : la ligne `test result`,
     `finished in …`. Vers 33 s, c'est bon ; vers 100 s, chercher la cause (voir plus haut).
  3. pour un rouge seul, le départager en le jouant seul cinq fois avec `ETAPE=module FILTRE=<nom>`.
  4. au vert, et au feu de c7 :
     `./poste push archi -- bash wt-flux-push.sh`. Le script :
     - refuse si le verdict n'est pas `VERT` ;
     - rebase sur master ;
     - s'arrête en exit 3 si du Rust est arrivé. Il faut alors refaire l'étape 2, puis
       relancer l'étape 4 ;
     - pousse `HEAD:master`, puis la branche rebasée sous `ide-par-le-flux-2` ;
     - fait les trois contrôles ;
     - efface le worktree et le target.
     
     Ensuite, donner le hash à c7, G et ED.

## Les lots nommés, pas commencés

- **Le découpage au survol de la jauge du contexte.** Au survol, la jauge dirait ce qui pèse
  dans le dernier appel au modèle : les instructions (le message système), les définitions des
  outils, l'historique, et dans l'historique les résultats d'outils. C'est estimé au prorata des
  caractères, puisque le fournisseur ne rend qu'un total. Il est nommé dans
  `extension/rag3weaver/docs/09-novembre-2026-atelier-lire-ecrire/02-la-jauge-du-contexte.md`.
  c7 l'a placé après l'IDE par le flux.
- **Les propositions et le fichier ouvert par le flux.** C'est le même moule que l'IDE par le
  flux, nommé dans sa page.

## Les pages de conception en attente

- `extension/rag3weaver/docs/09-novembre-2026-atelier-lire-ecrire/01-conception.md` (l'atelier lit
  et écrit, puis les commandes et les chemins sur la carte). Tout est sur master. La relecture
  d'ED se fera sur master : ED était bloqué depuis le 9 au matin sur une approbation de Lucie.
  Ses remarques feront un lot de suite. Les trois points que je lui ai soumis :
  - `accorder_ligne` resserré change aussi le oui du superviseur d'un équipier ;
  - la relance du tour est un message d'utilisatrice marqué « (carte d'accord) » ;
  - `Chat.tsx` suit désormais un tour qu'il n'a pas envoyé.
- `extension/rag3weaver/docs/09-novembre-2026-atelier-lire-ecrire/02-la-jauge-du-contexte.md`.
- `extension/rag3weaver/docs/09-novembre-2026-ide-par-le-flux/01-inventaire-et-forme.md`.

## Ce qui reste sur disque

Rien n'a été effacé pour ce passage.

- **Portable**, sous `/home/lucied/.cache/rag3weaver-build/` :
  - `wt-flux` : le worktree de la branche `ide-par-le-flux`, propre, à `a9c5a3108` ;
  - `flux` : son target cargo, 11 Gio ;
  - `wt-docs-archi` : mon worktree de documentation, sans branche (tête détachée) ;
  - `wt-flux-passe.sh` et `wt-flux-push.sh` : les scripts de la passe et du push du lot ;
  - les scripts `wt-*-passe.sh` et `wt-*-push.sh` des lots atterris, que l'on peut copier comme
    modèles.
  - Les autres worktrees de ce dossier (`wt-deplace`, `wt-docs-hooks-essai`, `wt-fuite3`,
    `wt-gabarits`, `wt-juge`, `wt-mf-96`, `wt-react5`) appartiennent à d'autres sessions.
- **Branches sur origin** laissées par mes lots, déjà fusionnées et à ne pas supprimer sans le
  mot de Lucie : `atelier-lire-ecrire` et `-2`, `delai-des-temoins-de-l-hote` et `-2`,
  `atelier-commandes-sur-la-carte` et `-2`, `atelier-chemin-relance`, `jauge-du-contexte`.
  Un suffixe `-2` est la branche rebasée, poussée sous un nom neuf parce qu'on ne force jamais
  un push.
- **luciepc** (l'autre poste) : aucun worktree ni target de cette session. Celui qui s'y trouve,
  `wt-lib-arbre`, n'est pas à moi.

## Ce qui est bloqué

- La relecture d'ED attend une approbation de Lucie.
- Le rouge connu de la CI de master,
  `daemon_host::worktrees::tests::owed_vectors_are_caught_up_on_open_when_the_manifest_declares_it`,
  attend aussi Lucie : le correctif est chez ED. Ce n'est pas à moi.

## Les tickets ouverts qui me concernent

Ils sont sous `docs/rag3weaver-archi/tickets/`, déplacés de `docs/tickets/` le 11 novembre.

- `2026-11-09-cinq-clients-de-production-attendent-sans-delai.md` : cinq clients HTTP hors des
  tests attendent un hôte sans délai. Il attend une décision de conception de Lucie.
- `2026-11-08-le-juge-du-rappel-coupe-sa-reponse-sous-400-jetons.md` : le juge du rappel des
  hooks a un plafond de 400 jetons, que la réflexion de Gemini consomme ; la réponse est coupée.
- `2026-11-07-search-structured-le-score-d-une-ligne-n-est-pas-comparable-d-un-signal-a-l-autre.md` :
  reste le seuil `recall_min_score` du hook, à mesurer.
- `2026-11-07-un-test-de-la-lib-qui-parle-au-service-d-embarquement-attend-sans-delai.md` :
  relevé par moi, sans porteur.

## Les trois prochains pas que je ferais

1. **Finir l'IDE par le flux.** Jouer le témoin sur `a9c5a3108`, puis la passe complète. Si la
   lib entière revient à environ 33 s et au vert, demander le feu à c7, puis pousser par
   `wt-flux-push.sh` (voir « Reprendre »).
2. **Le découpage au survol de la jauge**, avec son propre témoin rouge.
3. **Les propositions et le fichier ouvert par le flux**, au même moule. Puis prendre les
   remarques d'ED quand elles viennent.
