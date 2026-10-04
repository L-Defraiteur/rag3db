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

## Temps 1 — le mode tel qu'il marche aujourd'hui (4 octobre, 18 h 30 à 19 h 05)

Lucie a dit oui pour **un mode déclaré** (« concentrons-nous sur avoir un mode
pour ne pas l'avoir en blob ») ; `BlobBacked` reste le défaut. Mesure du mode
existant, `FtsStorage::LocalFs` (fichiers dans `<base>.fts/`), sans protocole
de validation : dépôt rag3db entier, paquets de 512, base sur disque (btrfs,
NVMe), transaction par paquet validée tous les 4 paquets, moteur bâti à
18 h 03, chaque passe seule sous le verrou de mesure.

| | Blobs dans la base | Fichiers à côté (8 Gio) | Fichiers à côté (4 Gio) |
|---|---|---|---|
| Durée du premier index | **116 s** | 159 s | 164 s |
| `COMMIT` des paquets (et leur point de reprise) | 30,9 s | **6,1 s** | 8,5 s |
| Vidages du plein texte (`flush_fts`) | 9,4 s | **83,8 s** | 90,1 s |
| Pousser les blobs en base | 5,3 s | 0 | 0 |
| Pic de mémoire résidente | 15,0 Go | **8,8 Go** | 8,7 Go |
| Base sur disque | 2 985 Mo | 327 Mo | 327 Mo |
| Dossier du plein texte | — | 687 Mo | 688 Mo |
| Réouverture, première recherche rendue | 2,7 s | **0,4 s** | 0,3 s |
| Passe avec 4 Gio de tampon | échoue (ticket) | — | **passe** |

Comptes égaux (6 927 fichiers, 79 074 scopes, 255 857 relations ; la passe
en base en a quelques dizaines de moins, le corpus bouge).

**Ce que le mode règle déjà** : le point de reprise ne porte plus que les
lignes (`COMMIT` 31 → 6 s : la table des blobs faisait 24 s sur 28 de ces
points de reprise, selon le chronométrage du cœur C++) ; la mémoire tombe de
moitié ; la base fait un tiers ; la réouverture est immédiate ; et **la passe
tient dans 4 Gio de tampon**, là où le mode en base meurt.

**Ce qui ne marche pas encore** :

- **Les vidages du plein texte coûtent 84 s au lieu de 9.** Cause lue dans
  lucivy : son répertoire sur disque (`MmapDirectory`) fait un `fsync` à la
  fermeture de **chaque** fichier écrit (`SafeFileWriter::terminate_ref`,
  `sync_data`). Lucivy le sait : son répertoire à blobs écrit son cache
  « sans fsync », avec ce commentaire : un commit devient « des dizaines de
  fsync (~65 ms chacun sur btrfs) ». C'est exactement ce que le protocole du
  temps 2 doit remplacer : écrire les fichiers d'une génération sans
  `fsync`, synchroniser une fois, puis valider la marque. Gain attendu
  (estimé, non mesuré) : le gros des 84 s, soit une passe vers 80 à 90 s.
- **Rien ne lie les fichiers aux lignes** : un arrêt brutal entre les deux
  laisse un plein texte en avance ou en retard sur la base. C'est le temps 2.
- Le dossier n'est pas nettoyé avec la base, ni rebâti s'il manque.

**Faits pour lucivy** (trace des blobs, passe en base) : 60 vidages ;
153 590 écritures de fichiers reçues par le tampon, 45 158 poussées après
dédoublonnage ; **1 951 Mo poussés pour un index qui fait 687 Mo** une fois
fini — l'index est réécrit environ 2,8 fois pendant le premier index (les
fusions de segments et les générations successives).

**Piste, non codée** : lucivy a déjà une politique de fusion réglable
(`MergePolicy`, `NoMergePolicy`, `IndexWriter::set_merge_policy`,
`merge(segment_ids)`), que rag3weaver n'utilise pas. Sans fusion pendant le
premier index et une seule à la fin, les 1 951 Mo écrits tomberaient vers la
taille de l'index (~700 Mo) plus une fusion finale — estimé, pas mesuré.

## Étape A du protocole, mesurée (4 octobre, 19 h 15 et 20 h)

`FtsStorage::Files` (`src/fts_directory.rs`, `16ab78413`) : un répertoire à
nous, écritures sans `fsync`, une synchronisation à chaque vidage. Même
réglage que plus haut (512, disque, K = 4, naissances, seul sous le verrou).

| Passe | Moteur | Durée | Vidages du plein texte | Rendre durable | Pic | Réouverture |
|---|---|---|---|---|---|---|
| fichiers, étape A | 18 h 03 | **97 s** | 12,5 s | 10,9 s (59 fois) | 8,6 Go | 0,3 s |
| blobs en base, poussée à chaque paquet | 19 h 36 | 113 s | 7,7 s | — | 14,9 Go | 1,2 s |
| blobs en base, poussée une fois à la fin (`c6cf2c263`) | 19 h 36 | 135 s | 8,1 s | — | 14,4 Go | 1,0 s |
| fichiers, étape A | 19 h 36 | **110 s** | 14,9 s | 20,8 s (60 fois) | 8,8 Go | 0,3 s |

- Le mode fichiers reste le plus rapide et de loin le plus sobre, mais son
  avance (19 s à 18 h, 3 s à 20 h) n'est pas stable : la synchronisation a
  pris deux fois plus longtemps sur la seconde passe, sans explication. Une
  passe chacune ; il en faut plusieurs pour trancher.
- La poussée unique des blobs à la fin écrit bien 699 Mo au lieu de
  1 922 Mo et ramène les `COMMIT` de 32 à 7 s, mais elle coûte 52 s à elle
  seule : en tout, plus lent.

## La série alternée (4 octobre, 20 h 19 à 21 h 15)

Trois passes de chaque mode, alternées, même moteur (bâti à 20 h 17 ; joué
avec `RAG3WEAVER_MOTEUR_ANCIEN=1`, un commit du moteur étant arrivé à 20 h 18),
512 sur disque, K = 4, naissances, 8 Gio, chacune seule sous le verrou.
L'état du poste au départ de chaque passe est relevé.

| | Passes | **Médiane** | Étendue | `COMMIT` | Pic | Réouverture |
|---|---|---|---|---|---|---|
| blobs en base, poussée par paquet | 113 · 110 · 107 s | **110 s** | 107–113 | 26–32 s | 14,7–15,1 Go | 1,2–2,2 s |
| fichiers (étape A) | 110 · 95 · 95 s | **95 s** | 95–110 | 6,7–8,1 s | 8,7–8,8 Go | 0,3–0,4 s |

La passe lente des fichiers (synchronisation 21,4 s au lieu de 11) est partie
avec une pression d'entrée-sortie de 10,7 % et des compilations qui
démarraient entre deux passes ; les deux autres, poste calme, font 95 s. Le
disque occupé est plausible, pas prouvé. **Le chiffre de 97 s rendu plus tôt
était une passe unique ; la médiane est 95 s.**
