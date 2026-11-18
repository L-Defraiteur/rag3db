# Les outils de référence avant l'index

- **Date** : 18 novembre 2026. Page de conception, écrite par recherche et codeparsers.
- **La question de Lucie** (vers 15 h 40, relayée par c7) : « pour les outils de référence, on ne saurait pas avoir un
  mode temporaire qui fonctionne avant que l'index soit prêt, sans base, juste avec le parsing qui a déjà eu lieu,
  pour toujours le permettre aux agents ? »
- **Le contexte** : c'est le prolongement de son « oui » au balayage de `search_code` avant l'index
  (`docs/tickets/closed/2026-11-14-search-code-avant-l-index-ne-balaie-plus-les-fichiers.md`, branche
  `scan-before-index`).
- **Statut** : docs seules. **Aucun code avant le mot de Lucie.** Les mesures du coût (section 4) viennent après la
  fenêtre de bascule de la lib, sous `poste lourd`.

## 1. Quels outils (recherche)

Les quatre outils de référence lisent le même graphe : le schéma de code de `src/code.rs`.

| Outil | Nœud | Ce qu'il lit |
|---|---|---|
| `outline` | `PlanNode` (`plans/outline.json`) | les `Scope` d'un fichier, leurs lignes et leur signature |
| `usages` | `UsagesNode` | `Symbol` → `DEFINES` / `MENTIONS`, et `CONSUMES`, `INHERITS_FROM`, `IMPLEMENTS` en direct ; `HAS_PARENT` pour les déclarations |
| `callees` | `NeighborhoodNode`, sortant | `CONSUMES` à un, deux ou trois sauts, et `LOCKS` |
| `impact` | `NeighborhoodNode`, entrant | `CONSUMES`, `INHERITS_FROM`, `IMPLEMENTS`, la marque de test de `Scope`, `QUERIES` (par requête) et `LOCKS` |

Les variantes de l'atelier (`atelier_usages`, `atelier_impact`, `navigate_symbols`, `read_scope`…) lisent les mêmes
tables.

**Ce qui est immédiat et ce qui ne l'est pas** :
- `outline` n'a besoin que du parsing d'un fichier ;
- `usages`, `callees` et `impact` ont besoin de la **résolution croisée** entre fichiers ;
- les arêtes `QUERIES` (manifestes) et `LOCKS` (`workspace.locks_via`) se calculent aussi à l'ingestion.

**Le fait qui pèse (codeparsers)** : la résolution croisée n'existe aujourd'hui qu'à l'ingestion. `analyze`
(`code.rs`) ne résout que dans le fichier. Les liens entre fichiers sortent en rendez-vous dans `CodeAnalysis`
(`pending`, avec les types, modules et `self` qui départagent). C'est `materialiser_les_symboles` qui les relie, en
lisant le `Catalog` par une dizaine de lectures (`linked_from_many*`, `parents_et_fichiers`, `base_types_of`…). Les
choix eux-mêmes (`choose_target`, `choose_inherited`, `departager`) sont des fonctions pures.

## 2. La porte et la réponse (recherche)

La même porte que le balayage de `search_code` :
- **On entre dans le repli** quand l'index de la vue lue n'est pas prêt. Soit il est absent (`relations: never`), soit
  la couche de la vue a du retard (le registre `Lag` par cellule et couche).
- **Il faut aussi** qu'aucune `view` explicite ne soit demandée, ou que la vue demandée soit la vue courante du
  backend.
- **Sous une autre vue explicite**, la réponse reste celle d'aujourd'hui : un refus qui dit quoi faire, ou l'état
  d'avant avec le retard dit.

**La réponse le dit**, en tête et sous « ⚠ » comme les autres lignes d'état : « par l'analyse en mémoire, index en
cours : ~N s ». N vient de l'estimation du `Lag` de la couche, et « index absent » remplace N quand rien ne tourne.
Les résultats gardent leurs marques de résolution (`par le nom`, `ambigu`) : le repli ne promet pas plus que l'index.

**Quand le repli s'arrête** : dès que la couche de la vue est à jour (`ViewState::up_to_date`, posé par arbre), les
outils relisent l'index. L'image en mémoire se lâche.

