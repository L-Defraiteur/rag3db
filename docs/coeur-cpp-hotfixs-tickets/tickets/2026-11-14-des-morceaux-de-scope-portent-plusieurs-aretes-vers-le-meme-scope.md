# Des morceaux de Scope portent plusieurs arêtes vers le même Scope

- **État** : ouvert (14 novembre 2026, hotfixs). Il se fermera par le retrait de l'ancien index, le pas
  d'arbre principal qui suivra 1b ; sa cause se juge par le témoin `e2e_liens_sous_point_de_reprise`
- **Famille** : produit
- **Gravité** : à départager. Ce ne sont pas des orphelines : chaque arête relie un morceau vivant à son
  Scope vivant. Mais un même morceau est rattaché jusqu'à sept fois à son Scope, ce qui fausse tout
  compte ou tout parcours qui ne dédoublonne pas (une remontée du morceau vers le Scope rend le Scope
  autant de fois).
- **Atteignable en service** : oui, dans la base de dépôt de Lucie (copie du 13 novembre)
- **Touche rag3weaver** : oui (l'écriture des morceaux de Scope et de leur arête `Scope_CHUNKED_FROM`)
- **Pour** : hotfixs pour la cause ; le correctif à recherche (l'indexation) si `Scope_Chunk` survit à 1b,
  sinon fermé par 1b (c7, 14 novembre)

## Ce qu'on voit

Dans `code-rag3db-depot`, sur la copie de l'instantané `avant-835d5298f` (13 nov., 20 h 07, hôte
arrêté), puis sur sa migration par 1b (`~/.cache/rag3weaver-build/1b-copie/migree-20261114-1641/`, lib
0c41f9fe6), avec les mêmes chiffres dans les deux :

| mesure | valeur |
|---|---:|
| morceaux Scope_Chunk | 127 195 |
| arêtes `Scope_CHUNKED_FROM` | 129 400 |
| morceaux à plus d'une arête | **1 291** (2 à 7 arêtes chacun ; 3 496 arêtes à eux seuls, soit 2 205 de trop) |
| morceaux dont les arêtes visent des Scope différents | **0** : toujours le même Scope |
| arêtes dont le Scope n'est pas le `_parent_uuid` du morceau | 0 |

Exemples (morceau, Scope, nombre d'arêtes) :
- `80a0b597-e4a5-c15f-5096-b64cdae25f01` → `461389fc-04f9-940b-d73c-655233bef0cd`, 3 ;
- `b5eceae9-fa98-5e1d-f36d-b7d20ad588bc` → `ead79a64-f2bd-b60e-4d40-1068fce05d66`, 7 ;
- `a78419ac-0421-5257-2c96-bbea415d7ab7` → `2067ebf2-6642-001a-f519-0d44f1e52cda`, 5.

**Les autres bases et tables sont propres.** code-lourd et code-principal (copies de
`avant-835d5298f`) n'ont aucun doublon, ni dans `Scope_CHUNKED_FROM` ni dans `File_CHUNKED_FROM`. Dans
le dépôt migré, `File_CHUNKED_FROM`, `Library_CHUNKED_FROM`, `ScopeContent_CHUNKED_FROM`,
`ScopeContent_DERIVED_FROM` et `MENTIONS` (comptées par paire de bouts) n'en ont pas non plus.

## L'histoire des exemples (hotfixs, 14 novembre, lecture seule de la copie migrée)

- **D'où viennent les doublons** :
  - **1 140 des 1 291 morceaux** viennent d'un seul fichier, `extension/rag3weaver/templates/ide/app/app.js`.
    C'est le bundle minifié de l'IDE : 501 Ko en 85 lignes, 1 561 Scope (`Pl`, `XT`, `mu`…),
    réindexés à 11 instantanés entre le 7 et le 13 novembre.
  - Les autres viennent de gros fichiers Rust souvent édités : `backend.rs` 53, `catalog.rs` 23,
    `daemon/embeddings.rs` 22, `daemon_host.rs` 17, `search.rs` 14…
- **Combien de fois** : 850 morceaux ont 2 arêtes, 145 en ont 3, 198 en ont 4, 38 en ont 5, 41 en
  ont 6 et 19 en ont 7.
- **Une arête de plus à chaque passage**. Pour un morceau donné, le nœud est unique (un seul offset ;
  `_index` 0 pour les dix exemples), mais ses arêtes ont des offsets dispersés en vagues. Par exemple,
  le morceau `e16be760-…` (Scope `Pl`, `app.js`) a 7 arêtes, aux offsets 130 931, 131 392, 132 380,
  135 404, 138 218, 149 986 et 151 185. Les neuf autres exemples ont les mêmes sept vagues, à quelques
  unités près. Ce sont donc **sept passages différents de l'index** sur ce fichier : à chaque
  réindexation, le morceau inchangé est retrouvé par son uuid, et une arête neuve est posée sans que
  l'ancienne soit vue.

**Ce que retire la règle d'exclusion.** 1 140 des 1 291 morceaux sont dans
`templates/ide/app/app.js`, le bundle minifié de l'IDE. La règle d'exclusion 1 de recherche (en cours),
qui écarte les fichiers minifiés et générés, en retire la plus grande part. Les gros `.rs` souvent
édités restent concernés : `backend.rs`, `catalog.rs`, `daemon/embeddings.rs`, `daemon_host.rs`,
`search.rs`…

## Le chemin d'écriture et une hypothèse (réfutée le soir même, voir plus bas)

L'arête vient de `LinkRecordNode` (« chunk_link », `src/catalog.rs` vers la ligne 6684). Au-delà de
200 arêtes par relation (`COPY_SEUIL`), le lot part par COPY : `copier_les_liens`, dans
`src/dataflow/record_nodes.rs`. Ce chemin **saute les paires déjà en base** depuis le 6 septembre
(9c9ba0967). Il les lit par `existing_links` (`src/dialect.rs:570`) :
`UNWIND $froms AS f MATCH (a:<de> {_uuid: f})-[r:<rel>]->(b:<vers>) RETURN a._uuid, b._uuid`.

Ce `MATCH` passe par **une recherche par clé primaire**. Or le cœur a prouvé le 13 novembre
(`docs/coeur-cpp/tickets/2026-11-13-une-recherche-par-cle-primaire-rate-des-cles-validees-pendant-un-point-de-reprise.md`)
qu'une telle recherche peut **rater une clé validée pendant un point de reprise V2**. Son correctif
n'est entré qu'avec la lib 0c41f9fe6, posée le 14. Les doublons datent du 7 au 13 novembre, donc des
libs d'avant. Un raté sur le morceau rendrait la paire « absente », et le COPY la reposerait. C'est la
forme vue : même morceau, même Scope, une arête de plus par passage, sur les gros fichiers dont les
lots dépassent le seuil du COPY.

Ce n'est pas prouvé. La preuve demande un témoin (confié à hotfixs par c7, le 14 novembre : un e2e, rouge
sous e93710f36 et vert sous 0c41f9fe6, qui atterrit comme non-régression et ferme la cause) :
1. un `Scope_Chunk` et son arête déjà en base ;
2. une réindexation du même fichier, dont le lot de liens passe par COPY, pendant un point de reprise
   V2 ;
3. le compte des arêtes du morceau, rouge sous une lib d'avant 0c41f9fe6 et vert sous 0c41f9fe6.

Si c'est bien ce chemin, la lib posée le 14 arrête la production de doublons, et il reste à décider du
sort de ceux qui existent.

## Pistes à lire

- La base du dépôt est celle qu'un hôte a réindexée au fil de nombreux redémarrages ; c'est aussi elle
  qui accumule 8 809 documents plein texte orphelins (ticket
  `docs/recherche/tickets/2026-11-14-un-scope-efface-du-graphe-garde-son-document-plein-texte.md`). Une
  redécoupe qui recrée l'arête par un CREATE au lieu d'un MERGE, quand le morceau existe déjà, donnerait
  cette forme : même morceau, même Scope, une arête de plus à chaque passage.
- Scope_Chunk a aussi 21 287 places vides sous son offset maximal (des morceaux effacés).

## Le premier témoin : vert sous les deux libs (14 novembre, soir)

`tests/e2e_liens_sous_point_de_reprise.rs`, branche `liens-sous-point-de-reprise` (87f592202), au
portable. 600 paires sont posées une fois puis reprises par un point de reprise. Elles sont ensuite
reversées quarante fois, en écriture automatique, pendant qu'une seconde connexion insère et force des
points de reprise.
- **Sous e93710f36** (garde d'âge levée en le sachant : le seul C++ qui manque à cette lib est
  0c41f9fe6) : **vert**. 143 points de reprise, et 600 arêtes à la fin. Sous 0c41f9fe6 : vert de même.
- **L'instrumentation** (`RAG3WEAVER_INGEST_PROFILE=1`) prouve le chemin. Le premier versement part par
  COPY (« Lien : 600 arêtes, 0 déjà là »). Chacun des 40 reversements lit
  « Lien : 0 arêtes, 600 déjà là ou en double » : la lecture des paires déjà là les a toutes trouvées,
  à chaque tour.
- **Pourquoi c'est vert, d'après le ticket du cœur** : les clés ratées par la recherche par clé primaire
  sont **celles qu'un point de reprise est en train de déplacer de la mémoire de l'index vers le
  disque**, c'est-à-dire des clés validées depuis le point de reprise précédent. Le témoin du cœur les
  rate aux seuils exacts du point de reprise (500, puis 1 000). Seul un lecteur READ ONLY est touché.
  Les clés de ce témoin (g0 à g599) étaient au disque, reprises avant les reversements, donc hors de la
  fenêtre. Dans le vrai cas, le morceau inchangé peut avoir été écrit dans la même session hôte, depuis
  le dernier point de reprise.
- **La forme suivante** : à chaque tour, des morceaux neufs et leurs liens posés, puis reversés pendant
  le point de reprise qui les fait passer de la mémoire au disque.

## Le second témoin, puis l'hypothèse réfutée par le plan (14 novembre, soir)

- **Le second cas** du même e2e (fa4b64e64), à la forme du raté : 30 tours de 300 morceaux neufs, posés
  par COPY puis reversés cinq fois pendant qu'une seconde connexion enchaîne les CHECKPOINT. **Vert sous
  e93710f36**, avec 10 125 points de reprise croisés et 9 000 arêtes pour 9 000 paires. Vert de même
  sous 0c41f9fe6.
- **Le plan de la lecture des paires déjà là**, lu sur la copie migrée (lib 0c41f9fe6) :
  `EXPLAIN UNWIND [...] AS f MATCH (a:Scope_Chunk {_uuid: f})-[r:Scope_CHUNKED_FROM]->(b:Scope) …`
  rend `SCAN_NODE_TABLE` sur Scope_Chunk, filtré par un `SEMI_MASKER` et joint par hachage (`HASH_JOIN`).
  **Ce n'est pas une recherche par clé primaire.** Seule une clé littérale,
  `MATCH (a:Scope_Chunk {_uuid: 'x'})`, prend `PRIMARY_KEY_SCAN_NODE_TABLE`.
- **L'hypothèse du raté de la clé primaire est donc réfutée.** La lecture d'`existing_links` ne passe pas
  par l'index de clé : le défaut prouvé par le cœur ne peut pas la tromper. Les deux verts sous
  e93710f36 s'expliquent ainsi. La cause des arêtes en double est à chercher dans les deux autres
  pistes ci-dessous, ou dans le chemin par lots (moins de 200 arêtes, `MERGE`), qui reste à lire.
- **Le témoin** reste une garde du chemin COPY des liens sous des points de reprise. Il ne dit rien de
  la cause. Son atterrissage est à la décision de c7.

## Les deux autres pistes, et ce qui les départagerait

- **Un lot repris après un plantage de l'hôte**, le 13 au soir par exemple. On le départagerait par les
  dates des arêtes en double, comparées à l'heure du plantage. Une arête n'a pas de date : on prendrait
  l'ordre de ses offsets (les sept vagues) comparé aux instantanés du Scope (`_snapshot`, onze pour
  `app.js` entre le 7 et le 13 novembre) et aux journaux de l'hôte.
