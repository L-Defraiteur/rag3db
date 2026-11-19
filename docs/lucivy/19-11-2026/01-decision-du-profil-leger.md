# Le profil léger du plein texte : la page de décision (19 novembre 2026)

Pour Lucie, à la demande de c7. La question est de savoir si les index plein texte de rag3weaver doivent garder leurs
positions. Sans positions, un index pèse environ trois fois moins, mais chaque requête relit le texte de ses candidats.
Les chiffres d'aujourd'hui sont au § 7, avec la recommandation (mesure du 19 novembre, 14 h).

## 1. Ce qui est en jeu

- **La place.** Le 14 novembre, sur 89 894 documents de `Lucivy_Scope` (932 Mo de texte), l'index pesait 4,39 Go avec
  positions et **1,44 Go sans** (−67 %). Les positions et leurs postings faisaient l'essentiel de la différence ; le
  dictionnaire et le texte stocké ne changent pas (`docs/lucivy/14-11-2026/02-les-fils-de-lucivy-et-le-profil-leger.md`, § 2).
  Depuis, la maintenance (4.3.8) et les exclusions de fichiers ont déjà réduit la base : `Lucivy_Scope` du dépôt pèse
  aujourd'hui 819 Mo à la tablette. Sans positions, ce serait de l'ordre de 300 Mo ; la mesure dira le vrai chiffre.
- **Le temps des requêtes.** Sans positions, lucivy trouve les documents candidats dans l'index, puis vérifie chaque
  occurrence et calcule chaque span en relisant leur texte stocké. Or rag3weaver demande toujours des spans : ils
  rattachent un hit à son morceau, et servent au bonus de titre et au surlignage. Le 14 novembre, sans positions,
  les requêtes de `search_code` étaient de 5 à 25 fois plus lentes pour un ou deux mots, 150 fois pour quatre mots,
  et la requête longue de l'archiviste prenait 63 s.
- **Ce qui a changé depuis** : la 4.3.11 (19 novembre) corrige le cas pathologique. La requête longue de l'archiviste
  passe de 6,8 s à 0,19 s sur 20 000 fichiers du dépôt (0,15 s avec positions). Les requêtes courtes restent plus
  lentes : de 1,2 à 8 fois sur ce corpus. C'est ce que la mesure de § 5 doit chiffrer sur le vrai index.

## 2. Ce qu'on ne perd pas

- **Les réponses sont les mêmes** : mêmes documents, mêmes spans, avec ou sans positions. Les témoins de lucivy le
  vérifient sur le noyau Linux v7.2, au span près, pour les littéraux, le flou et les regex
  (`test_positions_off::positions_off_answers_literals_like_positions` et `…_fuzzy_and_regex_like_positions`). Les
  phrases entre guillemets passent aussi (vérifiées sur le texte).
- **Les scores** : depuis la 4.3.10, les mêmes documents ont le même score au bit près, quelle que soit la disposition
  des segments.
- **Ce que lucivy refuse sans positions**, et que rag3weaver n'utilise pas : `derived_in_ram`, et un champ texte non
  stocké. Les champs texte de rag3weaver sont stockés.

## 3. Ce que ça coûte

- **Du temps et du CPU à chaque requête**, en proportion du texte des candidats. Ce coût est faible pour un mot rare,
  et plus fort pour un mot court et fréquent (`commit`) ou une requête de plusieurs mots en `contains`.
- **Un rebâti** : l'option se fixe à la création de l'index. Une base existante garde son réglage. Passer au profil
  léger demande de rebâtir le plein texte depuis les lignes, ce que rag3weaver sait déjà faire, sur une copie
  d'abord.

## 4. La forme proposée

- **Aujourd'hui** : `fts_positions` dans le manifeste du backend (`backend.rs`, défaut `true`), pour toutes ses tables,
  à la création. Seul `experiments/mtga/backend/backend.json` le pose à `false`.
- **Proposé** : un profil **par index** dans le manifeste, défaut inchangé (positions gardées). Par exemple, garder
  les positions de `Scope` (ce que `search_code` interroge à chaque appel) et alléger les tables lues plus rarement
  (`File`, `Library`, les tables de la mémoire longue), ou l'inverse selon les chiffres. Le réglage du backend reste
  le défaut de ses tables ; un réglage par entité le remplace pour son index. Le changement de manifeste est déjà
  détecté à l'ouverture (« fts_positions » dans la liste des clés changées) ; un index existant garde son réglage
  jusqu'à son rebâti.
- **Ce qui ne bouge pas** : le défaut, les réponses, l'API de recherche.

## 5. La mesure, prête à partir

- **La copie** : `Lucivy_Scope` de code-rag3db-depot, prise à la tablette le 19 novembre à 12 h 08 (819 Mo, environ
  82 000 documents vivants, segments complets), copiée à luciepc sous `~/.cache/rag3weaver-build/mesure-profil-leger/`.
