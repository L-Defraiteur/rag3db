# Une lecture pendant un lot réindexé voit le lot à moitié, sous la lib entière

- **État** : ouvert (13 novembre 2026), relevé par archi pendant la passe de son lot (la bulle
  fantôme du chat et « dire vrai »). Départagé le 14 novembre : **c'est un repli de rag3weaver, pas le
  moteur**. L'instantané de lecture d'un appel, plafonné à 1 s, s'est validé en cours d'appel, et la
  suite de l'appel a lu hors instantané sans le dire. La première conclusion de recherche (« lecture
  sale du moteur », 13 novembre au soir) est retirée ; elle reste écrite plus bas, avec son retrait.
  Correctif à trancher par Lucie (la proposition est en fin de ticket).
- **Famille** : produit
- **Gravité** : passe rouge sans cause dans le lot qui la joue ; et peut-être une lecture qui voit
  un lot réindexé à moitié, si ce n'est pas le seul témoin qui cède sous la charge.
- **Atteignable en service** : oui. Un appel de lecture qui dure plus d'une seconde pendant une
  réindexation de la montre peut répondre avec deux états mêlés, sans le dire.
- **Touche rag3weaver** : oui (`src/backend.rs`, le témoin vers la ligne 7321).

## Ce que c'est

`backend::tests::a_read_during_a_reindexed_batch_sees_the_batch_before_whole` (`a93e98353`,
12 octobre 2026, L3 des lectures) rougit dans la lib entière :

```
panicked at src/backend.rs:7321:13:
assertion `left == right` failed: un lot vu à moitié : {"delivery":{"ok":true,"results":[]},…,
"result":"> Une écriture de l'index est en cours : la réponse porte sur l'état validé, d'avant elle.
# usages: arrivee … ## Définitions (0) (aucune définition indexée sous ce nom) …
  left: false
 right: true
test result: FAILED. 2332 passed; 1 failed; 68 ignored
```

Joué seul, il est **vert deux fois sur deux**, dans le même worktree et le même target.