- **Deux drains du même fichier dans une même session**, le second relisant un « déjà là » faux. On le
  départagerait par le journal d'ingestion du dépôt : deux drains du même fichier entre deux
  ouvertures de l'hôte.

## Le test le plus simple, demandé par c7 : vert sur tous les chemins (14 novembre, 19 h 11)

`tests/e2e_liens_de_morceaux_reverses.rs`, branche `liens-sous-point-de-reprise` (b42dd353b), sous la
lib courante 0c41f9fe6, sans concurrence ni point de reprise forcé. Chaque cas fait une ingestion, trois
réingestions inchangées, puis trois réingestions modifiées, et compte après chaque passe les paires
morceau-parent à plus d'une arête :

| cas | morceaux | chemin des liens | paires doublées |
|---|---|---|---|
| texte de 300 lignes, ligne changée | 299 | COPY, 299 « déjà là » à chaque reversement | 0 |
| texte de 50 lignes, ligne changée | 49 | MERGE | 0 |
| `templates/ide/app/app.js` (86 lignes, 500 Ko), schéma sans versions, `ingest_code`, ligne de tête qui décale tout | 1 466 | COPY, 1 466 « déjà là » | 0 |
| le même, chemin différé (`ingest_code_differe` puis `finir_les_relations_differees`) | 1 466 | COPY, 1 466 « déjà là » | 0 |

