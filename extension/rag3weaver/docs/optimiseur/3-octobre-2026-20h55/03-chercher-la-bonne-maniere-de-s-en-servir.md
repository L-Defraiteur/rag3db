# Les modèles de décision : chercher la bonne manière de s'en servir

**3 octobre 2026, session « optimiseur ».** Suite exploratoire de
[la mesure](02-les-modeles-de-decision-mesures.md). Lucie : « faut trifouiller
et aussi diagnostiquer ce qu'il voit vraiment […] ça reste une
expérimentation, faut pas courir avec avant d'avoir trouvé si une bonne
manière de s'en servir existe ». Ce document ne désigne pas de vainqueur et
ne recommande rien pour le produit : il dit ce qui bouge quand on change la
façon de poser la question.

> **Corrigé par [le document 04](04-hors-du-jeu-qui-l-a-vu-naitre.md).** Rejouées sur des jeux qui ne sont
> pas de ma main, la forme à 15 sur 16 et le seuil de 0,07 ne tiennent pas, et le « 6 sur 6 » de GLiNER
> non plus. Ce qui suit reste vrai du jeu sur lequel il a été mesuré.

**Ce qui ressort : la façon de poser la question compte plus que le modèle.**
Sur les mêmes textes et les mêmes sujets, JevK5 rejoint le bon sujet 2 fois
sur 16 quand on lui fait comparer deux noms, et 15 fois sur 16 quand on lui
demande où ranger un texte en posant l'alternative « créer ce sujet neuf ou
rejoindre un existant ». Entre les deux, des formes qui paraissent
raisonnables font 4, 8 ou 13 sur 16.

Tout ce qui suit est JevK5-4B en Q8, servi depuis l'autre poste (carte libre,
par tunnel), sauf mention. Les sorties brutes et les scripts sont dans
`banc/`.

## 1. Ce que le modèle voit vraiment

Le texte exact envoyé, pour la paire « écritures parallèles » / « verrous par
clé » :

```
<|im_start|>system
Apply the supplied criterion to the supplied evidence. Choose exactly one listed option. Respond with only its uppercase letter, with no explanation or reasoning.<|im_end|>
<|im_start|>user
{"evidence": "Topic A: écritures parallèles\nTopic B: verrous par clé", "criterion": "Do these two topic names refer to the same topic?", "options": [{"letter": "A", "description": "same: both phrases name the same topic, worded differently or in another language"}, {"letter": "B", "description": "variant: related topics: one is a part, a mechanism or a special case of the other"}, {"letter": "C", "description": "unrelated: two different topics"}]}<|im_end|>
<|im_start|>assistant
<think>

</think>

```

Le modèle écrit alors un jeton, et on lit les log-probabilités des lettres.
Nos deux textes arrivent dans le champ `evidence`, le critère dans
`criterion`, les options dans un tableau où chaque lettre porte « clé :
description ». C'est le gabarit de l'auteur, nous ne l'avons pas modifié.

| paire | attendu | jetons les plus probables (log-probabilité) | masse sur A, B, C |
|---|---|---|---|
| reprise après un point de reprise interrompu / récupération quand le checkpoint a été coupé (juste) | même | A −0,09 · C −2,80 · B −3,59 · D −7,89 | 99,9 % |
| régime doux de la carte graphique… / limiter la charge du GPU… (fausse) | même | A −0,97 · C −1,00 · B −1,37 · D −8,22 | 99,9 % |
| écritures parallèles / verrous par clé (« variante » ratée) | variante | C −0,15 · A −2,28 · B −3,25 · D −7,30 | 99,9 % |

- **La liste fermée tient** : 99,9 % de la probabilité tombe sur les lettres
  offertes. Le modèle ne cherche pas à répondre autre chose.
- **La faute sur la paire du milieu est une hésitation, pas une erreur
  franche** : les trois lettres sont à 38 %, 37 % et 25 % avant la
  température. C'est la zone incertaine, et elle se voit.
- **Sur « verrous par clé », il est sûr de lui** (86 % pour « sans rapport »).
  Rien dans les deux noms ne dit qu'un verrou par clé sert aux écritures
  parallèles : il faut le savoir.

## 2. La formulation, sur les 42 paires de sujets

