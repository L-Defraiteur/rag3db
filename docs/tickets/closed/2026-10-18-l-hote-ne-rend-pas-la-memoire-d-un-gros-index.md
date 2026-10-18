# L'hôte ne rend pas la mémoire d'un gros index : 10 Go restent dans l'échange, l'index fini

- **État** : **fermé** le 19 octobre 2026 (G, jugé par le banc), pour la phase plein texte et
  relations : l'hôte rend au système ce que glibc garde (`malloc_trim(0)`) en fin d'index, à la
  fermeture d'un backend et au repos ; aucun `MALLOC_ARENA_MAX` (il coûte trop de débit). La phase des
  vecteurs reste à mesurer :
  [la mémoire d'un index avec ses vecteurs](../2026-10-19-la-memoire-d-un-index-avec-ses-vecteurs-a-mesurer.md).
  Ouvert le 18 octobre 2026 (seconde session cœur C++, à la demande de l'orchestration).
- **Gravité** : mémoire. Rien n'est perdu ni faux, mais le poste reste chargé tant que l'hôte vit.
- **Atteignable en service** : oui, après tout gros index ou rattrapage dans un hôte qui reste
  ouvert ensuite.
- **Touche rag3weaver** : oui (l'hôte du démon, celui des sessions MCP).

## Ce que c'est

Une fois un gros index fini, l'hôte rag3weaver ne rend pas sa mémoire au système. Sur le poste
principal, l'hôte de Lucie tenait encore **10 Go dans l'échange** le 15 octobre, trois jours après
le rattrapage du 12. Il n'indexait rien depuis. L'orchestration l'a arrêté proprement le 15, à la
demande de Lucie. Avec l'effacement de 755 dossiers temporaires morts, voir
[le ticket des dossiers temporaires](2026-10-18-les-tests-ne-nettoient-pas-leurs-dossiers-temporaires.md),
l'échange du poste est passé de 22 à 10 Go.

Le rattrapage du 12 est celui où l'hôte a monté à 44-52 Go de mémoire résidente : l'index de
`code-principal` a été lancé dans un hôte déjà figé à 30 Go (le `break` d'un garde-fou ; mémoire
de session `garde-fou-arrete-tout-le-script`).

## Recette

Aucune recette minimale pour l'instant. Seul existe le relevé du 15 octobre (orchestration,
`extension/rag3weaver/docs/11-octobre-2026-10h51/orchestration/01-les-decisions-du-matin.md`, au
15). Ce relevé ne garde ni la mémoire résidente ni la composition de l'échange (`smaps`) de l'hôte
avant son arrêt.

## Le témoin à écrire

Une sonde, sans assertion d'abord : elle mesure.

1. Un hôte neuf ouvre un backend de code. On lit `VmRSS`, `VmSwap` et `RssAnon` dans
   `/proc/<pid>/status`, et `smaps_rollup`.
2. On indexe un dépôt de taille connue sous un tampon donné (`RAG3DB_BUFFER_POOL_SIZE`), puis on
   attend la fin et le repos (plus aucune tâche).
3. On relit les mêmes compteurs à 0, 1 et 10 minutes de repos. On ferme le backend sans arrêter
   l'hôte, et on relit encore.

Le défaut est établi si la mémoire anonyme au repos, backend fermé, reste nettement au-dessus de
l'état neuf plus le tampon. La sonde se joue sous `poste mesure`, sur le jeton de l'orchestration
(une grosse indexation à la fois par poste).

## Pistes, aucune vérifiée

- **Le tampon du moteur** : les cadres déjà touchés restent à l'hôte tant que la base reste
  ouverte. Ce comportement est attendu jusqu'à la taille du tampon. La question est ce qui reste
  au-delà, et après la fermeture de la base.
- **La mémoire tenue hors du tampon** : c'est déjà le
  [ticket de la mémoire résidente au-delà du tampon](2026-10-11-memoire-residente-au-dela-du-tampon-sous-des-blobs.md),
  environ 1 Go par millier de fichiers à la première indexation. Le présent ticket en serait la
  suite : cette mémoire ne redescend pas, une fois le travail fini.
- **L'allocateur** : glibc garde ses arènes (une par fil) et ne rend une page au système qu'en haut
  du tas, sauf `malloc_trim`. Un essai consiste à appeler `malloc_trim(0)` au repos dans la sonde
  et à voir ce qui tombe. C'est un essai, pas un correctif.
- **Ce que tient l'hôte lui-même** : l'embarqueur, les caches de lucivy, les files de l'index.

## Mesuré : d'où vient le pic d'un premier index (arbre principal, 18 octobre 2026)

Luciepc, master `c6bb094fb` (lib du magasin 0fc7dd10f), worktree `rag3db-tampon`, la recette du
ticket du 11 octobre (`RAG3WEAVER_ESTIMATE_REPO=1`, `e2e_estimate
ce_depot_est_cherchable_par_mots_avant_ses_vecteurs`, base sur disque), qui passe à côté de la
borne, sous `poste lourd`, services d'embarquement locaux, régime doux. 6 318 fichiers, 57 Mo de
texte compté par la passe, 687 467 liens. Journaux : `~/.cache/rag3weaver-notes-A/tampon/` sur
luciepc. Le pic est relevé par le test toutes les 100 ms ; il tombe chaque fois pendant la pose
des relations.

| passe | plein texte | tampon | résultat | durée | pic au fil de l'eau | anonyme à 1 024 / 3 072 / 5 120 / 6 318 fichiers |
|---|---|---|---|---|---|---|
| paquets de 512 | fichiers | 2 Gio | **vert** | 69 s | 7 907 Mo | 2 359 / 4 576 / 5 569 / 6 288 Mo |
| paquets de 512 | fichiers | 4 Gio | **vert** | 68 s | 7 677 Mo | 2 365 / 4 493 / 5 265 / 5 967 Mo |
| paquets de 512 | blobs | 2 Gio | **vert** | 124 s | 9 212 Mo | 2 635 / 5 253 / 6 801 / 7 580 Mo |
| paquets de 512 | blobs | 4 Gio | **vert** | 122 s | 9 805 Mo | 2 673 / 5 609 / 7 269 / 8 405 Mo |
| paquets de 2 048, un COMMIT par paquet (le défaut d'`index`) | fichiers | 4 Gio | **vert** | 58 s | 7 982 Mo | 3 769 à 2 048 ; 6 446 à 6 144 |
| paquets de 2 048, une seule transaction (`RAG3WEAVER_TX_PAQUETS_PAR_VALIDATION=tous`) | fichiers | 4 Gio | **vert** | 56 s | 8 242 Mo | 3 699 à 2 048 ; 6 599 à 6 144 |

- **Ni le tampon ni la transaction ne font le pic.** Sous 2 puis 4 Gio, il bouge de 0,2 Go
  (7 907 contre 7 677 Mo) ; en une seule transaction plutôt qu'un COMMIT par paquet, de 0,26 Go
  (8 242 contre 7 982 Mo).
- **C'est l'anonyme**, qui croît d'environ **1,1 Go par millier de fichiers** et ne redescend pas
  d'un paquet à l'autre, alors que chaque paquet est validé. Le plein texte en blobs en ajoute
  environ 1,3 Go au bout du compte.
- **Le pic tombe pendant la pose des 687 467 relations**, après les paquets (« Relations, 687467
  liens à poser »), environ 1 Go au-dessus de la mémoire de la fin des paquets.
- Le pic de 8,2 Go de l'hôte de luciepc (10 Gio de tampon) est donc l'ordre de ce que fait la passe
  elle-même, pas un tampon trop grand. Les pistes à départager restent celles d'en haut, l'anonyme
  d'abord : ce que retient chaque paquet après son COMMIT (files de l'index, état des relations
  différées, caches de lucivy, arènes de l'allocateur).

## Mesuré : ce que garde l'allocateur après la fermeture (banc, 18 octobre 2026)

Ce n'est pas l'hôte, mais le même moteur : le banc de concurrence sur luciepc, master `cbe99e380`
à `a4505f0b3`, une sonde gtest non commitée qui lit `/proc/self/maps` et `/proc/self/status` avant
et après chaque test. `TwentyWriters.DdlEveryFifteenSeconds` fait travailler 20 écrivains de
vecteurs, 2 de `Note` et 4 lecteurs sur 8 fils de travail, avec un tampon de 2 Gio. La base est
ensuite fermée et tous les fils partis, il ne reste que le principal.

| moment | projections | VmSize | VmData | VmRSS |
|---|---|---|---|---|
| avant | 64 | 32 Mio | 0 | 9 Mio |
| après, base fermée | 151 | 3 032 Mio | 2 065 Mio | **1 737 Mio** |
| après `malloc_trim(0)` | 151 | 3 032 Mio | 2 065 Mio | **205 Mio** |

Trois passes du même scénario dans un seul processus laissent VmData stable, à 2,2 Gio, et les
projections autour de 149. Il n'y a donc pas de fuite d'une base à l'autre. Ce qu'on voit :
- **La mémoire résidente retenue après la fermeture est de la mémoire libérée que l'allocateur de
  glibc garde.** `malloc_trim(0)` en rend 1,5 Go sur 1,7.
- **VmData et VmSize ne redescendent pas, même après `malloc_trim`** : ce sont les arènes de malloc
  des fils, une par fil jusqu'à 8 × le nombre de cœurs, que glibc ne rend jamais. Elles comptent
  dans `RLIMIT_DATA`. Un test du banc, `RolledBackCopyThenCheckpoint`, en a rougi. Son plafond
  absolu est devenu relatif.

C'est la piste « arènes de l'allocateur » de ce ticket, et elle explique au moins la part qui ne
redescend pas après un gros travail fini.

**Le correctif, confié à G (orchestration)** : `malloc_trim(0)` après un gros travail (un index, un
rattrapage) et à l'endormissement de l'hôte, et un `MALLOC_ARENA_MAX` dans l'unité systemd.

**Le protocole qui le jugera (banc)** :
1. Un gros index de l'hôte (la recette du ticket du 11 octobre), puis l'hôte au repos.
2. Relever `VmRSS`, `VmData` et le nombre de projections de l'hôte (`/proc/<pid>/status` et
   `/proc/<pid>/maps`) à la fin de l'index, puis 30 s plus tard, au repos.
3. Le même relevé avant et après le correctif, sur la même recette et le même poste.
4. Attendu : au repos, `VmRSS` revient près de son niveau d'avant l'index, à quelques centaines
   de Mo près, au lieu de garder les Go de l'index.

## Pour le fermer

1. Écrire la sonde, puis savoir quelle part revient à chaque piste.
2. Corriger la part qui ne redescend pas (rendre au système au repos ou à la fermeture d'un
   backend).
3. Passer la sonde en témoin : au repos, backend fermé, la mémoire anonyme revient sous un seuil
   écrit dans le test, avec sa justification.

## Corrigé : la mémoire rendue au système (G, jugé par le banc, 19 octobre 2026)

La piste « arènes de l'allocateur » était la bonne : la mémoire tenue après un gros index est de la
mémoire libérée que glibc garde.

**Le correctif** (`src/allocateur.rs`) : `rendre_au_systeme(raison)` appelle `malloc_trim(0)`, sous
Linux avec glibc seulement (ailleurs, rien), et dit `VmRSS` avant et après sur la sortie d'erreur.
L'hôte l'appelle à trois endroits :
- à la fin d'un index, dans son fil, avec une ligne au journal de l'index ;
- à la fermeture d'un backend ;
- au repos, après 60 s sans requête ni travail en cours, une fois par repos, avant même l'arrêt par
  la socket.

L'unité systemd peut poser `MALLOC_ARENA_MAX` (`installer-l-unite-de-l-hote.sh --arenes <n>`), mais
ne le fait pas par défaut (`0`, le défaut de glibc) : voir le débit, plus bas. Le témoin `allocateur::tests::une_memoire_liberee_revient_au_systeme`
se joue dans un processus fils : 512 Mio écrits puis libérés y restent tenus, et le rendu en rend
plus de la moitié (528 → 17 Mio mesurés).

**La mesure** (protocole du banc, ci-dessus). Périmètre, tel quel :
- luciepc, corpus rag3db à `c6bb094fb`, 6 117 fichiers, tampon de 2 Gio ;
- phase plein texte et relations, vecteurs en dette (sans service d'embarquement) ;
- passes une à une, hôte isolé (`RAG3WEAVER_DAEMON_DIR` à part, port 7983) ;
- mémoire comptée en RssAnon + VmSwap, parce que luciepc envoie l'anonyme inactif en échange même
  seul, dès T0 (`swappiness` 150, échange zram de 93,6 Gio) ;
- A sur `bacf411e1` (sans correctif), B, C4 et C2 sur `cae40d28f`.

Bornes : T3 (repos, backend ouvert) ≤ T0 + tampon + 500 Mo ≈ 2 650 Mio ; T4 (backend fermé, 60 s)
≤ T0 + 500 Mio ≈ 605 Mio. En Mio :

| | T3 | T4 | pic VmRSS |
|---|---|---|---|
| A, sans correctif | 5 814, échoue | 4 648, échoue | 6 572 |
| B, `malloc_trim` | 2 329, tient | 567, tient (39 Mio de marge) | 6 124 |
| C4, et `MALLOC_ARENA_MAX=4` | 2 248, tient | 392, tient | 5 116 |
| C2, et `MALLOC_ARENA_MAX=2` | 2 398, tient | 340, tient | 5 608 |

A échoue aux deux bornes : la mesure voit le défaut. Les trois variantes du correctif tiennent les
deux. Le rendu de fin d'index rend environ 3,5 Go en moins de 200 ms (B : `VmRSS` 5 983 → 2 505
Mio). Le rendu à la fermeture fait tenir T4 (B : 531 → 203 Mio) : sans lui, la première série
échouait à T4. CSV et journaux : `~/.cache/rag3weaver-build/mesure-malloc/seq-0056/` sur les deux
postes.

**La réserve du banc** : la marge de B à T4 est mince, 39 Mio sous la borne de 500 Mo. C'est le rendu à
la fermeture d'un backend (`cae40d28f`) qui la fait : c'est la pièce à ne pas perdre.

**Le débit de `MALLOC_ARENA_MAX`** : deux passes par réglage, sous `poste mesure` (porte fermée, rien
d'autre de lourd), en alternance sans / 4 / 2 / sans / 4 / 2, binaire `cae40d28f`, même corpus et même
tampon. L'écart entre deux passes d'un même réglage (2 s au plus) est vingt fois plus petit que
l'effet. Dossier : `~/.cache/rag3weaver-build/mesure-malloc/debit-0133/` sur luciepc.

| réglage | durées de l'index | médiane | écart | pic VmRSS |
|---|---|---|---|---|
| sans | 38 s, 36 s | 37 s | — | 6 703 / 6 586 Mio |
| `MALLOC_ARENA_MAX=4` | 50 s, 52 s | 51 s | +38 % | 6 145 / 6 058 Mio |
| `MALLOC_ARENA_MAX=2` | 76 s, 74 s | 75 s | +100 % | 6 341 / 6 448 Mio |

La règle du banc était une perte de moins de 10 % : ni 4 ni 2 ne la tiennent, pour environ 0,5 Go de
pic en moins. **Verdict du banc : aucun `MALLOC_ARENA_MAX`**. Le rendu après le travail, ce que ce
ticket demandait, tient par `malloc_trim` seul.
