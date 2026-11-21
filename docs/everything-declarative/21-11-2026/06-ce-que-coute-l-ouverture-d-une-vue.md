# Ce que coûte l'ouverture d'une vue, et ce que la mesure ne dit pas

21 novembre 2026, session everything declarative. Une mesure demandée par c7, après le relevé d'arbre : « l'écriture
d'un fichier dans l'index coûte 1,0 s en release, dont 0,5 à 0,7 s dans les dérivations ».

## Les chiffres

**La sonde d'arbre** (`history_layers::tests::sonde_ecriture_en_couche`, 20 novembre, 22 h 36). 388 fichiers écrits en
couche : 379 s, dont 272,7 s dans le drain des dérivées. Le profil de ce drain est sous les lignes `[drain-profile]` du
journal, car ses nœuds ne passent pas par `[ingest-profile]` :
- `derive_embed` prend 263,8 s, dont `write_ms` = 262,3 s, pour 20 641 vecteurs ;
- le reste est petit : le rendu 5,4 s, l'insertion des morceaux 2,2 s, les liens moins d'une seconde.

**Mon rejeu** (21 novembre, sonde reprise en règles 13, release, lib `cb3624f3ec`, sous `poste lourd`, 412 fichiers) :
613 s, dont `derive_embed` 407 s et `write_ms` 404 s, pour 20 677 vecteurs (19,5 ms par vecteur). L'embarquement, à
hachage, tourne en parallèle et prend 171 s.

**La sonde de l'écriture** (`history_layers::tests::sonde_ecriture_des_vecteurs`). Sur une copie fraîche de la source
(161 873 morceaux), 1 024 morceaux par paquets de 64 :
- la requête des porteurs (un `MATCH` par `_uuid`, la clé primaire, puis `IS NOT NULL`) : **0,21 ms par vecteur** ;
- **le `SET` du vecteur dans une colonne sous index HNSW** (`ScopeContent_Chunk_vec__granite_278m`, `mu 30`, `ml 60`,
  `efc 200`, cosinus) : **36,6 ms par vecteur**.

**Ce qui est établi** : le temps est la maintenance de l'index HNSW à chaque écriture d'un vecteur, côté moteur. Ce
n'est ni une requête qui parcourt une table, ni une requête mal posée. Et la lib au moteur HNSW neuf n'y change rien.

## Pourquoi ces chiffres ne sont pas ceux d'une vraie vue

**Ne pas relire ces chiffres comme le coût d'ouverture d'une vue en service.** Deux faits les faussent.

1. **Le modèle de l'embarquement.** La copie source est entièrement sous granite : tous ses morceaux portent un vecteur
   `embedding__granite_278m`. Les sondes, elles, embarquent avec `HashEmbedder(768)`, dans une seconde colonne neuve et
   indexée. Rien n'y était reprenable, puisque ce n'est pas le même modèle, et chaque morceau écrit dans cette seconde
   colonne payait l'index.
2. **La version de l'analyse.** Les `File` de la copie portent quatre empreintes d'analyse (`8d14c58b…`, `cd2b5d77…`,
   `add33a35…`, et 8 125 vides). Aucune n'égale celle du binaire de la sonde en règles 13 (`58808e89…`). La sonde
   d'arbre était, elle, en règles 12. Le binaire **rend donc chaque contenu autrement** que la base : 13 811 à 14 349
   contenus neufs, aucun reconnu (`unchanged=0`, `dense_known=0`).

**Ce que cela change.** Une version de scope (`ScopeContent`) est repérée par son contenu : deux occurrences au même
contenu la partagent, avec ses morceaux et ses vecteurs. Une vraie vue, ouverte par un hôte aux mêmes règles que sa
base, ne rend neufs que les scopes réellement changés par le diff. Ses morceaux et ses vecteurs neufs se comptent sur ces
seuls scopes, pas sur tous les scopes des fichiers touchés. Les 400 s mesurées sont celles d'un **repassage** de
412 fichiers, sous un autre modèle.

## Ce qui reste vrai, et qui décide

- **Le coût unitaire tient** : 36,6 ms par vecteur écrit sous HNSW. Tout ce qui écrit beaucoup de vecteurs le paie : un
  repassage, une montée de règles, un nouveau modèle, une première vue d'un gros diff.
- **Les pistes**, relayées par c7 :
  - **l'insertion groupée dans HNSW, et le coût d'`efc`** : au moteur (hotfixs), après son lot en cours ;
  - **la reprise par empreinte de texte** : la conception d'embarquement. Elle évite le calcul du modèle, mais pas
    l'écriture sous l'index. Elle vaut pour un texte identique dans un contenu différent (un fichier copié ou déplacé),
    que le partage par contenu ne couvre pas.
- **À mesurer avant de conclure** : une vraie vue sous granite, aux mêmes règles que sa base. Il faut une copie
  repassée aux règles du binaire et la sonde avec l'embarqueur du service, ou, plus simple, le temps d'ouverture d'une
  vue relevé par l'hôte de service sur un diff réel.

## Les copies et les sondes

- La source : `mesures/repetition-versions-code-rag3db-depot-20261119-194739/copie`, à la tablette.
- Mon rejeu : `mesures/ed-sonde-deux-passes-20261121-091416`. La sonde des vecteurs :
  `mesures/ed-sonde-vecteurs-20261121-102958`. Toutes deux sont jetables à 72 h.
- La sonde reprise en règles 13 : branche `sonde-ecriture-en-couche-2`, qui porte aussi `sonde_ecriture_des_vecteurs`
  et `probe_reuse_by_text_hash`. Elle ne va pas sur master.
