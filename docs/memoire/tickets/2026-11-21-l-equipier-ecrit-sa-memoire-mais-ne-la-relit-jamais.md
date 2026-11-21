# L'équipier écrit sa mémoire longue, mais ne la relit jamais

- **État** : ouvert, le 21 novembre 2026.
- **Famille** : produit
- **Gravité** : ce qu'un équipier a appris à la clôture d'un plan ne lui revient pas à la tâche suivante ; sa mémoire
  longue est une sortie, pas encore une entrée.
- **Atteignable en service** : oui. Tout équipier d'une tâche dont le backend déclare `bilans.memoire`.
- **Touche rag3weaver** : oui. Le pas de l'équipier (`src/daemon_host/teammate.rs`), sa frontière, et les outils de
  lecture du gabarit memory.
- **Pour** : session mémoire (96), avec archi pour le pas de l'équipier.
- **Signalé par** : c7, le 21 novembre 2026, à la lecture de la page docs/memoire/21-11-2026/01-ce-que-cet-agent-sait.md.

## Ce qu'on voit

L'équipier écrit dans la mémoire déclarée par deux chemins :

- **ses tours** : `ranger_apres_le_tour` (`rangement.rs`), avec le run `<tâche>:<parent>`, donne `put_turn` puis
  `archive_slice` passé `slice_tokens` ;
- **à la clôture d'un plan** : `file_closed_plan` (`teammate.rs`) appelle `get_fiche`, puis `put_fiche`, `put_source`
  (`session: teammate-<tâche>`), `put_apport` (section `plan`, `L1`) et `relier_apport`.

Il ne la relit jamais. Aucun `recall_fiches`, `recall_apports` ni `whois` n'a lieu à une frontière de pas ni à
l'ouverture d'une tâche. La recherche de `recall`, `frontier` et `frontière` dans `teammate.rs` et `teammate_tools.rs`
ne trouve que la boîte aux lettres.

## Ce qu'on attend

À l'ouverture d'une tâche, ou à une frontière de pas, l'équipier reçoit ce que sa mémoire sait de sa demande : les
fiches et apports qui la touchent, bornés, au même titre que le contexte qu'il reçoit déjà. L'agent est sa clé stable
`agent:<clé>` (Lucie, 25 octobre 2026) : sa mémoire le suit d'une tâche à l'autre.

## Préalables

- **L'attribution** (feu vert de c7, 21 novembre 2026) : l'hôte pose `author = agent:<clé>` sur `put_turn`,
  `put_source` et `put_apport`. Sans elle, le rappel ne distingue pas ce que cet agent a appris.
- **La forme du rappel** est à décider avec archi : un nœud déclaré du pas (tout déclaratif), un outil que l'équipier
  appelle lui-même, ou les deux. Elle est aussi à borner en jetons, puisqu'elle s'ajoute au contexte de chaque pas.

## Témoin attendu (rouge d'abord)

Un équipier clôt un plan sur un sujet et range son L1. Une seconde tâche du même agent, sur le même sujet, reçoit ce L1
dans son contexte d'ouverture, et une tâche d'un autre agent ne le reçoit pas.
