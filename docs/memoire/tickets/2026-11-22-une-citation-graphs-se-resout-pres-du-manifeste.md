# Une citation `@graphs/` se résout près du manifeste, pas près du graphe qui cite

- **État** : ouvert, le 22 novembre 2026.
- **Famille** : produit
- **Gravité** : une copie locale d'un gabarit se casse à chaque pose d'un hôte qui change un graphe livré. La mémoire
  longue de Lucie (`memoire-longue`) a cessé de se charger le 22 novembre au soir, et tous ses outils avec elle.
- **Atteignable en service** : oui. Tout manifeste local qui cite un graphe livré (`builtin:`), quand ce graphe cite
  ses voisins en `@graphs/…`.
- **Touche rag3weaver** : oui. La résolution des graphes cités (`GraphTool`, les citations `@graphs/` et `@plans/`
  du lot graphs-on-disk du 20-21 novembre).
- **Pour** : la session mémoire (96).
- **Signalé par** : c7, le 22 novembre 2026, sur l'hôte ef1cddb8c9 posé à 21 h 12.

## Ce qu'on voit

`recall_fiches` de `memoire-longue` échouait : « le nœud « recall » : graphs/archive_recall.mmd :
/home/lucied/.config/rag3weaver/memoire-longue/backends/memory/graphs/archive_recall.mmd : No such file or
directory ».

La config de Lucie est une copie du gabarit memory datée du 15 novembre. Son `graphs/archive.mmd` local cite
`builtin:backends/memory/graphs/archive_decide.mmd`. Depuis graphs-on-disk, ce graphe livré cite son voisin
`@graphs/archive_recall.mmd`, et cette citation se résout dans le dossier du manifeste local, où le fichier n'existe
pas.

## Remède posé ce soir (réversible)

Le dossier entier est gardé dans `~/.config/rag3weaver/memoire-longue.avant-22-11-2026`. `backends/memory`,
`backends/notebook` et `tools` ont été rafraîchis depuis les gabarits de l'hôte posé, et les réglages de Lucie
(`database`, `files_root`) remis. Le remède tient jusqu'à la prochaine pose qui change un graphe livré.

## Ce qu'on attend

Une citation `@graphs/…` ou `@plans/…` se résout près du graphe qui cite : un graphe livré cite les voisins de son
propre gabarit livré, et un graphe local ceux de sa copie. Le manifeste de Lucie passe ensuite en références
`builtin:` (« suit le binaire »), avec ses seuls réglages en local.

## Témoin attendu (rouge d'abord)

Un manifeste local, dans un dossier sans `graphs/`, cite un graphe livré qui cite `@graphs/<voisin>` : il se charge,
et l'appel joue le voisin livré.
