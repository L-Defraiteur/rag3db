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
