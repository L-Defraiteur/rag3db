# Le tampon de 256 Mio est plein à la première écriture d'un backend

- **État** : ouvert, cause non trouvée
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
