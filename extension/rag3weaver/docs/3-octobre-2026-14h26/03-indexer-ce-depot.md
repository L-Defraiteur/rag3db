# « Indexer ce dépôt » — proposition, sans code

3 octobre 2026 — session embarquements. Pour Lucie, portée par
l'orchestration avec la page de la session recherche sur les outils de
l'agent.

> **À lire d'abord (4 octobre 2026, midi) : en mémoire ou sur disque.**
> Tous les chiffres de cette page mesurés avant le 4 octobre à midi le sont
> sur une **base en mémoire**. Sur disque — le cas d'un utilisateur —, la
> première indexation de ce dépôt (mots seulement, 8 fils lucivy, binaire de
> test, NVMe) donne :
>
> | Paquets de | En mémoire | Sur disque | Pic de mémoire sur disque |
> |---|---|---|---|
> | 512 | 120 s | **823 s** | 13,1 Go |
> | tout (un paquet) | 89 s | **232 s** | 15,3 Go |
> | 512, tampon du moteur à 2 Gio | — | **échoue** vers 2 000 fichiers | — |
>
> Où part le temps sur disque, à 512 : pousser les blobs du plein texte en
> base 346 s en 56 appels (8 s en mémoire), symboles 153 s, `flush_fts`
> 137 s, `chunk_link` 87 s, file des liens vidée en route 57 s, chargement
> final des relations 38 s. D'un tenant : pousser les blobs 77 s en 4 appels.
> La base fait 3,5 à 3,7 Go. La cause n'est pas encore mesurée ; l'hypothèse
> est un point de reprise du moteur à chaque poussée (seuil du journal :
> 16 Mio). Avec un tampon de 2 Gio, un point de reprise échoue (« buffer pool
> is full ») et la base doit être rouverte : un poste modeste n'indexe pas ce
> dépôt aujourd'hui. Une passe par ligne, sans répétition ; le corpus a bougé
> de quelques dizaines de fichiers entre les passes.
>
> **Pourquoi le disque coûte sept fois plus — mesuré le 4 octobre, 12 h 40.**
> Paquets de 512, base sur disque, trois réglages du point de reprise du
> moteur :
>
> | Point de reprise automatique | Durée | Pousser les blobs | Replis du journal vus |
> |---|---|---|---|
> | actif, seuil 16 Mio (le défaut) | 823 s | 346 s | non compté |
> | coupé, un seul demandé à la fin | 523 s | 28 s | 84 |
> | actif, seuil 512 Mio | 468 s | 22 s | 83 |
>
> - **Une cause est établie** : au seuil par défaut, chaque poussée de blobs
>   dépasse 16 Mio de journal et fait poser un point de reprise. Relever le
>   seuil rend 300 à 350 s. Le point de reprise demandé à la fin coûte 0 s.
> - **Une seconde reste** : 83 replis du journal sans que rien ne les
>   demande. Des instructions posent leur propre point de reprise — les
>   chargements en masse (`COPY`), d'après le moteur lui-même. Ce qui reste
>   cher passe par eux : `flush_fts` 125 à 152 s, `chunk_link` 110 s,
>   symboles 95 à 154 s, file des liens vidée en route 44 à 69 s, chargement
>   final 32 à 34 s. Pas encore mesuré : une passe avec moins de `COPY`.
> - Le pic de mémoire ne bouge pas avec ces réglages (13,7 à 13,8 Go).
> - Le coût d'un point de reprise n'est pas expliqué : la session cœur C++
>   avait d'abord lu « tout le dernier groupe de lignes réécrit », puis s'est
>   corrigée (seulement le dernier segment des colonnes touchées, et les
>   métadonnées). Les 6 s par poussée restent sans cause.
> - **Les 83 points de reprise restants sont les `COPY FROM`**, confirmé par
>   le cœur C++ : chacun en force un à sa validation.
>
> **Où est la mémoire — 4 octobre, 14 h**, poste seul (`poste mesure`),
> moteur rebâti à 12 h 43, paquets de 512 sur disque, seuil 512 Mio :
>
> | Réglage | Durée | Anonyme à la fin des mots | Partagé | Pic, et pendant quoi |
> |---|---|---|---|---|
> | témoin | 419 s | 8,6 Go | 0,2 Go | 13,2 Go, au chargement final des relations |
> | tampon du moteur 4 Gio | **échoue** vers 4 500 fichiers | 7,0 Go à 4 096 fichiers | 0,2 Go | — |
> | fusions lucivy bornées | 377 s | 9,4 Go | 0,4 Go | 14,5 Go, au dernier paquet |
> | les deux | **échoue** vers 4 500 fichiers | 8,0 Go à 4 096 fichiers | 0,3 Go | — |
>
> - **La mémoire est anonyme** : le tas, et le tampon du moteur (un mmap
>   anonyme). Les fichiers d'index lucivy du tmpfs ne pèsent pas (0,2 Go).
> - **Au moins 3 Go sont dans le tas** : avec le tampon borné à 4 Gio,
>   l'anonyme atteint 7 Go avant l'échec. La répartition exacte entre tas et
>   tampon reste à faire.
> - **4 Gio de tampon ne suffisent pas** à indexer ce dépôt (2 Gio non plus,
>   ticket). Le prochain essai : 8 Gio, la borne que Lucie accepte.
> - **Borner les fusions lucivy ne réduit pas la mémoire** (elle monte) mais
>   accélère la passe de 42 s — une seule passe, à confirmer.
> - **Le pic arrive au chargement final des relations** (+4,4 Go en
>   quelques secondes) dans le témoin, et pendant le dernier paquet avec les
>   fusions bornées.
> - Les passes de 13 h 05 à 13 h 40 sont jetées : d'autres sessions
>   compilaient et testaient pendant elles.

