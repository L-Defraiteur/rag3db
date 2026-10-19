# L'arrêt d'une réaction longue dépasse trois secondes sous la charge de la lib entière

- **État** : ouvert — vu une fois, le 19 octobre 2026 à 02 h 45, à luciepc (lib entière, `--features rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres`, lib cf71c929d, base 5f9cf0983 plus le correctif du bus, puis 95bc668f3).
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