| ce qu'on change | exactitude à trois choix | « même » trouvés (15) | « variante » trouvées (15) | verdicts changés | AUC de P(même) |
|---|---|---|---|---|---|
| la base | 28 | 13 | 3 | — | 0,97 |
| critère : « Are topic A and topic B the same topic? » | 32 | 14 | 6 | 5 | 0,97 |
| critère : « Decide how topic B relates to topic A. » | 30 | 11 | 7 | 7 | 0,95 |
| critère : « Should the notes of A and B be filed under one single topic? » | 27 | 8 | 7 | 12 | 0,93 |
| options sans description | 25 | 13 | 0 | 7 | 0,94 |
| descriptions courtes | 29 | 14 | 3 | 6 | 0,95 |
| descriptions longues | 29 | 11 | 6 | 6 | 0,96 |
| critère et options en français | 23 | 11 | 0 | 8 | 0,93 |
| trois exemples dans le critère | 27 | 11 | 4 | 7 | 0,97 |
| « sous-sujet » au lieu de « variante » | 29 | 11 | 6 | 6 | 0,95 |
| les cinq autres ordres des options | 26 à 30 | 13 à 14 | 0 à 5 | 4 à 6 | 0,94 à 0,96 |

« Sans rapport » est trouvé 12 fois sur 12 dans toutes les configurations.

- **Le verdict bouge, le score beaucoup moins.** Une reformulation change de
  4 à 12 verdicts sur 42 ; l'AUC de P(même) reste entre 0,93 et 0,97. Ce qui
  est fragile est le choix de l'option la plus probable, pas l'ordre des
  paires.
- **Le français n'aide pas** : 23 contre 28, et plus aucune « variante »
  trouvée. Le critère gagne à rester en anglais même pour des textes
  français.
- **Les exemples dans le critère n'aident pas** (27 contre 28).
- **Les descriptions comptent pour « variante » seulement** : sans elles, zéro
  variante trouvée ; avec des longues, six.
- **L'ordre des options compte un peu.** Sur les six ordres, la première
  option est choisie 76 fois, la deuxième 83, la troisième 93 : un léger
  recul devant la première. 34 paires sur 42 gardent le même verdict sous
  les six ordres. Sur les mémoires à quatre choix, l'effet est plus net :
  17, 14 et 15 justes sur 20 selon l'ordre.

## 3. « Variante » est-elle une mauvaise question ?

Les quinze paires étiquetées « variante », sous seize formulations à trois
choix :

| paire | jugée variante | même | sans rapport | lecture |
|---|---|---|---|---|
| écritures parallèles / verrous par clé | 0 | 0 | 16 | étiquette douteuse |
| concurrent writers / per-key locking | 0 | 0 | 16 | étiquette douteuse |
| écritures parallèles / per-key locks | 0 | 0 | 16 | étiquette douteuse |
| upstream pull requests to burn / AI disclosure line in a pull request | 0 | 0 | 16 | étiquette douteuse |
| poids des modèles / regenerating the granite weights from ONNX | 0 | 0 | 16 | étiquette douteuse |
| synchronisation par périmètre / missing-ratio threshold of a snapshot | 0 | 0 | 16 | étiquette douteuse |
| mémoire longue de l'agent / deduplicating memories at write time | 1 | 0 | 15 | étiquette douteuse |
| index vectoriel / entry points of the HNSW graph | 3 | 13 | 0 | étiquette douteuse, côté « même » |
| vector index / deleting a row from the vector index | 12 | 0 | 4 | tenue |
| full-text search / tokenizer used by the full-text index | 10 | 0 | 6 | tenue |
| modèle d'embarquement par défaut / choix du petit modèle au premier index… | 8 | 4 | 4 | instable |
| embedding daemon / GPU duty cycle of the embedding daemon | 8 | 0 | 8 | instable |
| index vectoriel HNSW / points d'entrée de l'index vectoriel hors transaction | 7 | 6 | 3 | instable |
| synchronisation par périmètre / suppression des lignes sorties de leur périmètre | 6 | 10 | 0 | instable |
| journal d'écriture anticipée (WAL) / WAL illisible après un arrêt brutal | 6 | 9 | 1 | instable |

**Huit de mes quinze « variantes » sont jugées autrement avec constance,
quelle que soit la formulation.** Je les note comme étiquettes douteuses, pas
comme fautes. Ce qu'elles ont en commun : le lien entre les deux sujets ne se
lit pas dans les noms. Les deux qui tiennent (« vector index » / « deleting a
row from the vector index ») portent le nom de l'un dans l'autre.