Deux produits, un moteur : un agent de code en cloud qui télécharge un dépôt
git, un agent en ligne de commande qui ingère ce qu'il y a sur le disque. Le
verbe qui leur manque à tous deux est le même : **dire ce que ça va coûter,
puis indexer en fond**. Rien de ce qui suit n'est propre au code : la source
est une liste de fichiers, qu'ils soient du C++ ou des comptes rendus.

## 1. Ce qu'on propose : deux verbes et un reçu

Les noms sont ceux de la surface d'outils de la session recherche.

| Verbe | Ce qu'il fait | Ce qu'il rend |
|---|---|---|
| `estimate(source)` | Lit la liste des fichiers et leurs tailles, n'écrit rien. | Fichiers retenus et écartés (avec la raison), caractères, modèle choisi et pourquoi, durées prévues par niveau, et si une confirmation sera demandée. |
| `index(source)` | Ouvre la synchronisation, écrit, rend la main. | Un **reçu**. Le travail avance en fond. |
| `wait(reçu)` (existe déjà) | Attend, ou lit l'état sans attendre. | « plein texte prêt, vecteurs à 40 % », puis le rapport de fin. |

Pas de troisième outil pour l'avancement : l'indexation est une tâche de fond
comme une autre, son reçu se suit par le `wait` que les agents ont déjà.

## 2. L'estimation

**Ce qu'on compte**, sans rien lire d'autre que les noms et les tailles : la
politique d'ingestion existe (`code::verdict` : liste d'extensions interdites,
seuil de 128 Kio pour le texte brut) et dit pour chaque fichier « retenu » ou
« écarté, parce que ». Sur ce dépôt-ci, c'est ce qui sépare 59,8 Mo de code et de texte retenus
(6 831 fichiers) de 524 Mo écartés, surtout des CSV et du Parquet.