- **La commande**, sous `poste mesure`, une fois le rang donné par c7 :
  `ssh luciepc 'nohup ~/.cache/rag3weaver-build/poste mesure bash ~/.cache/rag3weaver-build/mesure-profil-leger/mesure.sh > ~/.cache/rag3weaver-build/mesure-profil-leger/mesure.out 2>&1 &'`.
  Elle joue `test_light_profile` de lucivy main (4.3.11), avec les variantes `positions` et `no_positions` :
  - les documents stockés de la copie, réécrits dans un index neuf par profil ;
  - les dix requêtes du 14 novembre (`mesure-profil/requetes-scope.json`, les formes de `build_bm25_query`, dont
    celle de l'archiviste), cinq courses chacune, avec un `HighlightSink`.
- **Les relevés attendus**, pour chaque profil :
  - la taille de l'index par famille (positions, postings, dictionnaire, texte stocké) ;
  - le temps d'indexation ;
  - la latence médiane de chaque requête ;
  - l'égalité des réponses entre profils (`DIFFERENT ANSWERS` sinon).

## 6. Ce que Lucie décide

1. Passer certains index au profil léger, lesquels, sur les chiffres de § 5.
2. Si oui : le réglage par index dans le manifeste (un lot de rag3weaver), puis le rebâti des index choisis sur ses
   bases, sur copie d'abord.

La recommandation viendra avec les chiffres. Ce qui la guidera : le gain en octets contre la latence de `search_code`
sur des requêtes d'un ou deux mots, les plus fréquentes, avec « un poil plus lent » comme limite.

## 7. Les chiffres du 19 novembre, et la recommandation

Mesure jouée à luciepc sous `poste mesure`, sur lucivy 4.3.11 (main 792f514). Elle porte sur la copie de
`Lucivy_Scope` de la base du dépôt : 82 063 documents, **80,7 Mo de texte**. Le 14 novembre, c'était 932 Mo : les
exclusions de fichiers ont retiré les fichiers minifiés et générés.

| profil | sur disque | dictionnaire | positions | postings | texte stocké | indexé en |
|---|---:|---:|---:|---:|---:|---:|
| avec positions (l'actuel) | 865,8 Mo | 439,7 Mo | 171,7 Mo | 210,8 Mo | 43,6 Mo | 12,8 s |
| **sans positions** | **613,4 Mo (−29 %)** | 439,9 Mo | 0 | 130,1 Mo | 43,5 Mo | **8,9 s** |

Les requêtes, en médiane de cinq, avec les spans que rag3weaver demande toujours :

| requête | avec positions | sans positions |
|---|---:|---:|
| 1 mot, contains d1 | 20 ms | 15 ms |
| 1 mot court (`commit`), d1 | 40 ms | 62 ms |
| 1 mot, d0 | 3 ms | 13 ms |
| 2 mots, contains d1 | 22 ms | 48 ms |
| 2 mots, parse | 15 ms | 60 ms |
| 4 mots, contains d1 | 8 ms | 96 ms |
| 4 mots, parse | 29 ms | 89 ms |
| phrase entre guillemets | 2 ms | 4 ms |
| requête longue d1 (archiviste) | 75 ms | 95 ms |
| requête longue d0 (> 900 caractères) | 280 ms | 247 ms |

**Les réponses sont les mêmes** dans les deux profils : mêmes documents, et le même nombre de spans pour chaque
requête (9 794 pour `commit`, 23 009 pour les quatre mots en `parse`).

**Lecture** :
- **La latence n'est plus un obstacle.** Tout reste sous 100 ms. Le pire est « 4 mots, contains » : ×12, mais
  96 ms. C'est le « un poil plus lent » que Lucie accepte. La 4.3.11 et les exclusions ont fait disparaître les
  secondes du 14 novembre.
- **Le gain en place est plus petit qu'annoncé** : −29 %, soit 252 Mo sur cette base, au lieu de −67 %. Ce qui
  pèse aujourd'hui, c'est le **dictionnaire** : 440 Mo, la moitié de l'index, plus de cinq fois le texte. Les
  positions ne font plus que 172 Mo.
- **L'indexation est 30 % plus rapide** sans positions.

**Recommandation** :
1. **Passer les index de code au profil léger**, avec `fts_positions: false` par défaut dans les manifestes de
   rag3weaver. La possibilité de garder les positions index par index reste ouverte, pour qui le demanderait. Gain :
   un quart à un tiers de la place et de l'indexation, pour quelques dizaines de millisecondes par requête. Ce sera un
   lot de rag3weaver, puis un rebâti du plein texte de chaque base, sur copie d'abord.
2. **Le levier suivant est le dictionnaire**, pas les positions : 440 Mo pour 81 Mo de texte. C'est une mesure à faire
   (ce qui y pèse, par champ et par longueur de mot ; le 15 novembre, le vocabulaire en était la cause), avant tout
   changement de format, qui serait une 4.4.
