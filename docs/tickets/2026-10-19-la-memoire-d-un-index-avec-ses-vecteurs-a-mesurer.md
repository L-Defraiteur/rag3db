# La mémoire d'un index avec ses vecteurs : à mesurer

- **État** : ouvert (19 octobre 2026, G et le banc). C'est la suite du
  [ticket fermé de la mémoire d'un gros index](closed/2026-10-18-l-hote-ne-rend-pas-la-memoire-d-un-gros-index.md),
  dont le correctif n'est prouvé que pour la phase plein texte et relations.
- **Gravité** : mémoire. Rien n'est perdu ni faux.
- **Atteignable en service** : oui, après tout index qui paie sa dette de vecteurs (le rattrapage du
  12 octobre, qui avait laissé 10 Go dans l'échange, en était un).
- **Touche rag3weaver** : oui (l'hôte du démon).

## Ce qui manque

Le correctif (`malloc_trim(0)` en fin d'index, à la fermeture d'un backend et au repos, plus
`MALLOC_ARENA_MAX` dans l'unité) a été mesuré sans service d'embarquement : l'index faisait le plein
texte et les relations, les vecteurs restaient en dette. Les arènes et les tampons de
l'embarquement (les lots envoyés aux services, les réponses, les écritures de vecteurs) peuvent
garder autre chose, que `malloc_trim` ne rend pas.

## Le témoin à jouer

Le protocole du banc, comme au ticket fermé, avec les services d'embarquement de luciepc :
1. une passe A (sans correctif) et une passe B (avec), chacune seule, sur une base neuve ;
2. le corpus rag3db à `c6bb094fb`, tampon de 2 Gio, régime GPU doux
   (`RAG3WEAVER_EMBED_CHAR_BUDGET=4096`, `RAG3WEAVER_GPU_DUTY=70`) ;
3. T0 à T4 relevés par `sonde-hote.sh`, en RssAnon + VmSwap (luciepc envoie l'anonyme inactif en
   échange zram dès T0) ;
4. quand les services sont libres, sans déranger codeparsers ni l'hôte de luciepc.

Le pilote est `mesure-hote.py` (scratchpad de G, copie dans `~/.cache/rag3weaver-build/` sur
luciepc) : il suffit de ne plus retirer `RAG3WEAVER_EMBED*` de l'environnement de l'hôte.

## Mesuré (banc, 19 octobre 2026, nuit)

Conditions : luciepc, les passes l'une après l'autre, sous `poste lourd`, entre deux passes de
codeparsers sur les services d'embarquement. Corpus c6bb094fb, 6 117 fichiers ; 106 794 vecteurs
granite-278m, tous payés (« vecteurs prêts », aucun échec d'embarquement). Tampon de 2 Gio,
régime 4096/70. A sur bacf411e1, B2 sur cae40d28f (le lot malloc_trim). Valeurs en Mio, RssAnon +
VmSwap : sur luciepc, l'échange est de la RAM compressée (zram) où part tout l'anonyme inactif.
Dossiers : `~/.cache/rag3weaver-build/mesure-malloc/vec-0226/` et `vecp-0326/` sur luciepc.

| | T0 | T3, repos backend ouvert (borne ≈ 2 657) | T4, backend fermé (borne ≈ 609) |
|---|---|---|---|
| A | 110 | 6 253, échoue | 4 776, échoue |
| B2 | 109 | **3 073, échoue de 416** | 569, tient |
| B2, passe de preuve (avec smaps) | ≈ 110 | 3 136 | (rendu à la fermeture : VmRSS 974 → 381) |

**Backend fermé, le correctif tient. Au repos, backend ouvert, il ne tient pas.** La borne ne
compte que le tampon, et l'index vectoriel ouvert ne tient rien de taille N hors du tampon : graphe
et vecteurs passent par les pages du gestionnaire de tampon (lecture de l'extension par G).

**Ce qui retient, au repos (passe de preuve, `/proc/<pid>/smaps` à T3, après le malloc_trim de fin
d'index et celui du repos)** :

| famille | Anon + échange |
|---|---|
| les arènes de malloc : 74 projections anonymes alignées à 64 Mio, 93 fils vivants | **1 263** |
| **une seule allocation anonyme de 960 Mio** (à 0x7f872d9de000) | **646** |
| le tampon : la région des pages (807) et celle des pages temporaires (249, réservée à la taille du tampon, `buffer_manager.cpp:73`) | 1 056, sous son budget |
| autres anonymes (deux de 96 Mio, et le reste) et `[heap]` | environ 170 |

Le tampon n'est donc pas en cause. Ce qui dépasse, c'est la mémoire encore allouée dans les arènes
après `malloc_trim`, et un bloc vivant de 960 Mio. Hors du tampon, environ 2,1 Go sont tenus au
repos ; sans les vecteurs, environ 1,3 Go (seq-0056, B). Les vecteurs ajoutent environ 0,8 Go, dont
le bloc de 960 Mio est le suspect : un tampon d'embarquement, des lots reçus ou des files de la
dette gardés après le paiement, côté rag3weaver. C'est à chercher par G.

## Pour le fermer

B tient les mêmes bornes que la phase plein texte (T3 ≤ T0 + tampon + 500 Mo, T4 ≤ T0 + 500 Mo),
et A les manque. Sinon, chercher ce que l'embarquement garde. T4 se relève comme au ticket fermé
(60 s après la fermeture du backend, après le rendu de la fermeture) : en plein texte, la marge
de B n'y était que de 39 Mio.