**Le dossier n'est pas le dépôt.** Lu tel quel, ce dossier de travail compte
307 541 fichiers (builds, données d'expérience, dossiers de compilation) ;
l'estimation l'a montré avant qu'on indexe. Depuis, `WorkingTree` respecte
les règles d'exclusion du dossier (7 442 fichiers) et écarte les secrets
probables, avec la raison.

**Le modèle** : l'heuristique du premier index, telle qu'elle est sur master
— granite-278m, sauf plus de 50 000 fichiers, carte faible ou absente, ou
seule carte du poste portant l'affichage, où l'on part en granite-107m. Elle
rend sa raison en une phrase ; l'estimation la montre telle quelle. Quand un
service d'embarquement est attaché (`RAG3WEAVER_EMBED_SERVICE`), les deux
déclencheurs qui parlent de la carte d'ici ne s'appliquent pas : ce n'est pas
elle qui calcule. **C'est un changement à faire dans l'heuristique**, elle ne
le sait pas aujourd'hui.

**La durée** : pas de table de débits par modèle ni par carte — elle serait
fausse au premier poste différent. Une **sonde** : une soixantaine de
morceaux pris dans la source, embarqués par l'embarqueur réellement attaché
(la carte d'ici sous son régulateur, ou le service), une à deux secondes. Le
débit mesuré est gardé en base avec le modèle et l'origine ; les estimations
suivantes le relisent, et chaque indexation le corrige.

**Le seuil de confirmation** est déclaré dans la configuration, en durée
prévue (proposé : cinq minutes de vecteurs) — pas en nombre de fichiers, qui
ne dit rien d'un poste à l'autre. Au-delà, `index` sans `confirm: true` est
refusé **avant d'avoir rien écrit**, par le harnais de validation qui existe
(`ToolHarness`, règle `before`) : l'appelant reçoit `accepted: false`,
`executed: false`, et l'estimation dans l'erreur.

> Une précision sur ce qui existe : le harnais sait **refuser par
> validation**, il n'a pas de notion de confirmation. La confirmation est une
> règle de plus à écrire dans ce harnais, pas un mécanisme à inventer.

## 3. L'indexation en fond, plein texte d'abord

`index` s'appuie sur la brique que la session de l'arbre principal prépare,
`sync_source(catalog, source, options)` : elle ouvre elle-même la session de
synchronisation, traite les fichiers par paquets, et finit en deux temps (le
plan de ce qui sera retiré, puis son application). Elle n'existe pas encore ;
sa forme est fixée et cette proposition se branche dessus en trois points.

1. **Ce qu'on exige de chaque paquet** : `RECHERCHE_TEXTE` — la donnée et le
   plein texte tout de suite, les vecteurs laissés en dette. Les quatre
   niveaux existent (`Disponibilites` : donnée, plein texte, sparse, dense),
   l'écriture « jusqu'à un niveau » aussi (`ingest_entities_jusqu_a`) ; il
   manque seulement que l'ingestion de code la prenne en option.
2. **La dette se rembourse seule**, et entre processus : `embarquer_le_retard`
   réclame des lots de morceaux sans vecteur (`_embed_claim`, périmé après
   une minute), donc deux processus ne paient pas deux fois, et un processus
   tué reprend où il en était — la dette est retrouvée en base à l'ouverture.
3. **Le reçu** est l'identifiant de la session de synchronisation, déjà
   persisté en base : il survit à l'arrêt du processus.

### Ce qui manque vraiment

- **Un état d'avancement lisible.** Le compte existe mais il est privé
  (`count_marqueur_manquant`) et ne sort que sous forme d'avertissement. À
  rendre public, par table et par niveau : *plein texte : prêt · vecteurs
  denses : 8 200 sur 20 100 · reste environ 3 min*. Les événements
  d'embarquement sont déclarés (`events.rs`) et jamais émis : on ne propose
  pas de les brancher, un compte en base suffit et vaut pour tous les
  processus.
- **Un ordre pour les vecteurs.** Aujourd'hui les tables sont parcourues dans
  l'ordre d'une table de hachage, 512 morceaux par passe. Proposé, du plus
  simple : d'abord ce qu'une recherche vient de demander (ça existe : une
  lecture qui exige le dense rembourse sa cible), puis les fichiers les plus
  récemment modifiés. Rien de plus fin tant qu'on n'a pas vu le besoin.

### En combien de temps est-on cherchable par mots ?

**Mesuré le 3 octobre au soir**, deux fois : une fois l'ingestion de code
capable de s'arrêter au plein texte (`sync_source` avec
`exige = RECHERCHE_TEXTE`), puis après le chargement en masse des relations
— les deux livrés par la session de l'arbre principal. Le premier jet de
cette page donnait un calcul, « environ une minute » : la première mesure l'a
trouvé **faux d'un facteur trente** sur le dépôt entier, la seconde le ramène
à un facteur neuf. Les trois chiffres restent ici, datés.

| Corpus (binaire de test non optimisé, granite-278m par le service distant) | Cherchable par mots | Relations | Vecteurs |
|---|---|---|---|
| `src/` de la crate : 122 fichiers, 4 Mo, 9 528 morceaux | **17 s** | comprises | ~1 min prévue |
| Ce dépôt, première mesure : 6 735 fichiers, 59,6 Mo, 418 761 relations posées paquet par paquet | 1 798 s (30 min) | comprises | 710 s mesurés, 569 s prévus |
| **Ce dépôt, après le chargement en masse des relations** : 6 799 fichiers, 60,6 Mo, 123 750 morceaux, 384 790 relations | **514 s** (8 min 30) | **10 s** | **692 s** mesurés, 651 s prévus |

Quatre choses à en retenir.

- **L'exigence est tenue** : à la fin de la passe « plein texte », aucun
  vecteur n'est calculé et une recherche par mots répond. La toute première
  mesure, avant correctif, en trouvait 83 % de faits — un rattrapage tournait
  dans la liaison des symboles.
- **Le temps des mots ne croissait pas comme la taille, et c'est corrigé.**
  À la première mesure, les 1 024 premiers fichiers passaient en 19 s puis
  chaque millier coûtait 53, 151, 283, 422 s : l'insertion des relations
  paquet par paquet. La session de l'arbre principal les met maintenant en
  file pendant les paquets et les charge en masse à la fin ; par millier de
  fichiers, cumulé : 19, 66, 158, 244, 323, 400, 514 s. Ramené aux scopes
  c'est à peu près plat — 5 ms par scope au premier millier, 7 en moyenne.
  Le temps total pour chercher par mots passe de 1 798 s à 523 s.
- **Les relations arrivent d'un coup, en dix secondes**, après les mots.
  Entre-temps la file est vidée chaque fois qu'elle dépasse 200 000 liens
  (deux fois sur ce dépôt), pour borner la mémoire : ces vidages sont comptés
  dans le temps des mots.
- **L'estimation des vecteurs tient à 6 % près** : 651 s prévus, 692 s
  mesurés (20 % à la première mesure, où la sonde ne voyait pas la
  reconstruction de l'index). Chaque indexation note son débit réel ; la
  suivante prévoit mieux.

**Où en est la promesse.** Sur ce dépôt, on cherche par mots au bout de huit
minutes et demie, le graphe est là dix secondes plus tard, et les vecteurs
onze à douze minutes après : le plein texte d'abord fait gagner plus de la
moitié de l'attente. Ce n'est pas encore les deux minutes qu'un moteur
optimisé rendrait — ces mesures sont prises sur un binaire de test non
optimisé — et l'avancement dit donc des fichiers faits, pas un temps restant,
pendant ce premier temps.

### Où part le temps des mots, et ce que change un binaire optimisé

Une passe de plus, le 3 octobre à 22 h : le même dépôt, mots seulement, en
binaire **optimisé** (`--release`), avec le profil d'ingestion. **643 s** —
pas mieux que les 523 s du binaire de test. L'optimisation ne change rien
parce que les dépendances (moteur, lucivy, tokenizers) sont déjà optimisées
dans le binaire de test ; l'écart entre les deux passes est la charge du
poste (douze à treize processus actifs pendant la mesure), pas le binaire.
**La promesse des deux à trois minutes n'est donc pas tenue** sur un dépôt de
cette taille : on est à huit à dix minutes.

| Poste | Temps | Remarque |
|---|---|---|
| Vidages de la file des relations | 214 s | Sept vidages d'environ 200 000 liens. Cinq prennent **2 s** chacun ; deux prennent **57 s et 145 s** pour la même taille. Sans ces deux-là : ~12 s. |
| Ingestion des symboles | 108 s | 107 paquets, une seconde chacun. |
| Insertion des nœuds et des morceaux | 94 s | |
| Index plein texte (lucivy, 8 fils) | 26 s | La borne à 8 fils ne coûte pas : ce poste n'est pas le goulot. |
| Marques de découpe | 18 s | |
| Points de reprise | 3 s | |
| Non ventilé | ~180 s | L'analyse des fichiers (codeparsers), la lecture, les marques de session : aucun de ces temps n'est publié aujourd'hui. |

Où chercher, dans l'ordre : les **deux vidages aberrants** (200 s à eux
deux, un tiers de la passe — même nombre de liens que les cinq autres, qui
prennent deux secondes), puis **ce qui n'est pas ventilé** (il faut d'abord
le mesurer), puis les symboles. Trois COPY de morceaux sont aussi refusés à
la première ingestion (« vector with ANY type ») et retombent sur le chemin
ligne à ligne.