Preuves : `~/.cache/rag3weaver-build/validations/validation-pvke9lb1`.

- **`existing_links` et ce qu'on lui passe sont hors de cause** : les uuid de départ sont ceux des
  morceaux, la relation et la direction sont les bonnes (`Scope_Chunk` → `Scope`), et la lecture voit
  toutes les paires à chaque reversement, y compris sur le fichier qui porte 1 140 des doublons.
- **Le code d'aujourd'hui ne produit pas le doublon dans un seul processus.** Restent les deux pistes
  ci-dessus (un lot repris après un plantage, deux drains d'une même session), et une troisième : un
  code plus ancien, au moment où les vagues ont été écrites. Les doublons datent d'avant la migration,
  et les vagues sont sept pour onze instantanés d'`app.js` entre le 7 et le 13 novembre. Le reste de la
  chasse demande l'hôte (journaux, drains concurrents, reprises), pas un test de plus sur ce chemin.

## Ce qu'il faut pour le fermer (réponse d'arbre principal, 14 novembre)

- **Les doublons existants partent avec l'ancien index.** Après 1b, `Scope_Chunk`, `Scope_CHUNKED_FROM`
  et `Lucivy_Scope` restent dans la base pour permettre le retour, mais le produit ne les lit plus. Le
  pas « retrait de l'ancien index », à part, après la bascule des vraies bases et une validation, les
  effacera (environ 5 Go par base). Ce ticket se ferme par ce retrait.
- **La cause se juge par le témoin** `tests/e2e_liens_sous_point_de_reprise.rs` (branche
  `liens-sous-point-de-reprise`). S'il est rouge sous e93710f36 et vert sous 0c41f9fe6, la lib du 14
  novembre a mis fin à la production des doublons, et il n'y a rien à changer dans le code qui écrit
  les liens. Le témoin atterrit alors comme non-régression. S'il reste vert sous e93710f36, l'hypothèse
  ne tient pas, et la cause est à chercher ailleurs avant ce retrait.
