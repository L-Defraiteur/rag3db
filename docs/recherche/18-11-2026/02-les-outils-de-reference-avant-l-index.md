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
| `extension/rag3weaver/src` | | | | | | | | |
| dépôt entier | | | | | | | | |
| noyau Linux (ordre de grandeur) | | | | | | | | |

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

## Questions pour Lucie

_Après la mesure : A ou B, la borne de taille, et le moment où l'image se bâtit (au premier appel ou à
l'ouverture)._