### Après la fermeture des deux vidages aberrants : 499 s, il en manque 320

Troisième passe, le 3 octobre à 22 h 40, même protocole que celle de 523 s
(binaire de test, 8 fils lucivy, service distant, mots seulement) : la
session de l'arbre principal a trouvé la cause des deux vidages — le COPY
des liens était refusé dès qu'une liste entre guillemets apparaissait après
les premières lignes, et les liens repartaient par lots en balayant les
tables.

| Passe | Cherchable par mots (relations comprises) |
|---|---|
| Relations paquet par paquet | 1 798 s |
| Chargement en masse à la fin | 523 s |
| Binaire optimisé, avec les deux vidages à 57 s et 145 s | 643 s |
| **Vidages fermés** (sept vidages, 30 s en tout) | **499 s** |

La cible est deux à trois minutes : **il manque environ 320 s**. Cette fois
le temps est ventilé en entier.

| Poste | Temps |
|---|---|
| Ingestion des symboles | 109 s |
| Dans l'ingestion des paquets, hors de tout nœud (montage du graphe, points de reprise, construction des enregistrements) | 140 s |
| Insertion des morceaux | 56 s |
| Insertion des nœuds | 49 s |
| Analyse des fichiers (codeparsers) | 43 s |
| Index plein texte (lucivy, 8 fils) | 26 s |
| Vidages de la file des liens en route | 23 s |
| Marques de découpe | 20 s |
| Chargement final des relations | 14 s |
| Liaison des morceaux, mise en file des symboles, marques de session | 18 s |
| Lister et lire les fichiers | moins d'une seconde |

