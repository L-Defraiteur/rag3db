# Rapport de session : recherche, 18 novembre 2026

Session « recherche » (97). Les consignes viennent de c7 (rag3db-5a), qui relaie les mots de Lucie. Les passes
tournent à luciepc, sous `poste lourd`. Les pushes passent par `poste push … push_garde.sh` ; après chacun, trois
contrôles : status vide, aucun rebase en cours, FETCH_HEAD = HEAD.

## Ce qui est sur master

| Hash | Lot | Ce qu'il fait |
|---|---|---|
| f29e417c43 | lot 1 | La garde FTS sur les deux chemins de synchronisation, et la vraie porte du test 853 (`la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel`). |
| e1e6db7487 | entrée 1 + Lag par couche, en pile | Les comptes notés du plein texte sont datés et rafraîchis par la maintenance nocturne. Le retard se compte par (cellule, couche) : une lecture n'attend que la couche de sa vue et nomme les autres (`other_views`). |
| 17e59fa9da | entrée 2 | `search_code` balaie de nouveau avant tout index, sauf sous une vue explicite (au mot de Lucie). |
| b2272d070c | lot 3 | Pendant un index complet de sa vue, une lecture dit « un index écrit ta vue » au lieu de se taire. |
| e6c0307291 | C | Une première indexation pose les relations avant les dérivations. `usages` complet à 26 s au lieu de 42 sur rag3weaver/src, et à 89 s au lieu de 151 sur le dépôt. |

Docs sur master : la page `02-les-outils-de-reference-avant-l-index.md` (mesures, décision de Lucie, conception
de B, essai borné), le ticket de B, et le ticket de visibilité.

## Prêt, en attente du « à toi »

- **`usages-generations-visibility`** (ebaa98a8d6), passe `passe-visibilite-224612` VERTE. Sous générations,
  `usages` montrait la déclaration d'un conteneur retiré : rouge en f560496d6d, puis correctif (les six sélections
  d'`usage_nodes` passent par `traduire_select`). Le lot ferme son ticket.
- **`memory-image`** (6a655cb36b), passe `passe-image-195744` VERTE. `MemoryImage`, l'image en mémoire de B.

## En cours

- **B, le repli des outils de référence avant l'index.** Lucie : « C d'abord, puis B quand même ». L'essai borné a
  retenu la forme de codeparsers : une connexion mémoire qui exécute aussi les écritures (96 formes en huit
  familles). Partage :
  - **moi** : le dialecte et la connexion mémoire sur `MemoryImage` ;
  - **codeparsers** : le schéma sans dérivée (`register_code_schema_for_references`), la sonde de parité
    `the_image_draws_the_same_links_as_the_base` (rouge à dessein), puis la mesure.
  - Branche de départ : `codeparsers/memory-image-resolution`.
- **À surveiller** : la sonde `probe_index_timeline` sur le dépôt entier au prochain index des hôtes, pour voir
  C en service.

## Ménage

- **Tablette** : 31 worktrees de Codex retirés, 5,13 Gio rendus.
- **luciepc** : 9 retirés, 1,48 Gio rendus. Les 7 autres ont un sous-module initialisé et demandent `--force`, que
  ma garde a refusé : ils restent pour Lucie, à son réveil.

## Sondes à garder

- `probe-index-timeline` (`tests/probe_index_timeline.rs`) : la chronologie d'un premier index, à la seconde.
- `probe-surface-on-refs` (`tests/probe_memory_surface.rs`) : les formes d'instructions d'une première indexation.
