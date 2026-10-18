# Un chargement d'extension à la réouverture se fait passer pour un retrait de l'index

- **État** : ouvert
- **Gravité** : refus d'une lecture valide. Une recherche vectorielle liée pendant que
  l'extension charge l'index à la réouverture est refusée comme si l'index avait été retiré, alors
  que personne ne l'a touché. Le refus est nommé, et il suffit de rejouer la requête. Mais il est
  faux.
- **Atteignable en service** : oui. C'est le cas d'un hôte qui rouvre une base et reçoit une
  recherche pendant que son extension se charge.
- **Touche rag3weaver** : oui, au redémarrage d'un hôte.
- **Ouvert le** : 19 octobre 2026, banc de concurrence, à la demande de l'orchestration ; la file
  de hotfixs est pleine.
- **Pour** : hotfixs. C'est un cas du ticket
  `2026-10-18-la-retention-des-index-ne-couvre-que-l-index-vectoriel` (4169b9c77).

## Ce qu'on voit

`IndexReopen.QueryWhileTheExtensionLoads` (`test/transaction/concurrence/lock_bench_test.cpp`)
fait 30 tours. À chacun, il rouvre une base qui porte un index vectoriel. Un lecteur interroge
l'index en boucle (`QUERY_VECTOR_INDEX`) pendant que la connexion principale fait
`LOAD EXTENSION`. Avant le chargement, l'erreur attendue est « … is not defined ». Après, la
recherche doit passer.

Le cœur l'a vu rouge une fois sur quatre sur master a45a15adf, sous `compare_known_red`, par
`no-unexpected-error` (une erreur inattendue) :

```
Index doc_index of table Doc does not exist any more: a concurrent transaction dropped or rebuilt
it after this query was bound. Run the query again.
```

**Le test ne retire ni ne rebâtit jamais l'index** : il ne fait que charger l'extension. Le refus
de la rétention est donc faux.

## La cause

Elle est lue par hotfixs, et reste à vérifier dans le code. `captureForQuery` choisit l'exemplaire
de l'index par les oid de ses tables de relations, à l'instantané de la requête. `loadIndex` charge
un support (holder) non chargé ; un support détaché par le rejeu peut être effacé, puis remplacé
par `addIndex`. Si la liaison capture le support pendant que le chargement le remplace ou le
réattache, la recherche ne retrouve plus son exemplaire et conclut au retrait.

**La fenêtre** est interne au chargement de l'extension. Elle s'ouvre après l'enregistrement des
fonctions (avant, la requête ne se lie pas : « is not defined ») et se ferme au remplacement des
supports d'index.

## Forme probable (hotfixs, à vérifier)

Le chargement garde l'identité du support : même `holderID`, chargé en place sous `indexesMtx` en
exclusif. La capture, elle, attend ou refait le chargement au lieu de conclure au retrait.

## Témoin

**Aujourd'hui** : `IndexReopen.QueryWhileTheExtensionLoads` est rangé dans `probabilistic.txt`,
avec un renvoi à ce ticket. Il n'est pas déterministe, une fois sur quatre.

**Le témoin déterministe demandé** : un crochet de test dans le chargement de l'extension, entre
l'enregistrement des fonctions et le remplacement des supports. Il est à poser avec le correctif,
sans rien changer au chemin de service. Le témoin du banc s'écrit alors ainsi : au crochet, une
autre connexion lie `QUERY_VECTOR_INDEX` (requête préparée) ; on rend la main au chargement ; puis
on exécute la requête préparée. Elle doit passer, sans refus de la rétention. Le test du banc
retournera alors dans la passe ordinaire.