Où chercher, par ordre de prise : **les symboles** (une seconde par paquet ;
la réécriture des requêtes par lot, en cours, devrait y toucher), **les
140 s hors nœuds** (à instrumenter avant d'y toucher), **les insertions**
(105 s, dont trois COPY de morceaux refusés qui retombent en ligne à ligne),
puis **l'analyse**, qui se parallélise par fichier. Le plein texte et les
relations ne sont plus le sujet.

### Ventilé en entier : 352 s, et le plus gros poste n'était dans aucun profil

Le 3 octobre à 23 h 20, après deux changements : la réécriture des requêtes
par lot (session de l'arbre principal : toutes les formes `UNWIND … MATCH`
passent par jointure au lieu de balayer les tables) et des chronomètres posés
sur **tout** le chemin d'ingestion, cumulés par étape (`[ingest-total]`).

| Passe | Cherchable par mots (relations comprises) |
|---|---|
| Relations paquet par paquet | 1 798 s |
| Chargement en masse à la fin | 523 s |
| Vidages aberrants fermés | 499 s |
| **Requêtes par lot réécrites** | **352 s** |

La cible reste deux à trois minutes : **il manque environ 170 s**.

| Poste (passe de 352 s) | Temps | Avant la réécriture |
|---|---|---|
| **Pousser les blobs de l'index plein texte en base** | **119 s** | 194 s |
| Analyse des fichiers (dont 46 s dans l'analyseur, déjà parallèle) | 55 s | 51 s |
| Ingestion des symboles | 56 s | 114 s |
| Nœud plein texte (lucivy, 8 fils) | 29 s | 29 s |
| Points de reprise du graphe | ~30 s | ~24 s |
| Vidages de la file des liens en route | 27 s | 30 s |
| Insertion des nœuds et des morceaux | 29 s | 108 s |
| Chargement final des relations | 12 s | 10 s |
| Relire ce qui est en base (inchangés) | 11 s | 42 s |
| Marques de session, liaison des morceaux | 20 s | 22 s |

Ce que la ventilation a corrigé dans le diagnostic précédent :

- **Les 140 s « hors de tout nœud » étaient la poussée des blobs d'index.**
  Après chaque appel d'ingestion — quatre par paquet, 404 sur ce dépôt — les
  fichiers de l'index plein texte sont écrits en base : c'est ce qui rend
  l'index relisible après un arrêt brutal. Une demi-seconde à chaque fois,
  et aucune ligne de profil ne la montrait.
- **Les points de reprise du graphe ne sont pas le sujet** : une trentaine de
  secondes, pas cent quarante.
- **L'analyse ne se parallélise pas davantage chez nous** : l'analyseur
  travaille déjà par fichier en parallèle, et notre part en aval est de neuf
  secondes.

La prochaine prise est donc la poussée des blobs : ne la faire, pendant une
synchronisation de source, qu'aux vidages de la file et à la fin. Elle ne se
diffère pas gratuitement — une reprise ne réindexe pas les lignes déjà en
base, donc un arrêt brutal laisserait un index en retard sans que rien le
dise. La session de l'arbre principal la prend, avec une marque durable
« plein texte en retard » et la preuve par un arrêt brutal réel. Hors
synchronisation, rien ne change.

### Trois minutes sont-elles atteignables sur ce dépôt ?

Ce que demanderaient les 170 s qui manquent, poste par poste, à partir des
352 s mesurées. Les gains sont des estimations lues sur la ventilation, pas
des mesures.

| Prise | Ce qu'il faut | Gain plausible | Reste après |
|---|---|---|---|
| Les blobs d'index (119 s) | Pousser aux vidages et à la fin, avec la marque « plein texte en retard » — en cours chez la session de l'arbre principal. | ~100 s | ~250 s |
| Les points de reprise du graphe (30 s) | Un mode plus léger pour une première indexation en masse : elle se reprend par sa session de synchronisation, pas par l'état de chaque nœud. | ~25 s | ~225 s |
| Les symboles (56 s) | Les ingérer une fois à la fin, comme les relations, au lieu de 107 fois. | 30 à 40 s | ~190 s |
| L'analyse (55 s, dont 46 dans l'analyseur) | Chez l'analyseur, qui est déjà parallèle par fichier : le gain dépend du nombre de fils qu'on lui laisse, et le poste ne nous appartient pas. | incertain | — |
| Le nœud plein texte (29 s) et les vidages de la file (39 s) | Rien d'évident : c'est le travail lui-même. | — | — |

**La réponse honnête** : avec les trois premières prises, ce dépôt descend
vers **trois minutes à trois minutes et demie** ; les trois minutes rondes
demandent en plus un gain chez l'analyseur. Tant que ces prises ne sont pas
posées et mesurées, la promesse qu'on peut tenir est **« cherchable par mots
en quatre à cinq minutes »** pour un dépôt de 60 Mo — et **« en moins de
vingt secondes »** pour un projet de la taille de `src/` de la crate (4 Mo),
ce qui est le cas courant. Le chiffre est celui d'un binaire de test sur un
poste chargé : il se remesure à chaque prise.

### La cible devient 90 secondes : ce que donne un seul paquet

Lucie, le 3 octobre au soir : « ça commence à être acceptable mais pas encore
assez ; on devrait viser 1 min 30 max ». Les trois minutes de la section
précédente ne sont plus la cible.

Mesuré à 23 h 40, **sans rien changer au code** que la taille des paquets
(`RAG3WEAVER_ESTIMATE_BATCH_FILES=100000` : toute la source en un paquet au
lieu de 107) : **132 s** pour être cherchable par mots, relations comprises —
contre 352 s. Le prix : **16,1 Go de mémoire résidente au pic**. La passe a
tourné en même temps qu'une mesure d'une autre session : le chiffre est
plutôt pessimiste.

| Poste (un seul paquet, 132 s) | Temps |
|---|---|
| Analyse : l'analyseur | 22,5 s |
| Analyse : notre aval (texte propre, clés, rendez-vous), sur un seul fil | 21 s |
| Vidages de la file des liens en route (quatre, au seuil de 200 000) | 33 s |
| Graphe des entités : insertion 8 s, plein texte 7 s, morceaux 4 s, liaison 3 s, points de reprise ~4 s | 26 s |
| Symboles | 10 s |
| Chargement final des relations | 8 s |
| Mise en file des relations, marques de session | 8 s |
| Blobs de l'index plein texte (quatre poussées au lieu de 404) | 2 s |

Ce que la mesure apprend :

- **Le coût était dans le nombre de paquets, pas dans le travail.** Presque
  tout ce qu'on comptait optimiser — les blobs, les points de reprise, les
  symboles par paquet — disparaît de lui-même quand il n'y a qu'un paquet.
- **Il manque une quarantaine de secondes pour 90 s**, et deux prises les
  portent : notre aval de l'analyse (21 s séquentiels, parallélisables par
  fichier) et les liens (41 s).
