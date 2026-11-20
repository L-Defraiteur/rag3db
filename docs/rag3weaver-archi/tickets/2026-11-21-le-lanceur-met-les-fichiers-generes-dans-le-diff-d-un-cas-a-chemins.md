# Le lanceur met les fichiers générés dans le diff d'un cas à chemins

- **État** : ouvert, le 21 novembre 2026 ; pour archi
- **Famille** : produit
- **Pour** : archi
- **Gravité** : faible ; le banc juge un diff qui porte des fichiers que l'équipier n'a pas écrits
- **Atteignable en service** : non, c'est l'outillage de la campagne (`tools/campagne/lanceur.py`)
- **Touche rag3weaver** : non
- **Signalé par** : banc, au jugement du rejeu de nuit C, le 21 novembre 2026

## Ce qui s'est passé

Le diff du rejeu C porte `tools/campagne/__pycache__/lanceur.cpython-314.pyc` et
`test_lanceur.cpython-314.pyc`. Ce n'est pas l'équipier qui les a mis dans le diff : il n'indexe rien.
C'est le lanceur. Le cas déclare `paths: ["tools/campagne"]`, et `prepare` n'archive que ces chemins.
Le `.gitignore` de la racine n'est donc pas dans l'arbre. Ensuite, `diff_of` fait `git add -A` dans un
index temporaire, et les `.pyc` laissés par le témoin Python y entrent.

## Le pas exact qui bloque

`prepare` (`archive_into` avec `paths`) n'emporte pas les `.gitignore` de la racine et des dossiers
parents des chemins.

## Ce qu'il faut faire

`prepare` emporte toujours les `.gitignore` du parent qui couvrent les chemins du cas : celui de la
racine, et ceux des dossiers ancêtres. Le diff ne porte alors plus ce que le dépôt ignore.

## Comment le prouver

Rouge d'abord, dans `tools/campagne/test_lanceur.py` : un cas à `paths` dont le dépôt ignore
`__pycache__/` à la racine ; un `.pyc` posé dans l'arbre préparé n'est pas dans le diff.
