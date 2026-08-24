# Le plancher de commit : trouvé, corrigé, mesuré — 733 ms → 9,6 ms

Réponse au doc 17. Votre mesure était juste, votre méthode aussi, et la
conclusion « le processus dort » était la bonne — à un détail près : il ne
dormait pas dans un sleep ou un poll, il dormait dans **fsync**.

## Le mécanisme

Trois pièces s'emboîtaient :

1. Le `ManagedDirectory` (le registre interne des fichiers d'un index)
   réécrit `.managed.json` via `atomic_write` **à chaque fichier enregistré**.
   Un commit v3 crée ~25 fichiers de segment + 8 sidecars SFX par champ, par
   shard — donc des dizaines d'`atomic_write` par commit sale.
2. `BlobDirectory::atomic_write` déléguait au répertoire mmap du **cache
   local**, dont l'`atomic_write` fait un `sync_data` — un fsync par appel.
3. Sur btrfs+zstd, un fsync coûte ~65 ms (mesuré dès les benchs du 23 août,
   c'est pour ça que la construction 50k se faisait « en RAM puis copie sans
   fsync »).

Des dizaines de fsyncs × ~65 ms = votre plancher. Tout colle avec vos quatre
observations : ça sature (le nombre de fichiers par commit plafonne avec les
segments, pas avec les documents), le chemin à vide est bon marché (aucun
fichier créé → aucun fsync), release ne change rien (fsync n'est pas du CPU),
et le processus est idle à 88 % (il attend le disque). Votre intuition « si
c'est par shard, 4 shards comptent » était bonne aussi : plus de shards = plus
de fichiers = plus de fsyncs.

L'absurdité, et donc le correctif : ces fsyncs protégeaient un cache
**jetable**. Dans le montage blob, la durabilité vient du `BlobStore`
(`store.save` est synchrone, inchangé) ; le cache local se reconstruit depuis
les blobs à la réouverture — c'est le contrat testé par
`v3_blob_storage_create_reopen_search`, qui efface le cache entre deux
ouvertures. Fsyncer ce cache n'achetait rien.

## Le correctif (`9a66fbf`, poussé)

`BlobDirectory::atomic_write` écrit maintenant le cache local en
temp + rename **sans fsync** (l'atomicité locale est conservée pour les
lecteurs mmap), et sauve le blob comme avant. Rien d'autre ne change.

## Les chiffres (MemBlobStore, votre montage, `test_commit_floor` chez nous)

| | avant | après |
|---|---|---|
| 2 shards, 9 docs (votre cas) | 733,9 ms | **9,6 ms** |
| 2 shards, 900 docs | 12 636 ms | **19,6 ms** |
| 4 shards, 9 docs | 4 095 ms | **9,9 ms** |
| 4 shards, 900 docs (votre prod) | 17 179 ms | **38,1 ms** |
| commit à vide | 6-941 ms | **0,4-2,9 ms** |

(Chez nous c'était encore pire que chez vous — notre /tmp est sur btrfs.
Le harnais est committé : `lucivy_core/tests/test_commit_floor.rs`,
`cargo test --release -p lucivy-core --test test_commit_floor -- --ignored --nocapture`.)

Validation : suite ACID complète verte (dont réouverture depuis blobs seuls et
« aucun appel au store après close »), lucivy-core complet vert, lib
1415/1415.

## Ce que ça veut dire chez vous

- **Épinglez le submodule sur `9a66fbf`** et rejouez vos suites. Votre
  estimation « la suite retomberait autour de 40 s » devrait se vérifier, ou
  mieux.
- **Committer une fois par drain redevient le bon design.** Plus besoin
  d'envisager de committer moins souvent : le commit sale coûte ~10-40 ms,
  proportionné aux segments créés, plus d'attente pure.
- Un point à surveiller avec le **vrai** store (CypherBlobStore) : chaque
  réécriture de `.managed.json` part aussi dans le store (un `save` par
  fichier enregistré — des dizaines d'aller-retours DB par commit). Avec
  MemBlobStore c'est invisible ; avec une base derrière, ça peut redevenir le
  terme dominant. Si vos chiffres post-`9a66fbf` restent au-dessus des nôtres,
  mesurez le nombre d'appels `save` par commit et dites-le nous : le batching
  de ces sauvegardes est le correctif suivant, et il est chez nous.

Merci pour le doc 17 — quatre points de mesure, deux preuves d'innocence du
CPU, et la forme du coût : il y avait tout ce qu'il fallait pour trouver en
une heure.
