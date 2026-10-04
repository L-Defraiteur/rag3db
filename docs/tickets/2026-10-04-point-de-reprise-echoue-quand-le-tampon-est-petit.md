# Le point de reprise échoue pendant une indexation quand le tampon du moteur est petit

- **État** : ouvert
- **Gravité** : blocage (l'indexation s'arrête, la base doit être rouverte)
- **Atteignable en service** : oui — tout poste dont le tampon du moteur est
  plus petit que ce qu'un point de reprise veut tenir en mémoire
- **Touche rag3weaver** : oui (première indexation d'un dépôt, poussée des
  blobs du plein texte)

## Ce que c'est

Avec un tampon du moteur de 2 Gio, la première indexation du dépôt rag3db
(6 900 fichiers, 62 Mo de texte) meurt vers le 2 000ᵉ fichier : un point de
reprise déclenché pendant la poussée des blobs du plein texte ne trouve plus
de mémoire, échoue, et la base refuse toute requête jusqu'à sa réouverture.

Message exact :

```
index persistence failed: ingest: must reopen: Query execution failed:
Runtime exception: Buffer manager exception: Unable to allocate memory!
The buffer pool is full and no memory could be freed! A checkpoint of this
database failed, so it must be closed and reopened before it is used again:
the checkpoint did not complete.
```

## La recette

Pas encore de recette en Cypher brut. Par le produit, base sur disque :

```
RAG3DB_BUFFER_POOL_SIZE=2147483648 \
RAG3WEAVER_ESTIMATE_REPO=1 RAG3WEAVER_ESTIMATE_BATCH_FILES=512 \
RAG3WEAVER_ESTIMATE_DB_DIR=~/.cache/rag3weaver-build/base-mesure \
./run_e2e.sh --test e2e_estimate ce_depot_est_cherchable_par_mots
```

Échec après 130 s, entre 2 048 et 3 072 fichiers (4 octobre 2026, moteur
`e3daa2836`). Sans la borne (tampon par défaut du moteur, poste de 121 Go),
la même passe finit en 823 s avec un pic de 13,1 Go de mémoire résidente.

## Le témoin

`extension/rag3weaver/tests/e2e_estimate.rs`,
`ce_depot_est_cherchable_par_mots_avant_ses_vecteurs`, avec les variables
ci-dessus. Ce n'est pas un témoin de batterie : il prend le dépôt entier.
Une seule exécution à ce jour.

## La cause

Pas établie. Ce qu'on sait : l'erreur sort de la persistance de l'index plein
texte (`flush_blob_store`), donc d'un point de reprise posé pendant qu'on
écrit de gros BLOB ; le moteur a son point de reprise automatique actif avec
un seuil de 16 Mio de journal (`database.h`). Ce qu'on ne sait pas : ce que le
point de reprise épingle en mémoire (tout le journal ? les pages des BLOB ?),
ni la plus petite taille de tampon qui passe.

## Ce qu'il faut pour le fermer

- Une recette minimale au niveau du moteur (une table à colonne BLOB, des
  `MERGE` de plusieurs dizaines de Mo, un tampon étroit).
- Savoir si c'est le moteur (un point de reprise qui devrait vider par
  morceaux) ou nous (des blobs trop gros en une instruction, à découper).
- Dans rag3weaver, au minimum : dire à l'utilisateur quoi faire (taille du
  tampon, réouverture) au lieu d'un échec d'ingestion nu ; `estimate`
  pourrait prévenir quand le dépôt dépasse ce que le tampon permet.
- Une première indexation de ce dépôt qui passe avec un tampon de 2 Gio.
