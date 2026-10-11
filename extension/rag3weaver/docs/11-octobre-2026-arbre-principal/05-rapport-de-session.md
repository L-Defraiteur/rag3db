# Rapport de session — arbre principal, 11 octobre 2026 (après-midi et soir)

*Pour l'orchestration et la session qui reprendra l'arbre principal. État à 20 h 15.*

## Sur master

| hash | quoi |
|---|---|
| `cc1254ac9` | Migration (a) de la marche 1 bis : l'identité d'une ligne est (cellule, identité déclarée) ; la cellule par défaut garde la formule d'avant. |
| `b92d809ae` | `scripts/check_tickets.py` : l'index des tickets recompté sur les fichiers, appelé par `check_features.sh`. |
| `eb389901c` | Le rendu d'impact est le même d'un catalogue neuf à l'autre : `parcourir` trie chaque niveau avant le budget (ticket fermé). Cinquième jeu `rag3db-native` dans `check_features.sh`, deux sondes gardées par la feature `code`. |
| `6a5c19b03` | Le relevé à la main d'`e2e_usages` suit les sources (deux vrais appels arrivés : `node.rs`, `concurrent.rs`). |
| `db1c5836e` | `e2e_usages` ajouté au ticket « e2e_impact dépend de la forme du code vivant ». |
| `66752a44e` | Pointeur codeparsers `35faa0f` : les C++ anonymes (287 → 0), ticket fermé. |
| `cad384056` | Pointeur codeparsers `a6111cc` : nom seul et segments de module, ticket fermé. |
| `a0f3e3653` | Deux routes de l'hôte : `POST /backend/open` relit un manifeste modifié, `POST /backend/close` ferme et lâche la base ; ticket fermé. |

**A9** : les dix e2e du produit qui ouvrent un backend, verts sur `verrous-a9b` (6a4ebaa89 puis 4ee70e386 avec A8), 116 tests, artefact prouvé par ldd et md5 ; « fini » dit au cœur, la bascule est sur master (`a0a79016e`).

**La lib partagée** (`build/lecteurs-csv`) rebâtie deux fois, après les « fini » de 96, 97, ab et 6f :
- 17 h 39 : A8 et A9, librag3db.so `9f9d9409…`, libvector `a444b9c0…`, sur `e60f2342e` ;
- 20 h 08 : sur `faa00f279` (`b4655da39` et `CALL build_id()` compris, B2 non arrivé à 20 h), librag3db.so `ae3d781aa49ffadc694165364412f10e`, libvector `081c4dec9224aaab32b5b57488d6dece`.

GNU ld écrit un nouvel inode à chaque édition de liens : un processus qui tient la lib (l'hôte de Lucie, les clients MCP) garde l'ancienne image sans tomber ; il la prend au redémarrage.

## Relu et accordé (à d'autres)

- F : `impact-run-plan`, `cypher-brut` lots 1 à 4 (le ticket du Cypher brut fermé), `voisinage-pg` (la table d'arrivée par `rel_info`, la porte des relations par le catalogue).
- codeparsers : `reexport-au-site` (repli avec clé, `saturating_sub`), `envoi-virtuel-2` (une base indexée avant la branche, `git` hors du verrou, l'arbre indexé par cellule, le pas répété refusé en dernier).
- everything declarative : W3 (le retard de l'index par cellule ; la course `note`/`done` ; le `done` après l'écriture réussie), et le correctif de la montre (clé du registre = curseur).
- G : `embarqueur-absent-portes` (la raison dans le reçu ; ticket « service injoignable à l'ouverture » à mon nom).
- mémoire longue : la session d'un client mort (pid + instant de démarrage), `build_id` du moteur dans /sante.

## Ce qui a coûté, et la règle qui en est sortie

- Un push « déjà vert » repoussé à chaque `.md` arrivé sous le verrou : la commande rebase et repousse dans le même tour (règle 3 de `poste push`).
- Un push tenu 28 min derrière une mesure exclusive : `poste push --lourd` prend le poste avant le push (G) ; un refus 75 se relance (`relancer.sh`).
- `origin/master` partagé entre worktrees : lire master par `FETCH_HEAD` de son propre fetch.
- Un rebase sans conflit peut laisser un en-tête de l'index faux : `check_tickets.py` l'attrape ; la commande recompte et reprend le commit de tête.
- Vert = le vrai code de sortie, lu sans tube, et aucune ligne ROUGE/REFUS/FAILED dans la sortie écrite.

Les scripts de push sont hors du dépôt : `~/.cache/rag3weaver-notes-A/pousser4.sh` et `relancer.sh`.

## La suite, dans l'ordre de l'orchestration

1. L'extension vector par le nom (un gabarit livré la cherche relativement à lui-même) ; le dire à G.
2. Les manques du MCP du code : le nom qualifié (`Classe::methode`) ; « tests par requêtes : non suivis » dans impact ; `search_code` sans correspondance lexicale ; la portée d'une question ; les verrous pris par le départ ; les verrous tenus par les appelants.
3. La relecture de la commande « ajoute / retire cet arbre » de codeparsers (sur les routes `a0f3e3653`).
4. Le pas d de la marche 1 bis : marques par cellule, `oublier_les_aretes_du_fichier` filtré par cellule, le registre « indexation en cours » entre cellules.
5. Le ticket du service d'embarquement injoignable à l'ouverture ; le workspace en lecture seule.