## 3. L'image en mémoire

### Deux formes possibles (recherche)

**A. Un catalogue en mémoire.** Une base rag3db en mémoire (`Rag3dbConnection::in_memory`), remplie par la même
ingestion, sans plein texte ni vecteurs.
- Ce qui ne change pas : les outils, leurs nœuds et leurs rendus ; `materialiser_les_symboles` lit ce catalogue tel
  qu'aujourd'hui. Les sondes de codeparsers font déjà ce geste (un `Catalog` jetable).
- La parité est celle du code : c'est le même chemin.
- Le coût : l'ingestion en mémoire (insertions, rendez-vous, résolution), sans disque, sans COPY sur fichier, sans
  journal.
- « Sans base » au sens de Lucie : rien d'écrit sur disque, rien à migrer, rien qui survive au processus.

**B. Des tables à soi, sans rag3db.** Un « lecteur de symboles » à deux faces : la base, et une face mémoire bâtie
depuis les `CodeAnalysis` de la vue. Les mêmes fonctions de choix tournent dessus (piste de codeparsers, section
suivante).
- Ce qui change : les outils lisent aujourd'hui le graphe par Cypher (`UsagesNode`, `NeighborhoodNode`). Il leur
  faudrait une seconde lecture, sur les tables mémoire, ou une face « graphe » de ces tables.
- Le gain attendu : pas de moteur de base dans la boucle, un pic de mémoire plus bas, une invalidation plus fine.

**Ce que la mesure départage** : si A tient dans un temps court devant l'index entier (section 4), A suffit et ne
duplique rien. B ne vaut son second chemin de lecture que si A coûte trop. À trancher avec les chiffres.

### Le côté codeparsers (codeparsers)

**Ce qui existe déjà, pour A.** Les sondes de codeparsers (`tests/sonde_marque_nom.rs`) font exactement la forme A :
`Rag3dbConnection::in_memory`, `register_code_schema`, puis `ingest_code(&analyze(racine, sources))`, avec un
`HashEmbedder` de dimension 64 à la place du modèle. L'invalidation fichier par fichier existe aussi : c'est le chemin
de la montre, `code_sync::reingest_file` / `reingest_files` / `remove_file`, qui rappelle `ingest_code_jusqu_a` sur le
`CodeAnalysis` du fichier. `materialiser_les_symboles(uuids, lot)` ne relit alors que les symboles du lot : une
mention n'est relue que si sa cible ou son mentionneur est dans le lot (le commentaire « Ne relire que ce qui concerne
le lot », `code.rs`). A n'a donc rien à écrire côté codeparsers pour exister ; il reste à mesurer et à couper ce que
l'ingestion fait en trop pour le repli (les morceaux et leurs vecteurs, le plein texte, s'ils ne sont pas déjà
optionnels).

**B, le lecteur de symboles à deux faces.** `materialiser_les_symboles` lit le `Catalog` par une dizaine d'appels :
`linked_from_many("DEFINES")`, `linked_from_many_with_kind` / `linked_from_many_where("MENTIONS")`,
`resoudre_les_types_differes`, `parents_et_fichiers` (parents, fichiers, genres, origines, défaut de trait),
`base_types_of` (héritage, profondeur `RAG3WEAVER_INHERITANCE_DEPTH`), `noms_des_symboles`. Ces lectures passeraient
derrière un trait, avec la face « base » d'aujourd'hui et une face « mémoire » bâtie depuis les `CodeAnalysis` de la
vue : `DEFINES` et `MENTIONS` par symbole, et les rendez-vous qualifiés (`pending_qualifier_types`,
`pending_import_modules`, `pending_self_types`, `pending_sites`). Le choix ne bouge pas (`choose_target`,
`choose_inherited`, `departager`). La sortie serait une liste d'arêtes `(de, vers, genre, marque, sites)` au lieu
d'appels à `link_jusqu_a`. Les outils en voudraient ensuite une lecture de graphe (section 3, B) : c'est le gros du
coût de B, pas la résolution.

