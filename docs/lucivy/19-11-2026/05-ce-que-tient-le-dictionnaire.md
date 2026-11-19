# Ce que tient le dictionnaire de lucivy

Session lucivy, 19 novembre 2026. La mesure du profil léger (`01-decision-du-profil-leger.md`, § 7) a montré que,
sans positions, le **dictionnaire** pèse 440 Mo, la moitié de l'index, pour 81 Mo de texte. Cette page dit d'où
viennent ces octets et quel levier rendrait quoi. Aucune ligne de code n'est décidée ici.

## La mesure

- **Index** : une copie de `Lucivy_Scope`, l'index de code de la base du dépôt (copie de la tablette, 19 novembre à
  12 h 08, 82 063 documents, 4 shards, avec positions). C'est la copie déjà prouvée pour la mesure du profil léger,
  recopiée dans `~/.cache/rag3weaver-build/mesure-dictionnaire/` à luciepc. Seul le plein texte est lu : la base
  rag3db n'entre pas dans cette mesure.
- **Outil** : `lucivy_core/tests/test_dictionary_composition.rs`, un test ignoré, branche lucivy
  `measure-dictionary` (3133385, sur la 4.3.11). Il ne fait que lire.
- **Commande** :
  `LUCIVY_DICT_INDEX=<copie> cargo test --release -p lucivy-core --test test_dictionary_composition -- --ignored --nocapture`,
  sous `poste mesure`, le 19 novembre à 15 h 14.

## Où sont les octets

Les fichiers `dict-*` des quatre shards, par champ et par extension :

| champ | `.sfx` (suffix FST) | `.termtexts` | `.pidx` | total |
|---|---:|---:|---:|---:|
| content | 292,0 Mo | 89,2 Mo | 8,1 Mo | 389 Mo |
| docstring | 17,8 Mo | 6,1 Mo | 0,5 Mo | 24 Mo |
| name | 10,7 Mo | 2,8 Mo | 0,2 Mo | 14 Mo |

Le champ `content` fait 90 % du dictionnaire, et son suffix FST en fait les deux tiers. Il n'y a pas d'anciennes
générations oubliées : chaque shard a un dictionnaire principal d'environ 103 Mo et trois à cinq couches de moins de
1,4 Mo chacune.

## Ce que sont les entrées de `content`

**6 036 996 entrées** dans les quatre shards, soit 48,9 Mo de texte de termes. Chaque entrée coûte en moyenne
48 octets de suffix FST, et 15 octets de `.termtexts` (le texte, plus 4 octets de méta et environ 1,5 octet
d'adresse).

- **Le texte d'un terme n'est pas un mot.** Lucivy indexe des *jetons étendus* : le contenu, son séparateur et
  quelques octets empruntés au jeton suivant. 90 % des entrées portent un séparateur et un emprunt. Le même
  contenu donne donc une entrée par contexte : `foo(`, `foo.`, `foo `…
- **Le même contenu, sept fois.** Par shard, il y a 852 198 contenus distincts (par moitié, en minuscules) pour
  6 036 996 entrées : **7,1 entrées par contenu**. C'est le premier multiplicateur. Il réunit les variantes de
  séparateur, d'emprunt et de casse ; la mesure ne les sépare pas encore.
- **Le même contenu, dans plusieurs shards.** Chaque shard a son propre dictionnaire : 457 981 contenus distincts
  au total, 852 198 si l'on somme les shards, soit **×1,86**. 77 450 contenus sont dans les quatre shards.
- **Les deux moitiés.** 62 % des entrées sont des morceaux (le découpage de chaque jeton), 38 % des mots entiers.
- **Des jetons courts.** 55 % des entrées ont un contenu de 4 octets ou moins, et 93 % de 8 octets ou moins.
- **Nature du contenu.** Lettres : 74 %. Chiffres seuls : 10 %, soit 611 301 entrées. Vide (un séparateur seul) :
  6 %. Non-ASCII : 5,5 %. Mélange : 3,4 %. Hexadécimal de 8 caractères ou plus : 0,5 %. Identifiants de 32 octets
  ou plus : 0,1 %.
- **Des entrées mortes : aucune à compter.** 749 entrées sur six millions ne sont nommées par aucun segment vivant.
  Une compaction du dictionnaire ne rendrait rien.

Un écart reste à comprendre : dans chaque shard, les segments vivants nomment environ un millier d'identifiants de
plus que le dictionnaire n'a d'entrées (1 534 002 contre 1 533 013 dans le shard 0). C'est un compte d'ensemble,
pas encore une liste. Le contrôle `dictionary_check` dira s'il s'agit d'identifiants inconnus ou d'une couche que
la mesure ne lit pas.

## Les leviers, et ce qu'ils rendraient

Ce sont des estimations, en supposant la place proportionnelle au nombre d'entrées. Aucune n'est mesurée.

| levier | ce qu'il change | estimation sur `content` | coût |
|---|---|---|---|
| **1. Une entrée par contenu, au lieu d'une par contexte** | Séparateur et emprunt sortent de la clé du dictionnaire et vont dans les postings (ou un fichier à côté) | entrées ÷ 7 : de 389 Mo vers environ 60 Mo, si la place suit les entrées | Format neuf (une 4.4). La recherche à travers plusieurs jetons et le flou lisent aujourd'hui le texte étendu : c'est le cœur de lucivy à revoir |
| **2. Un dictionnaire pour tous les shards** | Les quatre dictionnaires partagent leurs contenus | ÷ 1,86 : environ −180 Mo | Architecture : les shards écrivent en parallèle, il faudrait un dictionnaire partagé et ses verrous |
| **3. Moins de shards pour un index de code** | 4 shards pour 82 000 documents ; à 2 shards, moins de doublons | entre −60 et −100 Mo, à mesurer | Réglage de rag3weaver, sans format neuf ; un rebâti de l'index ; à vérifier sur le parallélisme de l'indexation |
| **4. Ne pas indexer les nombres seuls** | 10 % des entrées | environ −35 Mo | Choix de produit : on ne trouverait plus `8080` dans le code |
| 5. Compacter les entrées mortes | — | rien (749 entrées) | — |

## Ce que je recommande

1. **Mesurer d'abord le levier 1, sans rien changer.** Séparer les sept variantes : combien viennent de la casse,
   du séparateur, de l'emprunt. Puis construire un dictionnaire sur le contenu seul à côté, pour voir sa taille réelle
   (le suffix FST ne suit pas forcément le nombre d'entrées). C'est le seul levier qui peut diviser le dictionnaire
   plusieurs fois, mais aussi le plus cher : il touche ce qui fait la justesse de `contains` à travers les jetons.
   S'il se fait, ce sera une 4.4, décidée par Lucie sur les chiffres.
2. **Le levier 3 tout de suite après, parce qu'il ne coûte qu'un rebâti.** Mesurer la même copie réindexée à 1, 2 et
   4 shards (taille, temps d'indexation, latence des requêtes du banc). Il peut se combiner avec le rebâti sans
   positions prévu à la fenêtre de G.
3. **Expliquer l'écart d'un millier d'identifiants** par `dictionary_check`, avant toute autre conclusion sur les
   identifiants.
4. Ne pas toucher aux nombres (levier 4) sans une demande de Lucie : c'est retirer des réponses.
