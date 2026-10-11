# La montre de fichiers coûte sept dixièmes d'un cœur en continu

- **état** : corrigé (le commit de la réconciliation et du balayage par dates, 11 octobre 2026) — à remesurer sur l'hôte de Lucie
- **gravité** : coût (un cœur pris en permanence sur le poste de travail)
- **atteignable en service** : oui, dès qu'un manifeste déclare `watches` (le gabarit `code` le fait)
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

Sur l'hôte de Lucie (pid 107223, binaire e9387d545), le fil `montre-code` prend
**69 tops d'horloge en 10 s, soit environ 69 % d'un cœur**, en continu, au repos : aucun
appel, aucune édition. La montre balaie les 7 310 fichiers du workspace
(`every_ms: 1000`, `settle_ms: 300`, ce que le gabarit déclare) : un parcours complet du
disque, règles d'exclusion comprises, chaque seconde.

Ce n'est pas un blocage : le balayage ne tient pas le verrou du backend (`/describe`
reste à 3–10 ms pendant qu'il tourne), et une édition est vue en 4,7 à 6 s. C'est un coût,
sur un poste où la compilation et les passes sont déjà ce qui gêne.

## Ce qu'il faut pour le fermer

Une période mesurée, pas posée : le coût d'un balayage pour N fichiers, et la période qui
garde une édition vue en quelques secondes pour un coût négligeable — ou un balayage moins
cher (ne relire les règles d'exclusion qu'à leur changement, `stat` seul hors des
fichiers qui ont bougé), ou les notifications du noyau derrière la même interface, que la
doc de `watch.rs` prévoit « si une mesure le demande ». Cette mesure-ci le demande. Témoin :
le CPU du fil sur 60 s au repos, sous un seuil dit, sur un workspace de la taille de
celui-ci. Chez le proto (la montre est à lui).

## Ce qui a été fait (11 octobre 2026, le proto)

**La mesure d'abord** (bâti de test non optimisé, luciepc, cache chaud) :

| arbre | fichiers | une liste de la source | un tour au repos, ancienne forme | un tour au repos, nouvelle forme |
|---|---|---|---|---|
| `src/` du moteur | 1 653 | 6 ms | liste + dates ≈ 8 ms | 1,7 à 2,8 ms |
| le dépôt entier | 6 818 | 20 à 33 ms | liste + dates ≈ 30 à 55 ms | 9 à 22 ms |

La première forme relisait **la liste de la source à chaque tour** (le parcours du
workspace avec ses règles d'exclusion), puis la date de chaque fichier. Le hachage, lui,
n'a lieu que pour un fichier dont la date ou la taille a bougé.

**Une note sur le chiffre de départ** : si les « tops » sont ceux de `/proc/<pid>/task/<tid>/stat`,
ils comptent en centièmes de seconde (`USER_HZ = 100`). 69 tops en 10 s font alors 0,69 s de
CPU en 10 s, soit **environ 7 % d'un cœur**, et non 69 % — ce qui concorde avec la mesure
ci-dessus de l'ancienne forme (une liste par seconde). À confirmer par la remesure.

**La correction** : un tour au repos ne lit plus que **la date des fichiers connus et de
leurs dossiers**. La liste n'est relue que quand un dossier bouge (une création, une
suppression ou un renommage changent la date du dossier) ou, en filet, toutes les
`relist_ms` (60 s par défaut). Une modification en place est vue au tour suivant ; une
création ou une suppression aussi (le dossier bouge).

**Les compteurs** : `index_state` rend `watches.<nom>` — tours, relectures, dates lues,
hachages, octets hachés, temps par phase (`list_us`, `hash_us`, `other_us`) et les cinq
chemins les plus hachés. Si une montre coûte encore au repos sur une vraie base, ces
compteurs disent où : un gros fichier qui change sans cesse sous la racine (un journal,
une base non ignorée) se verrait en tête de `most_hashed`.

**Témoins** : `watch::tests::a_quiet_scan_does_not_relist_the_source` (au repos, aucune
relecture ; une création en fait une ; une modification en place, non ; le filet de
période, une) et `a_quiet_scan_of_the_engine_tree_costs_less_than_a_listing` (la mesure
sur `src/` et sur le dépôt entier, imprimée).

**À faire par la mémoire** : remesurer le fil `montre-code` sur l'hôte de Lucie, au repos,
et lire `watches.code` dans `index_state`. S'il coûte encore, rouvrir ce ticket avec les
compteurs.
