# Un CREATE_VECTOR_INDEX à efc 50 va deux fois plus vite, au même rappel, sur du code

- **État** : ouvert (22 novembre 2026, hotfixs, au mot de c7), en attente d'une mesure du banc sur d'autres corpus
- **Famille** : moteur
- **Gravité** : lenteur. Le CREATE d'un index de code à efc 200 coûte deux fois le temps d'un CREATE à efc 50, pour
  le même rappel sur code-lourd.
- **Atteignable en service** : oui, à chaque CREATE ou reconstruction d'index (au plus, un DROP puis un CREATE
  d'index rendus nécessaires).
- **Touche rag3weaver** : oui. rag3weaver crée ses index à efc 200 ; le défaut du moteur est 200 aussi.
- **Signalé par** : hotfixs, la mesure de l'insertion pour ED
  (`docs/coeur-cpp-hotfixs-tickets/22-11-2026/01-le-cout-d-un-set-de-vecteur-sous-hnsw.md`)
- **Pour** : banc (la mesure), puis le moteur ou rag3weaver (le défaut)

## Le constat

Sur 122 902 vecteurs de `Scope_Chunk` de code-lourd (granite-278m, 768 dimensions, cosinus), en Release, à luciepc,
sous charge :

| efc | CREATE | rappel@10 (200 requêtes hors de la table, contre l'exhaustive) |
|---|---|---|
| 200 | 123 à 127 s | 0,997 à 0,9985 |
| 100 | 75 à 87 s | 0,9975 à 0,9985 |
| 50 | 55 à 57 s | 0,997 à 0,998 |

L'écart de rappel entre les trois est dans la dispersion d'une variante à l'autre.

## Ce qu'il faut avant tout changement de défaut

1. **La mesure du banc sur d'autres corpus que le code** : la mémoire longue (fiches et apports), des docs, et la base
   gaussienne du banc, dont la dimension intrinsèque est maximale. On y mesure le temps du CREATE et le rappel@10 à efc
   200, 100 et 50, sous `poste mesure`, les deux côtés dans les mêmes conditions.
2. **Le choix du lieu** : un défaut du moteur (`HNSWIndexConfig`, efc), ou un réglage de rag3weaver par sorte de
   base (code, mémoire, docs).
3. **Une remarque liée** : à l'insertion (SET, CREATE de nœud), l'efc de l'index ne sert pas aujourd'hui. La
   recherche prend l'efs par défaut des requêtes (`QueryHNSWConfig{}`, `hnsw_index.cpp:613` et `:626`). Ce point a
   son propre lot chez hotfixs : l'insertion prendra l'efc de l'index. Une fois ce lot posé, efc réglera aussi le coût
   et la qualité des insertions, et la mesure du point 1 devra l'inclure.
