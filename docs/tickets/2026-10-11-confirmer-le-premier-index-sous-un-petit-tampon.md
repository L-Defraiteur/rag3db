# Confirmer qu'une première indexation de ce dépôt passe sous un tampon de 2 Gio

- **État** : ouvert (11 octobre 2026) — une confirmation, pour l'arbre principal
- **Gravité** : blocage, s'il revenait (l'indexation s'arrête, la base est à rouvrir)
- **Atteignable en service** : oui, sur un poste au tampon étroit
- **Touche rag3weaver** : oui, c'est une passe du produit

## Ce que c'est

Le défaut moteur est corrigé en `47a283e6c` (ticket fermé `closed/2026-10-04-point-de-reprise-echoue-quand-le-tampon-est-petit.md`) : un point de reprise
n'échoue plus quand une table de BLOB dépasse le quart du tampon. Il reste à le voir dans le
produit, sur la passe qui l'avait montré : la première indexation du dépôt entier sous 2 Gio.

## La recette

Celle du ticket fermé, avec la lib de `47a283e6c` ou d'après :

```
RAG3DB_BUFFER_POOL_SIZE=2147483648 \
RAG3WEAVER_ESTIMATE_REPO=1 RAG3WEAVER_ESTIMATE_BATCH_FILES=512 \
RAG3WEAVER_ESTIMATE_DB_DIR=~/.cache/rag3weaver-build/base-mesure \
./run_e2e.sh --test e2e_estimate ce_depot_est_cherchable_par_mots
```

De jour, sur luciepc, en régime doux (orchestration, 11 octobre) : c'est une passe qui embarque.

## Pour le fermer

La passe va au bout sous 2 Gio. Relever aussi sa mémoire résidente au plus haut, pour le ticket
`2026-10-11-memoire-residente-au-dela-du-tampon-sous-des-blobs.md`.

## Joué (arbre principal, 11 octobre 2026, 10 h 28 à 10 h 34, luciepc)

Lib commune de luciepc à `4820e9154` (bâtie à 05:36 ; elle porte `47a283e6c`), worktree
`rag3db-arbre` à `c9362d9e8`, la recette ci-dessus sous `poste lourd`, en régime doux, avec les
services d'embarquement locaux. La mémoire résidente a été relevée toutes les 5 s sur le processus
du test.

| plein texte | résultat | durée | mémoire résidente au plus haut |
|---|---|---|---|
| **en fichiers** (le défaut d'une base neuve depuis la bascule du 10 octobre) | **vert** : 7 160 fichiers, 80 911 scopes, tampon de 2,0 Gio | 74 s | 7,24 Go |
| **en blobs** (`RAG3WEAVER_FTS=blobs` : une base d'avant la bascule, le chemin du 4 octobre) | **rouge** : « valider le paquet : must reopen … The buffer pool is full and no memory could be freed! A checkpoint of this database failed » | 41 s | 7,54 Go |

- **La passe en fichiers ne prouve pas le correctif.** Sans blobs en base, elle ne passe pas par le
  chemin qui échouait le 4 octobre. Seule la passe en blobs le joue.
- **En blobs, le défaut est toujours là**, plus loin : la passe avait fait 3 072 fichiers au dernier
  relevé (le 4 octobre, elle mourait vers le 2 000ᵉ). Il échoue à la validation d'un paquet, dont
  le COPY passe par le point de reprise forcé en blobs. Base : 584 Mo sur disque à la mort.
- **La mémoire résidente** pour un tampon de 2 Gio, relevée par le test lui-même (lignes
  `[mémoire]`), fichiers puis blobs :

  | fichiers faits | 1 024 | 2 048 | 3 072 | 7 160 |
  |---|---|---|---|---|
  | en fichiers (Mo) | 2 574 | 3 584 | 4 432 | 6 941 |
  | en blobs (Mo) | 2 721 | 4 160 | 5 313 | — |

  Presque tout est anonyme, et ça croît d'environ 1 Go par millier de fichiers, bien au-delà du
  tampon. C'est pour le ticket `2026-10-11-memoire-residente-au-dela-du-tampon-sous-des-blobs.md`,
  et ça vaut **aussi en fichiers**.

**État** : la confirmation est faite pour le défaut d'une base neuve (en fichiers). Le défaut reste
rouge en blobs, donc pour une base d'avant la bascule. Le ticket reste ouvert pour ce cas. Journaux
sur luciepc : `~/.cache/rag3weaver-notes-A/confirmation/e2e.log` et `e2e-blobs.log`.