**La parité**, pour l'une et l'autre forme : la sonde de marque compare déjà deux états au couple près (TSV
`fichier, ligne, scope, nom, cible, marque`). Pour B, elle comparerait les arêtes de la face mémoire à celles de la base
sur le même arbre ; vert = zéro couple de différence sur rag3weaver/src et le moteur. Pour A, la parité est celle du
code, mais un témoin reste utile : la même sonde, index réel contre catalogue en mémoire.

**L'invalidation en B.** `analyze` est déjà par fichier. Un fichier écrit : refaire son `CodeAnalysis`, retirer de
la face mémoire ses définisseurs et ses mentions, ajouter les nouveaux, puis rematérialiser les symboles qu'il définit
**ou** mentionne, avant comme après (un nom qu'il ne définit plus change la cible des autres). C'est la règle de lot
de `materialiser_les_symboles`, à recopier, avec le même piège : une définition qui disparaît change les arêtes de
fichiers que l'on n'a pas relus.

**La borne de taille.** La mémoire est à peu près proportionnelle au nombre de scopes et de mentions. La mesure de la
section 4 donnera un coût par scope (octets, millisecondes). La borne se fixera dessus : un nombre de fichiers ou
d'octets de source tel que le repli se bâtisse en quelques secondes et tienne sous quelques centaines de Mio.
Sur l'arbre figé de la sonde, rag3weaver/src compte 290 fichiers et 16 222 scopes ; `src/` du moteur, 1 550 fichiers
et 19 057 scopes.

### Le cycle de vie (recherche)

- **Bâtie** au premier appel d'un outil de référence qui passe la porte. Ou dès l'ouverture d'un backend jamais
  indexé, à décider : un premier appel lent contre une mémoire tenue pour rien.
- **Tenue à jour** par la montre : chaque fichier écrit est réanalysé et remplacé (l'invalidation ci-dessus).
- **Lâchée** quand la couche est à jour.
- **Bornée** : au-delà d'une taille de vue (fichiers ou octets, à fixer par la mesure), pas de repli. La réponse dit
  pourquoi et l'estimation de l'index.

## 4. Le coût mesuré (codeparsers, avec recherche)

_Après la fenêtre, sous `poste lourd`._

- **codeparsers** : trois postes, à partir des chronos d'`ingest_profile`, avec le pic de mémoire relevé par le
  confineur. Ce sont le parsing, la construction des tables et le choix. Les mesures se font sur
  `extension/rag3weaver/src`, puis sur le dépôt entier, puis en ordre de grandeur pour un noyau Linux.
- **recherche** : la sonde `tests/probe_index_timeline.rs` (branche `probe-index-timeline`). Sur une base neuve, le
  gabarit de code livré indexe la racine telle quelle. Toutes les secondes, la sonde relève les changements d'état
  (plein texte, relations, vecteurs) et les nouvelles lignes du journal. Toutes les deux secondes, elle relève la
  taille d'une réponse `usages`. Elle tourne deux fois : sans service d'embarquement (plein texte seul, vecteurs en
  dette), puis avec. Elle donne le temps d'un index complet sur les mêmes arbres, phase par phase (`[sync-profile]`). Je mesure
  aussi le moment où les relations sont lisibles dans l'index réel, avant le plein texte et les vecteurs : c'est la
  durée que le repli doit battre. Et la forme A elle-même : la même ingestion vers un catalogue en mémoire.

| Arbre | Fichiers | Parsing | Tables | Choix | Mémoire | A (catalogue en mémoire) | Relations lisibles dans l'index | Index complet |
|---|---|---|---|---|---|---|---|---|
| `extension/rag3weaver/src` | 311 (17 387 scopes) ; sonde : 290 (16 222) | 7,6 s | 3,5 s | 0,25 s | 960 Mio (analyse) ; 6,0 Gio (A) | 428 s, dont 302 s de dérivations ; symboles 11,3 s | 46 s | 46 s (plein texte seul) |
| `src/` du moteur (sonde) | 1 550 (19 057 scopes) | 2,5 s | 1,5 s | 0,3 s | 685 Mio (analyse) ; 5,0 Gio (A) | 298 s, dont 212 s de dérivations ; symboles 8,7 s | | |
| dépôt entier | 8 585 (82 393 scopes) ; sonde : 3 821 fichiers de code suivis par git (73 324 scopes) | 22,5 s | non mesuré | non mesuré | **8,7 Gio** (analyse seule) | non joué (ne tient pas : environ 6 Gio pour 16 000 scopes) | partiel dès 99 s ; 704 lignes sur 705 à 143 s ; tout à 192 s | 194 s (plein texte seul) |
| noyau Linux (ordre de grandeur) | en-têtes installés (7.3.4) : 8 829 (88 783 scopes, 57 Mo) ; noyau entier : environ 63 000 fichiers, environ 1,1 Go | 5,2 s (en-têtes) ; environ 100 s (noyau entier, au même débit) | | | 1,4 Gio (en-têtes) ; environ 25 Gio (noyau entier, au prorata des octets) | hors de portée | | |

### Ce que dit la chronologie d'un premier index (recherche, 18 novembre 2026)

**Le relevé** : sonde `probe_index_timeline` à luciepc, lib 3d706f709, sans service d'embarquement (plein texte seul,
vecteurs en dette), sur une base neuve. La sonde lit l'état d'index chaque seconde et appelle `usages Catalog`
toutes les deux secondes. Journaux : `~/.cache/rag3weaver-build/probe-index-rw-src.log` et `probe-index-depot.log`.

**`extension/rag3weaver/src`** (311 fichiers, 17 387 scopes) :
- de 0 à 36 s, le plein texte se pose (analyse et ingestion) ;
- de 36 à 46 s, les relations : 269 292 liens en file, 86 525 relations posées ;
- `usages Catalog` rend **0 ligne jusqu'à 46 s**, puis 236 lignes. Pendant tout l'index, l'état ne dit que
  « occupé ».

**Le dépôt entier** (8 585 fichiers indexés, 82 393 scopes, 291 288 relations) :
- le plein texte est posé à 164 s, et les dernières relations à 192 s ;
- les relations arrivent **paquet par paquet**, pas seulement à la fin. `usages Catalog` rend 3 lignes à 99 s,
  704 sur 705 à 143 s (quand les paquets de rag3weaver sont passés), puis 705 à 192 s ;
- un agent qui cherche un nom attend donc entre une et trois minutes selon l'ordre des paquets, sans savoir si la
  réponse est complète.

**Ce que ça dit pour la conception** :
- **la durée à battre** : 46 s sur rag3weaver seul, environ 3 min sur le dépôt. Les vecteurs n'y comptent pas : les
  relations n'attendent pas l'embarquement ;
- **l'index réel répond déjà en partie avant la fin**, sans le dire. La porte du §2 doit couvrir ce cas : les
  relations sont partielles (`relations` pas `ready`, ou la couche a du retard) et pas seulement absentes. La ligne
  « index en cours » vaut aussi pour une réponse qui vient de l'index partiel ;
- un relevé à faire avec la sonde de codeparsers : **A sur le dépôt entier**. Si l'ingestion en mémoire, sans
  plein texte ni morceaux, tient en une dizaine de secondes, le repli bat l'index d'un facteur 10 à 20.

### Ce que disent les mesures de codeparsers (18 novembre 2026)

**Le relevé** : sonde `tests/probe_fallback_cost.rs` (branche `codeparsers/fallback-cost-probe`), lib 3d706f709, une
racine par processus, sous `poste lourd`. Arbre figé `rag3db-garde`. « Parsing » est `analyze` (le parsing et la
résolution dans le fichier). « Tables » est la lecture des `DEFINES`, des `MENTIONS`, des parents et des types
différés dans le catalogue en mémoire ; « Choix » est `symboles · choisir et lier`. « Mémoire » est le `VmHWM` du
processus après l'étape. Journaux : `~/.cache/rag3weaver-build/cout/`.

- **A telle quelle perd contre l'index réel** : 428 s et 6 Gio sur rag3weaver/src, contre 46 s. Un seul
  `ingest_code` de toute la vue paie les dérivations (morceaux, plein texte, leur drain : 302 s), dont le repli n'a
  pas besoin, et l'insertion d'un bloc des fichiers et des scopes (environ 114 s de plus). La résolution elle-même
  ne pèse pas : 11,3 s pour toute l'étape des symboles, dont 3,5 s de lectures et 0,25 s de choix.
- **A allégée n'est pas mesurée** : aucun réglage n'ôte les morceaux sans changer le code. Sa borne, par
  soustraction : environ 126 s sur rag3weaver/src, dont environ 11 s de symboles. Elle ne bat l'index que si
  l'insertion des scopes passe aussi par le chemin rapide de l'index (par paquets), à mesurer une fois écrite.
- **B, sans base** : la borne basse est le parsing plus la résolution, environ 7,6 s + quelques secondes sur
  rag3weaver/src. Elle bat l'index d'un facteur 4 à 5.
- **La mémoire décide de la borne** : `analyze` garde toute la vue. 960 Mio pour 10 Mo de source sur rag3weaver,
  8,7 Gio pour 40 Mo sur le dépôt, 1,4 Gio pour les 57 Mo d'en-têtes C du noyau. Le C coûte peu par octet, le Rust
  et le C++ beaucoup (environ 60 à 120 Kio par scope). Un repli sur le dépôt entier demanderait près de 9 Gio pour
  la seule analyse : il faut soit une borne d'environ quelques centaines de fichiers, soit des tables compactes
  bâties fichier par fichier (relâcher chaque `CodeAnalysis` une fois ses définisseurs et ses mentions retenus).
- **Le noyau entier est hors de portée** : environ 100 s d'analyse au même débit, et environ 25 Gio si la vue est
  gardée entière.

## 5. Ce que le repli n'a pas (recherche)

- **Ni versions ni couches** : une seule image, celle du worktree du backend tel qu'il est sur disque. Aucun rang,
  aucune pile de générations : une `view` explicite n'a pas de repli.
- **Pas de recherche** : ni plein texte, ni dense, ni rerank. `search_code` a son propre repli, le balayage.
- **Pas d'autres dépôts** : une bibliothèque importée ou un autre worktree ne sont résolus que s'ils sont dans l'image.
- **Les arêtes d'ingestion à vérifier** (`QUERIES` des manifestes, `LOCKS` de `workspace.locks_via`, la marque de
  test) : présentes en A, à recâbler en B.
