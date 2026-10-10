# Les comptes de valeurs distinctes se trompent d'environ 13 % : 64 registres

- **État** : ouvert — confort, pas maintenant (orchestration, 10 octobre 2026)
- **Gravité** : estimation imprécise (seuls le planificateur et `STATS_INFO` lisent ces comptes)
- **Atteignable en service** : toujours
- **Touche rag3weaver** : non
- **Ouvert le** : 10 octobre 2026, seconde session cœur C++
- **Pour** : cœur C++

## Ce que c'est

L'HyperLogLog des statistiques a `P = 6`, soit 64 registres (`hyperloglog.h:16-18`) : une erreur
type d'environ 1,04 / √64 ≈ 13 %. Mesuré le 10 octobre (sonde jetable) : 5 909 distincts estimés
pour 5 000, 12 233 pour 10 000, juste après des COPY, sans autre écriture. Un `CALL analyze`
rebâtit le même HyperLogLog : il ne corrige pas cette erreur-là.

## Pour le fermer

Monter `P` : 1 024 registres (`P = 10`) donneraient ≈ 3 % pour 1 Kio par colonne. Les registres
sont sérialisés tels quels (`HyperLogLog::serialize`, `serializeArray<uint8_t, M>`) : changer `M`
change le format des statistiques sur disque, donc la version du stockage, et demande de lire
l'ancienne forme. Témoin : `ColumnStatsTest` avec une marge resserrée (10 % au lieu de 40 %),
rouge avant.

Voir aussi `2026-10-10-analyze-au-seuil.md`.
