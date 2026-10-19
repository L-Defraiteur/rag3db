# Un chargement d'extension à la réouverture se fait passer pour un retrait de l'index

- **État** : corrigé par le commit qui déplace ce ticket (19 octobre 2026, seconde session cœur
  C++, forme relue par le cœur C++) ; voir « Ce qui a été fait », à la fin.
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

## Ce qui a été fait (19 octobre 2026)

**La cause, relue** (elle remplace la lecture d'abord attribuée à hotfixs plus haut) : le holder
n'est pas remplacé. `VectorExtension::load` enregistrait d'abord ses fonctions (QUERY_VECTOR_INDEX
et les autres), puis le type d'index, puis chargeait les index (`initHNSWEntries` : `setAuxInfo`,
`loadIndex` sur le même holder, même holderID). Dans l'intervalle, une requête se liait,
`hasIndexHolder` ne demandant pas l'index chargé. `captureForQuery` ne retenait ensuite que les
holders chargés, n'en trouvait aucun, et concluait au retrait.

**Le correctif** :
- les extensions vector et geo chargent leurs index (type enregistré, puis chargement) **avant**
  d'enregistrer leurs fonctions : une requête ne se lie plus qu'à un index chargé ;
- la capture d'une recherche est commune à tous les genres d'index, dans `NodeTable`
  (`captureVisibleIndexes`, à la place de `getVisibleSharedIndexes`). Un exemplaire visible mais
  pas encore chargé s'y refuse par « is not loaded yet », un refus passager, et non par l'absence
  de l'index. Le choix de l'exemplaire par l'instantané du catalogue reste au genre d'index
  (`OnDiskHNSWIndex::captureForQuery`) ;
- un écrivain qui croise un index pas encore chargé est refusé par son nom
  (`INDEX_NOT_LOADED_FOR_WRITE`), sous le verrou de la liste, que le chargement prend en exclusif.

**Témoins** :
- `IndexReopen.AQueryDuringTheLoadIsNotToldTheIndexIsGone` : l'extension vector s'arrête 1,5 s
  après l'enregistrement de ses fonctions (le crochet `RAG3DB_TEST_PAUSE_AFTER_VECTOR_FUNCTIONS_MS`),
  et une requête s'y glisse. Il est rouge 3/3 sur bc2cb48e7, avec le crochet posé à l'ancien
  ordre (« Index doc_index of table Doc does not exist any more »), et vert 3/3 avec le correctif ;
- `IndexReopen.QueriesAndWritersWhileTheExtensionLoads`, avec des requêtes et des écrivains en
  boucle pendant 30 rechargements, est vert des deux côtés : la fenêtre s'ouvre trop rarement sans
  le crochet ;
- `IndexReopen.QueryWhileTheExtensionLoads` sort de probabilistic.txt : vert 3/3 avec le
  correctif ;
- les suites proches (VectorIndexRetention, Rebuild, ExtensionIndexRecovery, DropCreate et les
  CREATE de VectorIndexUpdate) sont vertes (25 tests).

