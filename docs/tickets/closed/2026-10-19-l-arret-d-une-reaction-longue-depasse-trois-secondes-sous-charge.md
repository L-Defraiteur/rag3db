# L'arrêt d'une réaction longue dépasse trois secondes sous la charge de la lib entière

- **État** : fermé (19 octobre 2026, everything declarative) — c'était la borne du test, pas l'arrêt. Vu une fois, le 19 octobre 2026 à 02 h 45, à luciepc (lib entière, `--features rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres`, lib cf71c929d, base 5f9cf0983 plus le correctif du bus, puis 95bc668f3).
- **Gravité** : test rouge sans défaut prouvé du produit ; au pire, un arrêt d'hôte plus lent que promis.
- **Atteignable en service** : peut-être (un arrêt pendant une réaction de retrait sur un poste chargé).
- **Touche rag3weaver** : oui (`Backend::stop_reactions`).

## Ce que c'est

`backend::tests::stopping_cuts_a_long_reaction_and_the_restart_resumes_it` exige que
`stop_reactions` rende la main en moins de 3 s pendant la réaction de retrait de cent
fichiers. Sous la charge de la lib entière, il a attendu 4,498906195 s :

```
panicked at src/backend.rs:5313:9:
l'arrêt a attendu la réaction : 4.498906195s
```

Rejoué seul juste après, trois fois avec le correctif du bus et trois fois sans : vert six fois
sur six. Le lot du bus (95bc668f3) ne touche pas les réactions.

## La recette

Pas de recette déterministe : il faut la charge de la lib entière. Le soupçon : l'arrêt est
coopératif entre deux retraits, et un retrait coûte « des centaines de millisecondes sur ce
moteur » (le commentaire du test) ; sous charge, un seul retrait peut dépasser 3 s. À prouver
par la pile pendant l'attente (`eu-stack` avec `LD_PRELOAD` de `ptracer-any.so`), avant de
lire le code.

## Le témoin

Le test lui-même. Voisin fermé, l'autre assertion du même test :
[la reprise après arrêt rougit sous charge](closed/2026-10-18-la-reprise-apres-arret-rougit-sous-charge.md).

## Pour le fermer

Savoir où l'arrêt attend (la pile), puis : soit rendre l'arrêt préemptif au sein d'un retrait,
soit reformuler la promesse (borne liée au coût d'un retrait plutôt qu'à 3 s fixes), avec la
preuve que ce que le test mesurait reste mesuré.

## Ce qu'on a trouvé (everything declarative, 19 octobre 2026)

**Où l'arrêt attend** : à la fin du lot en cours. `ReindexFileNode` lit le drapeau d'arrêt entre
deux lots (« l'arrêt passe entre deux lots »), et un lot est une transaction
(`reindexer_en_lot`). Le premier lot fait quatre fichiers (`FIRST_BATCH`) ; les suivants se
dimensionnent pour tenir le verrou environ `HOLD_MS` (400 ms) d'après le coût du précédent. La
promesse écrite dans `stop_reactions` (« au prochain fichier ») était périmée depuis les lots.

Mesuré sur luciepc, sous seize boucles CPU, par un diagnostic (non commité) qui sépare l'arrêt
des montres de celui des réacteurs et lit le retard avant et après :

- **dans 20 passes sur 20, l'arrêt du test tombait avant le premier fichier**. Il prenait 18 ms,
  le retard valait 100 avant comme après, et le test ne coupait rien du tout ;
- **arrêté en pleine réaction** (un lot soldé), sur 10 passes : un fichier coûte 130 à 200 ms,
  l'arrêt prend 0,3 à 0,6 s, et la réaction solde encore deux à quatre fichiers après la
  demande (la fin du lot en cours) ; l'arrêt des montres, lui, prend 5 à 21 ms.

Sous la charge de la lib entière, un premier lot de quatre fichiers à plus d'une seconde chacun
fait les 4,5 s du rouge. L'arrêt tient sa conception (il n'attend que le lot en cours) ; la borne
de 3 s ne la mesurait pas.

## Le correctif

- La promesse reformulée, comme le ticket le proposait : `stop_reactions` dit « au prochain lot ».
- Le témoin juge l'**ordre**, pas une durée, et mesure ce qu'il annonçait. Il attend que la
  réaction soit en cours (un lot soldé, le retard entre 0 et 100), puis il arrête, et exige que
  du retard reste : l'arrêt est revenu avant la fin de la réaction. La reprise au remontage est
  inchangée.
- Rouge d'abord : la variante qui attend la fin de la réaction avant d'arrêter rougit (« l'arrêt
  a attendu la fin de la réaction : plus aucun retard »).
- Le correctif : 0 rouge sur 20 passes sous seize boucles CPU.

L'arrêt préemptif au sein d'un lot n'est pas fait. Il couperait une transaction, et le lot borné
à `HOLD_MS` est déjà ce que l'arrêt attend au plus, hors premier lot.
