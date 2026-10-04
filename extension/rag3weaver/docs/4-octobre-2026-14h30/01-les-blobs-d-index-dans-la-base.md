# Les blobs du plein texte doivent-ils vivre dans la base ?

4 octobre 2026 — session embarquements, demi-page pour Lucie, portée par
l'orchestration. Rien n'est codé ; c'est une question de conception, posée
par les mesures de la journée.

## Ce qui se passe aujourd'hui

L'index plein texte (lucivy) de chaque entité est rangé **dans la base**, une
ligne par fichier d'index dans la table `_index_blobs` (`FtsStorage::BlobBacked`,
le défaut). À chaque ouverture, lucivy recopie l'index entier dans un cache
local (`/tmp/rag3weaver_cache/<pid>/…`, un tmpfs sur ce poste) et lit ce cache ;
à chaque validation, les fichiers changés repartent dans la base. Le moteur
garde en plus deux générations de fichiers supprimés.

Ce que cela coûte, mesuré aujourd'hui sur le dépôt rag3db (paquets de 512,
base sur disque) :

- **346 s sur 823** passaient en points de reprise déclenchés par ces
  poussées, au seuil par défaut du moteur (28 s une fois le seuil relevé) ;
- les deux échecs de tampon observés (« buffer pool is full … A checkpoint
  of this database failed ») suivent de près une poussée du plein texte —
  suspect, pas prouvé (une sonde en Cypher brut tourne) ;
- la base pèse 3 à 3,7 Go pour 62 Mo de texte ;
- chaque fichier de segment passe par 5 à 6 copies en mémoire entre
  l'écrivain et les pages du moteur (lecture du code).

## L'alternative

Les fichiers d'index **à côté de la base**, comme `.wal`, `.shadow` et
`.extensions` déjà : un dossier `<base>.fts/`, lu directement par lucivy. En
base, une seule chose : **une marque de génération** qui dit quel état des
fichiers correspond aux lignes validées.

Elle existe à moitié : `FtsStorage::LocalFs` ouvre déjà un index lucivy dans
un dossier (`FsShardStorage`), pensé pour le navigateur. Ce qui manque, c'est
le lien de validation avec la base.

## Ce qu'on y perd

- **La durabilité liée à celle des lignes.** Aujourd'hui une ligne et son
  plein texte sont validés ensemble. Avec des fichiers à côté, il faut un
  protocole : écrire les fichiers d'une génération, les synchroniser sur
  disque, puis valider la marque en base ; à l'ouverture, une génération sans
  marque validée est jetée et rebâtie à partir des lignes. C'est ce que fait
  déjà le rattrapage des vecteurs, pas un territoire neuf, mais c'est du code
  et des tests d'arrêt brutal.
- **La copie d'une base en un geste.** Il faudrait copier un dossier, pas un
  fichier. Le `.wal` impose déjà de copier deux fichiers.
- **Le lecteur d'un autre processus.** Un second processus qui ouvre la base
  doit aussi voir les fichiers de la même génération. Un lecteur qui lit des
  fichiers en cours de remplacement est le cas à garder (des fichiers jamais
  modifiés en place, seulement ajoutés puis retirés, comme les segments
  lucivy, le rendent tenable).
- **Le serveur sans disque local** (une base distante, un service) : ce mode
  a été choisi le 2 septembre pour que l'index survive à la perte du disque
  local. Il resterait possible comme option.

## Ce qu'on y gagne

- Plus de points de reprise du moteur pour des octets d'index, ni de pages du
  tampon occupées par des blobs : le tampon ne porte plus que les lignes.
- Plus de recopie de l'index entier à chaque ouverture : lucivy lit ses
  fichiers en place (mmap), et l'ouverture d'un gros dépôt devient immédiate.
- Une base de la taille de ses lignes, et un index qu'on peut jeter et
  rebâtir sans toucher aux données.

## Ce que font les autres

- **PostgreSQL** range chaque index dans ses propres fichiers, à côté de ceux
  des tables, sous le même journal : l'index n'est pas une valeur dans une
  table, et la cohérence vient du journal commun.
- **Elasticsearch et Tantivy** gardent leurs segments sur disque, immuables ;
  un commit écrit un petit fichier qui liste les segments vivants. C'est
  exactement la « marque de génération » proposée, mais dans un fichier.
- **Neo4j** range ses index plein texte (Lucene) dans des dossiers à côté du
  magasin, et les rebâtit s'ils manquent.

## Recommandation

Essayer l'alternative, sur la base de `LocalFs` : les fichiers dans
`<base>.fts/`, une marque de génération validée en base avec les lignes, le
rebâti à l'ouverture si les deux divergent. Avant de décider, deux mesures :
la sonde en cours (le tampon meurt-il des blobs ?) et une passe du dépôt
entier en `LocalFs` (durée, mémoire, taille). Lucie, le 2 octobre : « on n'est
pas obligé d'avoir tout dans un seul fichier ».