Les autres découpes de la même décision :

| découpe | résultat |
|---|---|
| deux oui/non enchaînés (« même sujet ? », puis « l'un est-il une partie de l'autre ? ») | 24 sur 42 ; aucune variante trouvée : le second oui/non répond toujours non |
| « sous-sujet dirigé » (B dans A, A dans B) | sur les 15 variantes : « B est un sous-sujet de A » 9 fois, « sans rapport » 6 fois, jamais l'inverse |
| deux options : même / différent | 36 sur 42 ; **une seule fusion à tort** ; 10 vraies « même » trouvées sur 15 |

La découpe à deux options est celle que le modèle tient le mieux sur des
paires de noms. Elle laisse cinq doubles non reconnus.

## 4. Les petits encodeurs, servis comme leurs auteurs le montrent

Je soupçonnais d'avoir mal employé GLiNER. J'ai relu les exemples des cartes
et rejoué dans leur forme.

| modèle | forme | résultat sur les 42 paires |
|---|---|---|
| Laya | notre usage (état en texte, critère en question) | 19 justes |
| Laya | la forme de sa carte (état en objet, champs cités, descriptions courtes) | 12 |
| Laya | deux options à clés neutres A/B, le conseil de sa carte pour les oui/non | 15 : il répond « même » 42 fois sur 42 |
| Laya | sa primitive oui/non | 15 : il répond « même » 42 fois sur 42 |
| GLiNER | notre usage (critère, descriptions d'options) | 13 |
| GLiNER | la forme de sa carte (un texte, une tâche au nom court, étiquettes courtes) | 16 |
| GLiNER | une phrase et deux étiquettes | 15 : il répond « même » 42 fois sur 42 |
| GLiNER | **son vrai métier** : ranger un texte parmi dix sujets nommés, plus « aucun » | **6 sur 6** |

**Nous ne les avions pas mal servis pour juger une paire : ils ne savent pas
juger une paire, dans aucune forme.** En revanche GLiNER fait sans faute ce
pour quoi il est fait, ranger un texte sous une étiquette, en 117 ms sur CPU.
Les exemples de sa carte sont tous de cette forme (un message, une liste
d'intentions). Six cas ne prouvent rien ; c'est une piste, puisque la forme
que Lucie propose ci-dessous est exactement celle-là.

## 5. La forme proposée par Lucie : quel sujet va le mieux à ce texte ?

Ne plus comparer deux noms : donner le texte à ranger, et en options les
sujets existants trouvés par la recherche, plus le sujet neuf proposé.

**Le jeu.** Dix sujets, chacun avec un nom, une description et trois textes
déjà rangés ; 24 textes à ranger, dont 16 appartiennent à un sujet existant
et 8 demandent un sujet neuf ; moitié français, moitié anglais. Les textes
sont des règles de travail réelles de ce projet (l'index des mémoires de
travail des sessions, le journal des chantiers), une phrase chacune. Sujets,
rangement et noms de sujets neufs proposés sont posés par moi, non relus.
Les sujets proposés au modèle sont les quatre premiers d'une recherche par
cosinus ; quand le bon n'y est pas (1 fois sur 16), il est ajouté, pour
mesurer le verdict et non la recherche.

| | textes à ranger (16) : rejoint le bon | crée à tort | range ailleurs | textes neufs (8) : crée | fusionne à tort | jetons |
|---|---|---|---|---|---|---|
| **la forme d'origine** : deux noms, même / différent | 2 | 14 | — | 8 | 0 | — |
| **formulation 1**, contexte : nom seul | 12 | 2 | 2 | 7 | 1 | 188 |
| nom et description | 13 | 2 | 1 | 7 | 1 | 247 |
| + un texte déjà rangé | 9 | 7 | 0 | 8 | 0 | 348 |
| + deux textes | 8 | 8 | 0 | 8 | 0 | 431 |
| + trois textes | 9 | 7 | 0 | 7 | 1 | 503 |
| deux textes, le neuf présenté avec le texte à ranger comme seul exemple | 4 | 12 | 0 | 8 | 0 | 447 |
| deux textes, le neuf avec une description d'une ligne | 5 | 11 | 0 | 8 | 0 | 440 |
| deux textes, « aucun de ces sujets » à la place du neuf nommé | 12 | 1 | 3 | 7 | 1 | 424 |
| deux textes, le neuf **en premier** | 13 | 2 | 1 | 7 | 1 | 431 |
| deux textes, le neuf décrit et en premier | 13 | 2 | 1 | 8 | 0 | 440 |
| deux sujets existants seulement | 5 | 11 | 0 | 8 | 0 | 274 |
| dix sujets existants, deux textes | 9 | 7 | 0 | 7 | 1 | 901 |
| dix sujets existants, nom seul | 10 | 5 | 1 | 7 | 1 | 294 |
| **formulation 2**, un choix, « créer » en dernier | **15** | 1 | 0 | **8** | **0** | — |
| un choix, « créer » en premier | 14 | 0 | 2 | 8 | 0 | — |
| deux temps (« un existant convient-il ? » puis « lequel ? »), oui en premier | 11 | 2 | 3 | 7 | 1 | — |
| deux temps, non en premier | 10 | 3 | 3 | 8 | 0 | — |

La formulation 2 pose l'alternative dans le critère : « A new topic «…» is
proposed for this text. Should the text go under that new topic, or can it
join one of the existing topics? », avec pour options « join the existing
topic … » (nom, description, deux textes) et « create the new topic «…» ».

Ce que le tableau dit :

1. **Comparer deux noms ne marche pas pour décider de rejoindre** : 2 sur 16.
   Le nom qu'un agent propose (« adresse des commits ») est plus étroit que
   le sujet existant (« identité git ») ; le modèle répond « différent », et
   il a raison sur les noms. C'est le même phénomène que les « variantes ».
2. **Donner le texte change tout** : de 2 à 12 ou 13 sur 16 avec le seul nom
   des sujets.
3. **Le contexte peut nuire.** Ajouter des textes déjà rangés aux sujets
   existants fait *baisser* le résultat (13, puis 9, 8, 9) : le modèle crée
   à tort. Lecture probable, non prouvée : plus un sujet existant est
   décrit par des textes précis, plus un texte nouveau lui paraît différent
   de ces textes-là.
4. **Le sujet neuf est favorisé dès qu'on le décrit.** Sans texte : 8 créés à
   tort. Avec le texte à ranger comme exemple : 12. Avec une description :
   11. La probabilité moyenne donnée au neuf, pour un texte qui a déjà son
   sujet, passe de 0,15 (« aucun de ces sujets ») à 0,66 (le neuf illustré
   par le texte lui-même).
5. **La place du neuf compte** : les mêmes options avec le neuf en premier
   donnent 13 au lieu de 8. Le recul devant la première option, vu au §2,
   sert ici de contrepoids.
6. **Trois options, c'est trop peu ; dix, c'est trop long.** Avec deux
   sujets existants, 5 sur 16 ; avec dix et leurs textes, 9 sur 16 pour 901
   jetons. Quatre existants est le meilleur de ce qui a été essayé.
7. **Poser l'alternative dans le critère est la meilleure forme essayée** :
   15 et 14 sur 16, tous les neufs créés, aucune fusion à tort, dans les deux
   ordres.
8. **Deux temps font moins bien qu'un seul choix** (10 et 11 sur 16) et
   coûtent deux appels.

**Quand deux sujets conviennent presque autant.** Dans la formulation 2 à un
choix, les trois fautes des deux ordres sont des hésitations qui se lisent
dans les probabilités :

| texte | attendu | répondu | probabilité du choix, du second |
|---|---|---|---|
| « Put build directories on disk under the cache folder… » | compilation et charge du poste | créer | 0,60 / 0,35 |
| « Do not load the graphics card of this machine while the user is working on it. » | régime GPU | compilation et charge du poste | 0,52 / 0,42 |
| « One heavy build at a time; tell the orchestrator before starting it. » | arbre partagé entre sessions | compilation et charge du poste | 0,49 / 0,31 |

Les deux derniers textes parlent vraiment des deux sujets à la fois. Les
choix justes ont une probabilité médiane de 0,82 à 0,84 ; les trois faux
sont entre 0,49 et 0,60. Sur ce petit jeu, la zone incertaine est bien là où
on la voudrait.

**La même forme pour les mémoires** (la nouvelle mémoire en question, trois
mémoires existantes en options, plus « aucune : c'est nouveau ») : la bonne
mémoire est retrouvée 5 fois sur 5 pour une redite, 5 sur 5 pour un
complément, 5 sur 5 pour une mémoire neuve, et **3 sur 5 pour une
contradiction**. Cette forme dit *de quelle mémoire on parle*, pas *ce qu'on
en dit* ; le verdict à quatre choix du document 02 reste nécessaire ensuite.

## 6. Les fautes, une par une

Paires de sujets, configuration de base, hors « variantes » (traitées au §3) :

| paire | attendu | répondu | hypothèse |
|---|---|---|---|
| poids des modèles publiés sur Hugging Face / mise en ligne des fichiers de poids sur le hub | même | sans rapport, 0,49 (P(même) 0,21) | « le hub » ne dit pas lequel ; le lien demande de savoir que c'est le même service |
| régime doux de la carte graphique pendant les embarquements / limiter la charge du GPU quand on calcule les vecteurs | même | sans rapport, 0,38 (P(même) 0,36) | hésitation à trois ; « embarquements » pour « calcul de vecteurs » est notre vocabulaire, pas le sien |

Les deux fautes sur « même » sont des reformulations françaises longues, dans
notre jargon. Les douze autres fautes de la base sont des « variantes ».

## 7. Ce qui a aussi été mesuré

| | résultat |
|---|---|
| **latence sur carte** (4B en Q8, Radeon AI PRO R9700, par tunnel) | 230 ms par décision à trois options (170 jetons) ; 330 ms à 430 jetons ; 525 ms à 900 jetons. Sur CPU : 1 955 ms |
| **exactitude sur carte**, même jeu que le document 02 | 28 sur 42 et 17 sur 20 (Q4 sur CPU : 27 et 17) |
| **contrôle indépendant** proposé par la session de la mémoire longue : les six paires que son banc scripté écrit lui-même, jouées avec le seuil 0,07 du document 02, sans rien régler | la redite (« le dossier temporaire vit en RAM » pour « /tmp est de la mémoire vive ») : P(même) 0,31, au-dessus du seuil ; les cinq autres : 0,02 à 0,06, en dessous. **Le seuil tient**, sur six paires |
| **JevK5 en taille 2B** (Q8, CPU) | sujets 24 sur 42, mémoires 11 sur 20 (4B : 27 et 17) ; AUC 0,94 et 0,84 ; 1 503 ms par décision : à peine plus rapide, nettement moins bon sur les mémoires |
| **deux étages** (le cosinus classe 42 sujets, JevK5 juge les trois premiers plus « aucun », 20 requêtes) | le bon est dans les trois 11 fois sur 14 ; verdict juste 9 sur 14 ; **4 fusions à tort** ; « aucun » rendu 6 fois sur 6. Avec le reranker au premier étage : 12 sur 14, puis 9 sur 14, et une seconde de plus |
| **critère en français**, paires françaises, 4B sur CPU | sujets 6 sur 14 (8 avec le critère anglais) ; mémoires 6 sur 8 (7) |

Les quatre fusions à tort du montage à deux étages viennent de la forme
« noms seuls » : le modèle y choisit un sujet voisin faute de mieux. C'est la
forme que le §5 montre comme la plus faible.

## Ce que j'en retiens, sans conclure

- **Il existe au moins une manière de s'en servir qui tient sur ce petit
  jeu** : donner le texte à ranger, quatre sujets existants avec nom et
  description, et poser dans le critère l'alternative « créer le sujet
  proposé ou rejoindre un existant ».
- **La sensibilité à la forme est le résultat principal.** Entre deux formes
  voisines, le résultat va de 4 à 15 sur 16. Rien ne garantit que la
  meilleure d'ici le reste sur d'autres textes.
- **Comparer deux noms est la forme la plus pauvre**, et c'était celle de la
  première mesure. Les chiffres du document 02 sur les sujets décrivent
  cette forme-là, pas le modèle.
- **Mes étiquettes « variante » ne valent pas juge** : huit sur quinze sont
  contredites avec constance.

## Ce qui reste à essayer

- Un jeu de rangement plus grand, bâti sur de vrais sujets de la base et non
  écrit par la personne qui mesure ; c'est la réserve la plus lourde de ce
  document.
- La forme du §5 avec GLiNER, dont c'est le métier, et qui coûte dix fois
  moins.
- Pourquoi les textes déjà rangés nuisent : des extraits plus courts, ou
  choisis pour leur proximité avec le texte à ranger.
- Moyenner deux ordres d'options (le neuf en premier, puis en dernier) pour
  annuler le biais de position.
- Le critère en français sur la forme du §5.
