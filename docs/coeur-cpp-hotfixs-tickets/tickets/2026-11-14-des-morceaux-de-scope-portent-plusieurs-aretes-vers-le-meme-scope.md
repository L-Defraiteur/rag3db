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

## Le chemin d'écriture et une hypothèse (non prouvée)

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