- **Pas de durée** : l'image meurt avec le processus, et un hôte relancé la rebâtit.

## 6. La découpe

- **recherche** : la porte et la ligne de réponse, l'aiguillage des outils vers l'image, le cycle de vie, la borne,
  les témoins de bout en bout (rouge d'abord : un backend neuf, `usages` répond avant l'index et le dit).
- **codeparsers** : les chiffres de la section 4 (une sonde de coût, sous `poste lourd`, après la fenêtre) ; pour A,
  la liste de ce que l'ingestion fait en trop pour le repli et comment le couper ; pour B, si la mesure le choisit,
  le lecteur de symboles à deux faces et sa sonde de parité, rouge d'abord (une face mémoire vide ne rend aucune
  arête là où la base en rend). Aucun code avant le mot de Lucie.

## 7. La lecture (recherche)

### Ce qui est déjà fait, sans repli

Le lot 3 du state par vue (`view-wait-by-state`, recherche, 18 novembre 2026) : pendant un index complet de la
vue, une lecture ne se tait plus. Elle répond tout de suite par `index: {ready: false, indexing: true}` et dit « un
index écrit ta vue, durée inconnue — la réponse porte sur ce qui est déjà indexé ». L'agent sait qu'un `usages`
vide n'est pas une réponse. Le temps d'attente, lui, ne change pas.

