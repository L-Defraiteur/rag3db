# Réponse au doc 20 : le flush coûte 4,5 ms, ne regroupez pas les fichiers de segment

`e6176f5` épinglé, tout rejoué. Vos attendus sont tenus et au-delà :

| | avant | après |
|---|---|---|
| `kb_and_relation_persist_and_reopen` seul | 43 s | **2,5 s** |
| `e2e_idempotent_registration` (22 tests) | 178,6 s | **9,9 s** (33 s ce matin) |
| `e2e_symbol_search` | 32,1 s | 1,2 s |
| `e2e_search` | 60,6 s | 10,1 s |
| passe complète, 12 suites | ~4 min | ~90 s (dont 60 s de recompilation après le bump) |

Et votre correction de mon doc 19 est acceptée telle quelle : le trou luciole
est réel, mais il bloque sans fin — ce n'était pas lui. J'avais laissé ouverte
la question « qu'est-ce qui débloque après 25 s », et c'était celle qui
tranchait. Un erratum est en tête du doc 19.

## Votre question : regrouper les ~35 fichiers d'un segment en un seul blob ?

Non, et voici le chiffre.

Depuis hier, `CypherBlobStore` est derrière un tampon write-back
(`BufferedBlobStore`) qui ne pousse que la dernière version de chaque clé, et
`save_many` envoie tout en **une** requête `UNWIND … MERGE … SET` (le moteur
accepte un `BLOB` dans une liste de structs). Profil 9 documents, 4 shards,
avec vos correctifs :

```
blob flush (drain): 282 save(s) reçus → 114 poussés, 168 aller-retours évités,
                    69 516 octets, 4,5 ms
drain()            :  99,9 ms
commit lucivy seul :  14,8 ms
```

Le coût unitaire de `save` que vous proposiez de mesurer n'existe plus comme
terme : 114 blobs partent en un aller-retour de 4,5 ms. Regrouper les fichiers
d'un segment ramènerait 114 lignes à ~4 dans la même requête — quelques
millisecondes au mieux, contre un format de blob composite à maintenir chez
vous et un `load_range` paresseux à re-câbler. Le ratio n'y est pas. Si un jour
la taille totale d'un flush (pas son nombre de lignes) devient le problème,
on rouvrira — mais ce sera un problème de volume, pas de comptage.

## Ce qui reste dans le drain est chez nous

100 ms de drain, dont 15 de commit lucivy et 4,5 de flush. Les ~80 ms
restants sont notre pipeline : agrégation KB, écritures d'entités, chunking,
graphe de dataflow. Rien à vous demander là-dessus.

## Pour votre point ouvert

`test_luce_playground_search` (highlight fuzzy coupé au milieu d'un `→`) :
noté, sans effet chez nous — nos tests d'accents et de séquences ZWJ passent
au span près en mode `Symbol`. Si vous voulez un cas de plus pour l'épingler,
notre corpus `e2e_symbol_search` a un `—` (tiret cadratin) dans le document
`accents`, jamais requêté à ce jour.

## Le bilan de la journée, côté chiffres

Quatre couches retirées en séquence, chacune mesurée avant la suivante, sur le
même profil de 9 documents (drain) :

```
1331 ms   départ — index C++ maintenu en double
 540 ms   fsync retirés du cache jetable            (vous, 9a66fbf)
 193 ms   tampon write-back, 1518 → 225 saves       (nous)
 145 ms   une requête UNWIND au lieu de 225        (nous)
 100 ms   managed.json au commit + routage 64      (vous, e6176f5)
```

Ce que ni le mode release ni un profil de compilation n'auraient touché :
aucune de ces couches n'était du calcul.
