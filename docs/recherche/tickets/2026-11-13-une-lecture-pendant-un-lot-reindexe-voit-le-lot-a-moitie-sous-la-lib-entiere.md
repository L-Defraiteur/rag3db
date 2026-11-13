# Une lecture pendant un lot réindexé voit le lot à moitié, sous la lib entière

- **État** : ouvert (13 novembre 2026), relevé par archi pendant la passe de son lot (la bulle
  fantôme du chat et « dire vrai »). **Départagé le soir même par recherche** : c'est une lecture
  sale du moteur, pas le témoin. Le lot était une seule transaction encore ouverte, et une lecture
  sur une autre connexion, en READ ONLY, en a vu une partie. Pour le cœur, avec un repro C++.
- **Famille** : moteur
- **Gravité** : passe rouge sans cause dans le lot qui la joue ; et peut-être une lecture qui voit
  un lot réindexé à moitié, si ce n'est pas le seul témoin qui cède sous la charge.
- **Atteignable en service** : oui. Toute lecture d'outil qui tombe pendant une réindexation de la
  montre peut voir des lignes non validées.
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
