# Un plan de quelques lignes prend un fil par cœur

- **État** : corrigé (le commit qui déplace ce fichier dans `closed/`, 12 octobre 2026) — les fils d'une tâche sont proportionnés à la cardinalité estimée le long du pipeline (un fil par 2 048 lignes, sous le plafond `threads`, plafond gardé sans estimation) ; les puits nés dans le mapper héritent l'estimation, l'optimiseur remet les cardinalités à jour après toute réécriture ; témoin `ThreadsByCardinality` ; avant/après sur la mesure d'impact de F : appels 2,1 à 2,5 × plus courts (page coeur-cpp/12 §9)
- **Gravité** : coût (une lecture d'outil passe moitié de son temps à mettre des fils en route)
- **Atteignable en service** : oui, à chaque lecture d'outil sur rag3db (impact, usages, liens, recherche)
- **Touche rag3weaver** : oui (les lectures d'outil ; contournable par une connexion de lecture à peu de fils)

## Ce que c'est

Le moteur donne à chaque exécution autant de fils que le réglage de la connexion
(par défaut : tous les cœurs), quelle que soit la taille du plan. Une lecture d'outil
touche quelques lignes (`UNWIND $uuids … MATCH` sur une poignée d'uuids), et la mise en
route des fils coûte plus que le travail. Mesuré
(`extension/rag3weaver/docs/11-octobre-2026-embarquements/06-ou-va-le-temps-d-un-aller-retour.md`,
`e2e_mesure_impact::ou_va_le_temps_d_un_aller_retour_sur_rag3db`, poste à 32 cœurs) :
impact, 150 appels, **7 336 ms à 32 fils, 4 758 ms à 4, 3 855 ms à 1** ; l'exécution
par instruction passe de 1,55 ms à 0,73 ms.

## Ce que font les moteurs établis

PostgreSQL décide du parallélisme **par requête**, au planificateur, sur le coût
estimé : un plan sous `min_parallel_table_scan_size` / `parallel_setup_cost` reste sur
un seul processus ; les ouvriers ne partent que pour un balayage qui les vaut.

## Ce qu'il faut pour le fermer

Proportionner les fils d'une exécution à la cardinalité estimée par le planificateur
(un plan de quelques lignes sur un fil, un balayage ou un COPY sur plusieurs), et un
témoin : la mesure ci-dessus, à fils par défaut, au niveau de la mesure à 1 fil. D'ici
là, rag3weaver contourne par une connexion de lecture à peu de fils (page 06).
