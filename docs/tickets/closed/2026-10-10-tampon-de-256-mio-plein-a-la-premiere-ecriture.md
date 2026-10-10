# Le tampon de 256 Mio est plein à la première écriture d'un backend

- **État** : fermé — non reproduit sur master (moteur `e0fad1325` + rag3weaver master) sous 256 Mio et 32 fils, sur les deux postes (seconde session cœur C++, 10 octobre 2026, décision de l'orchestration)
- **Gravité** : blocage (la base doit être rouverte ; le script échoue)
- **Atteignable en service** : oui, avec un tampon de 256 Mio
- **Touche rag3weaver** : oui (backend, première écriture)

## Ce que c'est

`scripts/test_backend_persistence.py` et `scripts/test_backend_sparse.py` posent eux-mêmes
`RAG3DB_BUFFER_POOL_SIZE=268435456` (256 Mio). Sur master, les deux rendent :

> Buffer manager exception: Unable to allocate memory! The buffer pool is full and no
> memory could be freed!

Le backend dit « la base doit être rouverte » et s'arrête. L'échec arrive **à la première
écriture**, un seul enregistrement : `put_note` pour persistence, la création pour sparse.
Ce n'est donc pas un gros paquet qui déborde.

Isolé par la session recherche le 10 octobre au soir, au commit de master `001054116`.
- Lib commune de 17 h 27 : moteur `aeec6888f`, avec `ff9bad960`, sans `d10b92306`.
- Service d'embarquement de luciepc, `bge-m3` (1 024 dimensions) enregistré.
- python3 du système.

Sous le même tampon et le même montage, `test_backend_harness`,
`test_backend_lifecycle_batch`, `test_backend_must_reopen` et `test_backend_snapshot`
passent. Journaux : `~/.cache/rag3weaver-build/tampon-256/`.

## Ce que ma batterie couvrait

Rien : ces scripts Python ne sont pas joués par `run_e2e.sh`. La batterie du basculement
(74 suites vertes) ne dit donc rien d'eux.

## Les suspects, non départagés

- **Les défauts basculés** (`1b0d5483b`, `21a1d67c5`) : le plein texte en fichiers pour une
  base neuve, la transaction par paquet active, les paquets de 2 048. Une première écriture
  d'un seul enregistrement ne fait pourtant pas de gros paquet.
- **Le COPY journalisé par défaut** (`ff9bad960`) : le journal d'une transaction vit en
  mémoire, qui est celle du tampon.
- **La dimension 1 024** (`bge-m3`) et l'index vectoriel, par rapport aux scripts qui
  passent.

## Recette, à écrire

Rejouer `test_backend_persistence.py` :
1. sur master ;
2. puis avec chacun des anciens défauts, un à la fois : `RAG3WEAVER_FTS=blobs`,
   `RAG3WEAVER_TX_PAR_PAQUET=0`, `RAG3WEAVER_BATCH_FILES=64` ;
3. puis avec `CALL force_checkpoint_on_copy=true`.
Le premier qui rend le vert nomme le coupable.

## Pour le fermer

Selon la cause : corriger ce qui gonfle le tampon à la première écriture, ou, si 256 Mio
n'est plus réaliste sous les nouveaux défauts, le dire dans la page des défauts et remonter
le tampon de ces scripts. Témoin : les deux scripts verts sous le tampon retenu.

## Non reproduit (10 octobre 2026, seconde session cœur C++)

Le premier `put_note` du gabarit notebook sur base neuve est un `COPY` d'une ligne dans la table
qui porte `embedding__bge_m3` en `FLOAT[1024]` (session de l'arbre principal, par lecture :
`ingest_entities_jusqu_a`, `catalog.rs` ~5590-5631, depuis `c7067475d`) ; ni la transaction par
paquet ni les paquets de 2 048 ne s'y appliquent.

Ce qui a été joué :
1. **Dans le moteur nu, sur luciepc** (sonde jetable) : un `COPY` d'une ligne sous 256 Mio, clé
   entière ou chaîne, vecteurs de 0, 384 ou 1 024 dimensions, avec ou sans `CREATE_VECTOR_INDEX`,
   1 à 32 fils, `COPY` forcé ou journalisé — 36 cas verts. La réservation d'avance du groupe local
   du `COPY` (131 072 lignes par fil, `node_batch_insert.cpp:121-123`) n'est pas en cause.
2. **Le script, sur luciepc** : backend bâti depuis master contre la lib commune (`e0fad1325`),
   24 fils, au défaut, avec `RAG3WEAVER_INGESTION_LIGNE_A_LIGNE=1` et avec `RAG3WEAVER_FTS=blobs` —
   le premier `put_note` passe dans les trois (le script s'arrêtait plus loin, faute du module
   Python `mcp`).
3. **Le script, sur le poste principal** : backend bâti depuis master contre la lib de la passe
   rouge (17 h 27, `aeec6888f`), copie de l'extension vector de l'arbre principal (21:26:03, md5
   `9ba45dd1e79f26e0c74dbe9c29c21f28`, 912 048 octets), 32 fils, le python du venv-mcp —
   `test_backend_persistence.py` passe en entier. `test_backend_sparse.py` passe son premier
   `put_note` et échoue plus loin, à la recherche (`search_notes` avec le signal `sparse`), sur un
   autre défaut, de rag3weaver : « le graphe n'a pas pu être construit : duplicate node name:
   sparse » (ticket de la session de l'arbre principal).

Le nombre de fils est écarté, et la lib de la passe rouge aussi. Restent deux différences avec la
passe rouge de la session des embarquements (20 h 13) :
- l'**extension vector** : celle de 17 h 27, bâtie avec la lib, d'avant l'élagage (`c841d507c`) ;
  elle a été réécrite à 21:26 par un bâti non identifié et n'existe plus ;
- le **code de rag3weaver** : `001054116` contre master (Hop, Select et Count, l'embarqueur absent
  entre-temps).

## S'il revient

Une bissection côté rag3weaver entre `001054116` et master, contre la lib du jour (session de
l'arbre principal), avec `RAG3WEAVER_TRACE_CYPHER=1` pour nommer la requête ; puis les requêtes une
à une dans le moteur nu sous 256 Mio. Le remède générique à l'extension d'un autre bâti chargée
sans un mot : `2026-10-05-extension-chargee-au-rejeu-sans-controle-de-bati.md`.