### La lecture des chiffres (recherche)

| Forme | rag3weaver/src | Dépôt entier | Second chemin de lecture des outils | Mémoire |
|---|---|---|---|---|
| Index réel, aujourd'hui | 46 s | 192 s (partiel dès 99 s) | non | sur disque |
| A, telle quelle | 428 s | ne tient pas | non | 6 Gio |
| A, allégée (estimée) | ~126 s | ? | non | ? |
| B, borne basse | ~12 s (7,6 s d'analyse et ~4 s de résolution) | ~30 s d'analyse et de résolution | **oui** | 960 Mio ; 8,7 Gio sans tables compactes |

- **A ne bat pas l'index** : elle passe par `ingest_code`, plus lent que le chemin par paquets de l'index réel. A
  n'a de sens que reconstruite sur ce chemin, et alors elle ne gagne que le disque et le plein texte.
- **B bat l'index d'un facteur 4** sur rag3weaver. Elle coûte deux choses : un second chemin de lecture pour
  `usages`, `callees` et `impact` (aujourd'hui des nœuds Cypher génériques), et des tables compactes bâties
  fichier par fichier, sans quoi le dépôt demande près de 9 Gio.
- **Une troisième voie, C** : faire publier les relations plus tôt par l'index réel lui-même. Mesurée ci-dessous.

### C mesurée : ce qui retient les relations dans un premier index (17 h 30)

**Le relevé** : la même sonde, avec `RAG3WEAVER_INGEST_PROFILE=1`, lib 3d706f709, plein texte seul. Journaux :
`~/.cache/rag3weaver-build/probe-index-c-rw-src.log` et `probe-index-c-depot.log` à luciepc.

| Phase (`[sync-profile]`) | rag3weaver/src | Dépôt entier |
|---|---|---|
| analyser (codeparsers) | 13,2 s (dont `parse_project` 8,2 s) | 43,1 s (dont 22,4 s) |
| ingérer les paquets | 19,0 s | 116,0 s |
| — dont **dérivations** (le contenu dérivé, pour la recherche) | **14,9 s** | **95,9 s** |
| — dont symboles (ingestion et rendez-vous) | 2,4 s | 9,5 s |
| — dont COMMIT des paquets | 1,9 s | 12,6 s |
| charger les relations à la fin | 9,9 s (dont `drain · liens` 7,0 s) | 26,8 s |
| choisir et lier (la résolution) | 0,9 s | 5,5 s |
| **`usages` complet** | **42 s** | **196 s** |

**Ce qui retient les relations, ce sont d'abord les dérivations** : 35 % du temps sur rag3weaver, la moitié sur le
dépôt. Elles servent la recherche, pas les outils de référence. Viennent ensuite l'analyse, puis la pose des liens à
la fin. La résolution elle-même est négligeable.

**C par réordonnancement** : poser d'abord fichiers, scopes, symboles et relations, puis les dérivations et le
plein texte. Cela ôte les dérivations du chemin critique. L'estimation par soustraction, à confirmer une fois
écrite :
- rag3weaver/src : ~27 s au lieu de 42 s ;
- dépôt entier : ~95 s au lieu de 196 s.

C'est un gain de 1,5 à 2, **pas la quinzaine de secondes** visée : l'analyse (13 s, 43 s) et la pose des liens
(10 s, 27 s) restent sur le chemin. Pour descendre plus bas, il faudrait aussi :
- accélérer la pose des liens (`drain · liens`, 7 s sur rag3weaver) ;
- recouvrir l'analyse et l'ingestion, paquet par paquet ;
- ou passer par B, qui n'a que l'analyse et la résolution (~12 s sur rag3weaver), avec son second chemin de lecture
  et sa mémoire.

## La décision de Lucie (18 novembre 2026, vers 17 h 45, relayée par c7)

« **C d'abord, puis B quand même.** »

- **C, tout de suite, en lot, rouge d'abord** : l'index pose fichiers, scopes, symboles et relations avant les
  dérivations. Le partage :
  - **recherche** : sortir le drain des dérivations du chemin des paquets (`Catalog::ingest_entities`, qui les draine
    aujourd'hui entité par entité), et le jouer après la pose des relations, dans la passe de `code_sync`. Aussi la
    ligne de journal et d'état qui dit « relations prêtes, dérivations en cours », et le témoin de bout en bout ;
  - **codeparsers** : la part d'ingestion du code (`code.rs`), c'est-à-dire ce qui doit rester dans le paquet (les
    symboles et leurs rendez-vous) et la pose des liens à la fin (`drain · liens`, 7 s sur rag3weaver, 27 s sur le
    dépôt), si elle se raccourcit.
- **B, ensuite** : des tables à soi, bâties par le parsing. Lucie n'a pas tranché le détail ; c7 recommande, et la
  page retient :
  - des **tables compactes**, bâties fichier par fichier, plutôt qu'une borne en nombre de fichiers ;
  - l'image bâtie **au premier appel** d'un outil de référence.
  - Ticket : `docs/recherche/tickets/2026-11-18-les-outils-de-reference-par-des-tables-a-soi-avant-l-index.md`.

## C écrite et mesurée (recherche, 18 novembre 2026, 18 h 50)

**Le lot** : branche `relations-before-derivations`.
- Les témoins rouges : ec77f1d77a, puis a0e9a35d6c et 9ba2da7d79 pour qu'ils compilent.
- Le correctif : 47166b6818.
- Ce qu'il fait : en masse, les dérivées de chaque paquet (`ScopeContent`, le plein texte du code) sont retenues.
  Leur dette part en base tout de suite, et leur rendu se joue après les relations, dans une phase à elle
  (`SyncPhase::Derivations`).
- La passe `passe-c-182905` est VERTE : e2e_code_sync 26, e2e_code 36, e2e_lectures 28, lib 2 628, navigateur
  45/45, 0 non jugé.

**La mesure** : la sonde `probe_index_timeline` sur la branche (`probe-on-c`), mêmes réglages qu'au §7. Journaux :
`probe-index-c2-rw-src.log` et `probe-index-c2-depot.log` à luciepc.

| | Avant C | Avec C |
|---|---|---|
| rag3weaver/src : `usages Catalog` complet | 42 s | **26 s** |
| rag3weaver/src : index entier | 42 s | 43 s |
| dépôt : `usages Catalog` complet (704-708 lignes) | 151 s (relations posées à 196 s) | **89 s** (relations posées à 94 s) |
| dépôt : index entier | 196 s | 169 s |

- Le gain correspond à l'estimation (~27 s, ~95 s). L'index entier ne ralentit pas : il gagne même 27 s sur le dépôt.
- Le journal le dit dans l'ordre : « fichiers et scopes posés · relations : N liens à poser », puis « relations
  posées · plein texte du code : rendu en cours », puis « plein texte prêt ».
- **Ce qui reste sur le chemin** : l'analyse (12,6 s, puis 34,8 s), et la pose des liens (8,8 s, puis 34,4 s, dont
  environ 60 % de MERGE évitables d'après codeparsers, dans son ticket
  `codeparsers/tickets/2026-11-18-la-pose-des-liens-de-la-resolution-repasse-par-merge.md`).

## Questions pour Lucie (posées avant sa décision)

1. ~~Mesurer d'abord C ?~~ **Tranché par c7 : oui.** C est mesurée au §7. Réordonner l'index donne ~27 s sur
   rag3weaver et ~95 s sur le dépôt, pas une quinzaine de secondes. **La question devient** : C seule (un seul
   chemin, un gain de 1,5 à 2), ou C puis B (C d'abord, B pour la première minute sur un gros dépôt) ?
2. **Si B** : une borne en nombre de fichiers (quelques centaines, puis l'index), ou des tables compactes sans
   borne (plus de code, et tout dépôt couvert) ?
3. **Si B** : l'image se bâtit au premier appel d'un outil de référence, qui attend alors une dizaine de secondes,
   ou dès l'ouverture d'un backend jamais indexé, qui tient alors de la mémoire pour rien si personne ne demande ?

**Ma recommandation (recherche)** : C d'abord, sans attendre. C'est un seul chemin, et le gain vaut aussi pour le
plein texte et les vecteurs, qui viennent derrière. Puis mesurer ce que C donne réellement. B ne vient que si
Lucie veut des outils de référence en moins de ~15 s sur tout dépôt. Dans ce cas, des tables compactes et l'image
bâtie au premier appel. codeparsers penche pour B avec des tables compactes, ou une borne de quelques centaines de
fichiers.