- **Le code** : master `93cc95a43` (le journal des runs écrit à l'arrêt), plus le lot d'archi, qui
  ne touche que `Chat.tsx` et l'atelier.
- **La lib** : `835d5298f`, sans `MOTEUR_ANCIEN`.
- **Le poste** : lucie-tablette, `POSTE_MEM_MAX=32G poste lourd`, le jeu large.
- **Le journal entier** : `~/.cache/rag3weaver-build/archives/fantome-complete-rouge-*/lib.log`.

## Ce qu'il faut pour le fermer

Le départager : un témoin qui dit trop sous la charge (un seuil, un ordre de fils), ou une
lecture qui voit vraiment un lot à moitié. Puis le correctif ou la borne du témoin, et le témoin
vert dans la lib entière.

## Le départage (13 novembre 2026, recherche)

**L'armement** (branche `lecture-pendant-un-lot`, 3afcf257e) :

- un lot en échec repris un à un se dit maintenant au journal de l'hôte (`[montre] … repris un à
  un`) ;
- au premier lot vu à moitié, le témoin attend la fin des lots avant d'échouer. Les lignes
  `[montre]` de la réaction entrent ainsi dans sa sortie capturée. Dans le rouge d'archi, elles
  manquaient, parce que le test avait déjà paniqué quand le lot s'est fini.

**Les tours joués** :

- **luciepc**, 65c7dd96d, lib 835d5298f, lib entière en `--nocapture` : 12 tours verts sur 12. Le
  lot du témoin y est chaque fois d'un seul tenant, `2 fichiers (neuf/__pause_du_lot__.rs,
  neuf/def.rs)`, tenu de 3,8 à 4,9 s, sans reprise.
- **Le portable**, le même code armé, la même lib, sous `poste lourd` à 32 Gio : rouge au premier
  tour. Le journal entier est archivé sous
  `~/.cache/rag3weaver-build/archives/lecture-sale-L3-portable-2112/lib.log`.
  - La ligne du lot : `[montre] 2 fichiers (neuf/__pause_du_lot__.rs, neuf/def.rs) réindexé :
    verrou attendu 0 ms, tenu 3737 ms`. Aucune ligne « repris un à un ».
  - La lecture : « Une écriture de l'index est en cours : la réponse porte sur l'état validé, d'avant
    elle. … Définitions (0) … non attribués (nom ambigu) (1) - function h —
    neuf/__pause_du_lot__.rs:1 (par le nom) ».

**Ce que cela établit** :

- **Le lot est une seule transaction.** `reindexer_en_lot` ouvre `begin_lot` (BEGIN), réingère les
  deux fichiers, tient 2 s (le crochet), puis `commit_lot`. Rien ne commite au milieu : la
  transaction par paquet ne sert qu'à l'indexation entière (`synchroniser`), et les deux chemins du
  lot atomique (`ecrire_un_lot_atomique`, catalog.rs:6335 et :8888) sont gardés par
  `!in_transaction`.
- **`h` n'existe que dans ce lot.** Les deux fichiers sont écrits dans `prep/`, hors de l'arbre
  surveillé, puis le dossier est renommé d'un coup sous `sources/`. La ligne de lot est la seule
  ligne `neuf/` du tour.
- **La lecture passe par une autre connexion.** Le mutex du catalogue est tenu par le lot, donc
  `usages` lit par la vue de lecture. Celle-ci prend une connexion du pool des instantanés (L6) :
  `Rag3dbConnection::read_connection`, une `Connection` neuve sur la même `Database`, en
  transaction READ ONLY ouverte à sa première requête. Sans instantané, la lecture passe par
  `read_conn`, autre connexion encore, en auto-commit. Aucun chemin ne lit par la connexion de
  l'écrivain.
- **La lecture a donc vu une partie d'une transaction non validée** : la ligne `Scope` de `h`, le
  `Symbol` `arrivee`, l'arête `MENTIONS` de `h` vers lui. Elle n'a vu ni le `DEFINES` de
  `def.rs`, ni sa définition. C'est une lecture sale, et partielle.
- **La même famille que la lecture sale sous V2 du 7 novembre** :
  `RollbackUnderReaders.AReaderSurvivesTheRollbackOfAnotherWriter`, un lecteur qui voit des lignes
  non validées pendant le COPY d'un autre écrivain. On ne la voit que sous charge : jamais en 12
  tours à luciepc (24 cœurs), au premier tour sur le portable chargé.

**Ce qui reste** :

1. Le cœur : un repro C++. Un écrivain ouvre une transaction et écrit, dans une table de nœuds et
   deux tables de relations, ce que fait le lot (COPY des relations par le drain des liens). Il la
   tient ouverte. Un lecteur sur une autre `Connection`, en READ ONLY, compte les lignes en boucle,
   sous charge. Toute ligne vue avant le COMMIT est rouge.
2. Le témoin rag3weaver reste tel quel : il dit vrai. Il redevient vert quand le moteur ne laisse
   plus voir une transaction ouverte.

## Le cœur (13-14 novembre 2026)

**La capture.** Le test Rust a été rejoué sous `LD_PRELOAD` à luciepc, sur la lib `835d5298f` et
la branche `ba12af941`. Le capteur note chaque appel de `Connection::query` et de
`executeWithParams`, avec son heure, sa connexion, son fil, son entrée et son issue :
`~/.cache/rag3db-moteur-notes/lecture-sale/capture-1.txt` à luciepc, capteur `capture2.cpp`.
Trois tours, tous verts. Ce que la capture établit :

- **Le lot est une seule transaction** sur la connexion de l'écrivain : BEGIN, des
  `UNWIND … MERGE/CREATE` (aucun COPY dans ce lot), la pause, puis COMMIT. Le Scope de
  `__pause_du_lot__.rs` naît dans le lot : le `SET` qui le cherche au début rend « OK 0 ».
- **Chaque `usages` est une transaction READ ONLY** sur une autre connexion, qui pose trois
  requêtes :
  1. `Symbol {_uuid} <-DEFINES- Scope` ;
  2. `Symbol {_uuid} <-MENTIONS- Scope` ;
  3. un balayage de `Scope.declarations`.
- **Le COMMIT du lot dure moins d'une milliseconde.**

**Le rouge est cohérent avec deux instantanés.** Il a vu « Définitions (0) », puis l'appel `h`
par MENTIONS, « non attribué » faute de définition. C'est exactement ce que donne la requête 1
avant le COMMIT et la requête 2 après, si ces deux requêtes ne partagent pas un instantané.

**Le moteur, témoin C++ sur la forme capturée** : `ReadDuringOpenBatch`, branche
`lecture-sale-sous-lot`, `0066dce28`.

- L'écrivain crée par MERGE, dans une transaction et 400 fois : deux Scope, un Symbol, DEFINES
  et MENTIONS. Il passe un point de reprise tous les 50 lots.
- Six lecteurs posent les trois questions dans un même instantané READ ONLY.

À luciepc : 5 tours seul et 5 tours sous 14 fils de charge, environ 300 000 lectures, **aucune
vue à moitié**.

**Le test Rust sous charge** : 60 tours sous capture et sous 15 fils de charge, **60 verts**. Le
rouge ne se reproduit pas à luciepc : 0 sur 75 en tout, 12 tours de recherche compris.

**Deux chemins de rag3weaver donnent ce rouge sans faute du moteur** (`src/read_pool.rs`) :

1. **Le pool plein.** Après `ATTENTE` (50 ms), l'appel passe sans instantané, et chaque
   requête valide seule. La réponse le dit par la ligne `SANS_INSTANTANE` (« Pas de connexion de
   lecture libre… »).
2. **L'instantané plafonné** (`RAG3WEAVER_SNAPSHOT_MAX_MS`, 1 000 ms par défaut). Passé le
   plafond, la requête suivante valide l'instantané et en ouvre un neuf. Sur un poste très
   chargé, un appel qui dépasse 1 s voit alors le COMMIT entre deux de ses requêtes.

Le rouge du portable ne permet pas de trancher. Son message est coupé à 400 caractères par le
`panic!` (`&u[..u.len().min(400)]`), avant la fin de la réponse où la ligne `SANS_INSTANTANE`
paraîtrait.

**Ce qu'il faut pour trancher**, côté rag3weaver : armer le témoin pour qu'au rouge, il imprime

- la réponse entière ;
- `ReadPool::sans_instantane` ;
- le nombre d'instantanés renouvelés par le plafond.

Puis rejouer au portable, sous la charge du rouge. Si l'un des deux compteurs a bougé, le moteur
est hors de cause, et le témoin doit tenir compte du repli : il dit vrai, mais pas sur le moteur.
Si aucun n'a bougé, le cœur reprend, avec la capture au portable.

## Le retrait et la cause (14 novembre 2026, recherche)

**Ma conclusion du 13 novembre au soir était trop rapide.** J'avais vérifié que le lot tenait dans
une seule transaction, et que la lecture passait par une autre connexion. Je n'avais pas vérifié
que l'appel gardait son instantané d'un bout à l'autre. Le cœur ne reproduit rien dans le moteur (la
section précédente), et il désigne deux replis de `read_pool`. Le témoin réarmé tranche entre eux.

**Le réarmement** (branche `lecture-ecourtee`, c1dae9363) : au lot vu à moitié, le témoin imprime
la réponse entière et les compteurs `reads` de `index_states` (`without_snapshot`,
`shortened_snapshots`), au départ, au rouge et à la fin.

**Le rouge**, au portable, lib e93710f36, sous `poste lourd` à 32 Gio, au tour 2 de la lib entière.
Le journal est archivé sous `~/.cache/rag3weaver-build/archives/lecture-ecourtee-L3-portable-0015/lib.log` :

```
un lot vu à moitié (définition false, appel true) ;
instantanés de lecture au départ {"connections":1,"max":8,"shortened_snapshots":0,"without_snapshot":0},
au rouge {"connections":1,"max":8,"shortened_snapshots":1,"without_snapshot":0}, …
[montre] 2 fichiers (neuf/__pause_du_lot__.rs, neuf/def.rs) réindexé : verrou attendu 8 ms, tenu 4700 ms
```

- **Le pool n'a jamais été plein** (`without_snapshot` reste à 0). Le repli (1) est hors de cause.
- **Un instantané a été écourté, celui du rouge** (`shortened_snapshots` de 0 à 1). L'appel a duré plus
  de 1 s (`RAG3WEAVER_SNAPSHOT_MAX_MS`) sur le portable chargé. Sa requête des définitions a lu dans
  l'instantané, avant le COMMIT du lot. Ses requêtes suivantes ont lu requête par requête, après lui :
  d'où `h` par MENTIONS, sans définition. La réponse ne le dit pas : seul le compteur le note.

**La raison d'être du plafond est tombée.** Il existait parce qu'un point de reprise forcé (celui d'un
COPY) attendait les lecteurs ouverts, puis échouait au bout de 5 s. Le témoin
`read_pool::tests::un_lecteur_ouvert_ne_retient_pas_le_point_de_reprise_d_un_copy` était ignoré :
« vert sous la lib de V2-2 (f8c0c8df0) : se retire quand courant la porte ». Sous e93710f36, il est
vert dix fois sur dix (COPY 8 à 14 ms, CHECKPOINT 2 ms, un lecteur ouvert). Son `#[ignore]` est retiré
(4205e1085) : il garde la décision.

## La proposition, à trancher par Lucie

- **Retirer le plafond de 1 s**, ou le rendre très large avec sa raison écrite. Le repli silencieux part
  avec lui.
- **Un appel ne mêle jamais deux instantanés** (la règle de c7), comme filet. Si un instantané doit
  tomber en cours d'appel, l'appel recommence entier sous le nouvel instantané, et la réponse porte
  `instantane_renouvele: n`. Le pool plein suit la même règle : attendre ou refuser en le disant,
  jamais lire sans instantané en silence.
- **Rouge d'abord** : un appel `usages` forcé à durer au-delà du plafond, pendant un lot ouvert, rend
  aujourd'hui une réponse mêlée sans le dire. Après le correctif, il rend un seul état, ou il dit
  qu'il a recommencé.
