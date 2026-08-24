# Réponse au SIGSEGV — un correctif chez nous, une vérification chez vous

Vos trois faits étaient les bons, et le point de principe aussi : après
`close()`, rien de ce que lucivy détient ne doit plus pouvoir toucher le
store. **Ce n'était pas garanti.** Corrigé — épinglez `6e6bd24`.

## Ce qu'on a trouvé chez nous

`close()` drainait bien les merges et libérait les writers (vérifié), mais
les **acteurs** (pool de shards, readers, routeur) restaient vivants sur le
scheduler global, chacun tenant des `Arc` des handles de shard — et à
travers eux, votre `CypherBlobStore`, donc la connexion C++. « Fermé »
voulait dire « au repos », pas « inerte » : aucune preuve qu'une tâche
tardive (cascade de merge, GC, message en retard) ne pouvait plus atteindre
un store dont vous étiez en train de libérer le support.

## Le correctif (`6e6bd24`)

- `close()` arrête maintenant les trois pools (message `Shutdown` acquitté,
  acteurs stoppés). Après `close()`, le handle est **inerte** — même
  sémantique que le `CLOSE_LUCIVY_INDEX` qu'il remplace. (`drop_index`
  passe par `close()`, donc couvert aussi.)
- Test-sentinelle `v3_close_means_no_more_store_calls` : un store qui
  enregistre tout appel après armement — commits avec merges en vol,
  `close()`, armement, drop du handle, 300 ms de grâce — doit rester muet.
  Vert.

## Réponse à votre question sur le scheduler

Les 24 threads du scheduler global sont des **workers** : ils ne touchent
rien d'eux-mêmes, seulement les tâches/acteurs qu'on leur donne. Il n'y a
pas (et il n'y aura pas) d'arrêt du scheduler global au close d'un index —
il est partagé par tous les handles du process. La garantie utile est celle
ci-dessus : plus rien de lucivy ne peut lui soumettre du travail qui
référence votre store après `close()`.

## Ce qu'il reste à vérifier chez vous

1. Reprenez `6e6bd24` et rejouez le crash. S'il persiste, le suspect change
   de camp : la **durée de vie de la connexion capturée par votre `qf`**.
   Les `Arc` Rust ne maintiennent pas l'objet C++ en vie — si le Drop du
   `Catalog` détruit la connexion pendant qu'un `CypherBlobStore` encore
   référencé ailleurs peut être appelé, le même symptôme revient sans que
   lucivy soit impliqué.
2. Ordre de drop du `Catalog` : Rust droppe les champs **dans l'ordre de
   déclaration**. La connexion doit être déclarée **après** `fts_handles`
   (et tout ce qui peut l'appeler) pour être détruite en dernier.
3. Si ça crashe encore : `gdb --args <binaire de test> --exact <test>` puis
   `run`, et au SIGSEGV `thread apply all bt` — la pile du thread fautif
   dira une fois pour toutes si c'est un thread luciole (préfixe lucivy/
   luciole dans la pile) ou le destructeur C++. C'est la backtrace qu'il
   nous faudrait si le point 1 ne suffit pas.

Sur le principe « une lib Rust ne devrait pas segfaulter » : d'accord, et
c'est pour ça que le correctif est chez nous même si le déréférencement
fautif est probablement du côté C++ du pont — c'est notre contrat de
`close()` qui laissait la fenêtre ouverte.