- **La mémoire décide de la forme.** Seize gigaoctets ne passent pas sur un
  poste modeste : la forme tenable est un paquet borné par sa taille en
  octets, pas « tout ». Les points intermédiaires de la courbe restent à
  mesurer.
- **Les deux chemins ne construisent pas le même graphe** : 819 808
  relations en un paquet, 438 104 par paquets de 64. L'analyseur relie entre
  fichiers tout ce qu'on lui donne ensemble. Lequel est juste se tranche
  avant de choisir la forme (session de l'arbre principal).

### Deux nouvelles du 4 octobre qui déplacent ce budget

Relayées par l'orchestration, mesurées par la session codeparsers ; pas
encore remesurées ici.

- **L'analyse du dépôt entier passe de 34,7 s à 12,8 s** par paquets de 64,
  et à **5,9 s en un appel « fichier seul »** : un quadratique retiré de
  l'extraction des scopes (branche `fichier-seul`, en cours d'intégration
  dans l'arbre principal). Des 22,5 s d'analyseur du paquet unique, il en
  resterait donc une demi-douzaine.
- **L'analyseur ne résout plus que dans le fichier** ; les liens entre
  fichiers passent par les rendez-vous. Le graphe ne dépend plus de la
  taille du paquet : l'écart 819 808 contre 438 104 relations disparaît par
  construction, et de gros paquets bornés en octets deviennent sûrs.

La remesure du dépôt entier se fera quand le résolveur unique sera sur
master, en une passe annoncée.

### La remesure du 4 octobre : 89 s d'un tenant, 111 s par paquets de 2 048

Sur master après le résolveur unique (`63d154730`) et les fichiers générés
écartés (95 fichiers). Même protocole : les mots seulement, 8 fils lucivy,
binaire de test, base en mémoire, moteur de master (bâti le 4 à 0 h 59),
poste calme (charge 0,4), quatre passes dos à dos.

| Paquets de | Durée | dont mots | dont relations | Pic de mémoire | Relations |
|---|---|---|---|---|---|
| 64 | **289 s** (352 s la veille) | 278 s | 11 s | 16,6 Go | 305 357 |
| 512 | **120 s** | 109 s | 10 s | 11,3 Go | 304 883 |
| 2 048 | **111 s** | 102 s | 9 s | 10,9 Go | 304 922 |
| tout (un paquet) | **89 s** (132 s la veille) | 81 s | 8 s | 14,8 Go | 304 914 |

6 822 fichiers et 74 404 scopes aux deux premières passes, 6 824 et 74 417
aux deux dernières : d'autres sessions écrivaient du code, le corpus a bougé
de deux fichiers.

Où part le temps, par poste (secondes) :

| Poste | 64 | 512 | 2 048 | un paquet |
|---|---|---|---|---|
| analyser (codeparsers) | 24 | 17 | 21 | 24 |
| ingérer les paquets | 230 | 73 | 60 | 38 |
| — dont pousser les blobs d'index | 109 | 8 | 2,5 | 1,2 |
| — dont exécuter le graphe d'ingestion | 91 | 46 | 40 | 24 |
| — dont symboles | 52 | 16 | 13 | 9 |
| vider la file des liens en route | 21 | 18 | 20 | 18 |
| charger les relations à la fin | 11 | 10 | 9 | 8 |

Ce que ça dit :

- **La cible de 90 s est touchée d'un tenant (89 s), sans marge**, et au prix
  de 14,8 Go. Par paquets de 2 048 il manque 21 s, avec 10,9 Go.
- **Les paquets de 64 perdent 109 s à pousser les blobs d'index** en 411
  appels : c'est presque tout l'écart avec 512. Le report de cette poussée
  rendrait les petits paquets proches des gros.
- **Un plancher d'une cinquantaine de secondes ne dépend pas de la taille** :
  analyser (17 à 24 s), vider la file des liens en route (18 à 21 s), charger
  les relations (8 à 11 s). C'est là qu'est la marge sous 90 s : la file des
  liens pourrait ne se vider qu'une fois, à la fin.
- **Le graphe ne dépend presque plus du paquet** : 304 883 à 305 357
  relations, contre 438 104 et 819 808 la veille. Il reste 474 relations de
  plus à 64 qu'à 512, à corpus identique — pas expliqué.
- **Le pic de mémoire est le plus haut à 64 (16,6 Go)**, pas d'un tenant —
  inattendu, pas expliqué. Aucune taille ne passe sur un poste modeste : la
  mémoire reste à traiter avant de choisir la forme.

Non mesuré ici : ce que les 95 fichiers générés retirent à eux seuls (il
aurait fallu une passe règle levée), un binaire optimisé, une base sur disque.

## 4. Les deux politiques

Le mécanisme ne connaît que `FileSource` (lister, lire). La politique décide
quelles sources on a le droit de lui passer.

| | Politique « poste » (ligne de commande) | Politique « cloud » (agent en ligne) |
|---|---|---|
| Dossier local | Lu en place (`WorkingTree`), rien n'est copié. | **Interdit** : pas de chemin du disque. |
| Adresse git | Clonée dans un espace de travail géré. | Clonée dans l'espace de travail de la session — la seule source permise. |
| Révision | Celle qu'on demande, sinon la branche par défaut ; écrite dans le reçu. | Idem, et **obligatoirement résolue en un commit** avant d'indexer. |
| Rafraîchir | Relancer `index` : seuls les fichiers changés sont repris. | `fetch` puis `index` sur la nouvelle révision. |
| En plus | — | Schémas d'adresse permis (https), taille maximale du clone, pas de sous-modules ni de crochets git. |

La source git n'existe pas (`FileSource` porte le commentaire « demain
`git:<sha>` ») : c'est un clone suivi d'un `WorkingTree` sur le dossier
cloné. Les champs `repo` et `revision` sont déjà dans le schéma des fichiers.

