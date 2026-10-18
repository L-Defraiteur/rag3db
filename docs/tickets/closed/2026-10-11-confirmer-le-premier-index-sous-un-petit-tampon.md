# Confirmer qu'une première indexation de ce dépôt passe sous un tampon de 2 Gio

- **État** : **fermé** le 18 octobre 2026 : la passe va au bout sous 2 Gio, en fichiers et en blobs (§ « Rejoué »). Ouvert le 11 octobre, une confirmation pour l'arbre principal
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

## Rejoué (arbre principal, 18 octobre 2026) : vert sous 2 Gio, en fichiers et en blobs

Luciepc, master `c6bb094fb` (lib du magasin 0fc7dd10f), worktree `rag3db-tampon`, la recette du
ticket du 11 octobre (`RAG3WEAVER_ESTIMATE_REPO=1`, `e2e_estimate
ce_depot_est_cherchable_par_mots_avant_ses_vecteurs`, base sur disque), qui passe à côté de la
borne, sous `poste lourd`, services d'embarquement locaux, régime doux. 6 318 fichiers, 57 Mo de
texte compté par la passe, 687 467 liens. Journaux : `~/.cache/rag3weaver-notes-A/tampon/` sur
luciepc. Le pic est relevé par le test toutes les 100 ms ; il tombe chaque fois pendant la pose
des relations.

| passe | plein texte | tampon | résultat | durée | pic au fil de l'eau | anonyme à 1 024 / 3 072 / 5 120 / 6 318 fichiers |
|---|---|---|---|---|---|---|
| paquets de 512 | fichiers | 2 Gio | **vert** | 69 s | 7 907 Mo | 2 359 / 4 576 / 5 569 / 6 288 Mo |
| paquets de 512 | fichiers | 4 Gio | **vert** | 68 s | 7 677 Mo | 2 365 / 4 493 / 5 265 / 5 967 Mo |
| paquets de 512 | blobs | 2 Gio | **vert** | 124 s | 9 212 Mo | 2 635 / 5 253 / 6 801 / 7 580 Mo |
| paquets de 512 | blobs | 4 Gio | **vert** | 122 s | 9 805 Mo | 2 673 / 5 609 / 7 269 / 8 405 Mo |
| paquets de 2 048, un COMMIT par paquet (le défaut d'`index`) | fichiers | 4 Gio | **vert** | 58 s | 7 982 Mo | 3 769 à 2 048 ; 6 446 à 6 144 |
| paquets de 2 048, une seule transaction (`RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION=tous`) | fichiers | 4 Gio | **vert** | 56 s | 8 242 Mo | 3 699 à 2 048 ; 6 599 à 6 144 |

**La passe en blobs, rouge le 11, est verte le 18**, sous 2 comme sous 4 Gio. Le chemin du 4
octobre ne casse donc plus sur ce dépôt. Le
[ticket du COPY de blobs](../2026-10-11-copy-de-blobs-en-point-de-reprise-force-sous-un-petit-tampon.md) reste ouvert : son témoin moteur (un gros COPY de blobs)
n'est pas celui-ci.

La borne du produit, recalculée sur ces passes, est fermée dans le même commit
([ticket](2026-10-18-la-borne-du-tampon-refuse-le-premier-index-de-ce-depot.md)).