### Les fichiers générés, écartés avec leur raison (4 octobre)

Un analyseur ANTLR, un `*_onnx.rs` sorti d'un convertisseur n'apprennent rien
à un agent et gonflent le graphe. Ils sortent de l'indexation et de son
estimation comme les secrets probables : par une règle nommée, comptée, que
le manifeste règle ou lève (`src/generated.rs`).

```json
"workspace": { "source": "working_tree", "root": ".", "index": "code",
               "generated": { "skip": true, "dirs": ["generated"] } }
```

- **Deux signaux** : un dossier déclaré (`dirs`, défaut `generated`) ; un
  marqueur dans les 2 premiers Ko (`markers`, casse comprise : `@generated`,
  `DO NOT EDIT`, `auto-generated`… ; `line_starts` : une ligne de
  commentaire qui commence par `Generated by`, `Generated from`…).
- **`"skip": false` lève la règle.** Un dossier de documents déclare les
  siens (`"dirs": ["exports", "journaux"]`).
- `estimate` les compte avec leur poids (« 81 écartés (3,7 Mo) : généré —
  marqueur en tête de fichier ») ; `index` ne les analyse pas et le rapport
  de synchronisation les compte (`filesSetAside`) ; une réindexation retire
  ce qu'ils avaient laissé. Lire, lister, chercher par motif dans un fichier
  généré reste permis.

Mesuré sur ce dépôt (7 006 fichiers retenus, 62,1 Mo) : **95 fichiers,
5,4 Mo** — 14 dans `generated/` (1,7 Mo), 81 par marqueur (3,7 Mo, dont
`roaring.c` 969 Ko et `cypher_parser.cpp` 589 Ko). Lire les têtes coûte
1,5 s à l'estimation (binaire de test). Faux positifs connus : un
`cbindgen.toml` de quelques lignes, dont le texte parle d'« autogenerated ».
Une règle sans casse écartait `pybind11.h` (128 Ko) pour un commentaire
« Do not modify… » : d'où la casse.

Pas fait : ce que retirent ces fichiers en relations et en temps (à lire à
la prochaine passe du dépôt entier) ; `.gitattributes linguist-generated` ;
le troisième signal proposé, une grosse taille pour très peu de scopes
(`utf8proc_data.h`, 1,9 Mo) — il demande l'analyse, et la taille seule
écarterait `catalog.rs`.

## 5. Ce qu'on recommande de coder en premier

> **État au 3 octobre au soir** : les points 1 et 2 sont sur master
> (`src/estimate.rs`, `EstimateNode`, `templates/tools/estimate.mmd`,
> `Catalog::index_progress`). Le point 3 attend la correction de l'insertion
> des relations.

1. **`estimate`**, entièrement : compte par `verdict`, modèle et raison,
   sonde de débit, seuil. C'est une lecture, elle ne dépend de personne, et
   elle se teste avec un embarqueur factice. Elle sert dès maintenant, même
   devant l'ingestion d'aujourd'hui.
2. **L'état d'avancement public** par table et par niveau. Petit, et il rend
   visible ce qui existe déjà (la dette, son remboursement).
3. **`index`**, quand `sync_source` est livrée avec son option d'exigence :
   le reçu, la confirmation dans le harnais, le remboursement en fond.
4. **La source git et la politique « cloud »**, en dernier : c'est ce qui
   demande le plus de décisions (où cloner, quoi interdire) et le moins de
   moteur.

## 6. Ce qui attend une décision de Lucie

- **Le seuil de confirmation** : cinq minutes de vecteurs prévues, ou autre.
- **L'ordre des vecteurs** : « ce qu'on vient de chercher, puis le plus
  récent » suffit-il pour commencer ?
- **La politique « cloud »** : faut-il autoriser autre chose que https public
  (jeton d'accès à un dépôt privé) dès la première version ?
