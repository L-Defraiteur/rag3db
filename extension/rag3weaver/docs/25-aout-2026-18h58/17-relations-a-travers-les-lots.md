# 17 — Les relations à travers les lots : résoudre sans dépendre de l'ordre

26 août 2026, 10h. Question de Lucie : *« les relations se reconstruisent
comment quand codeparsers lit un fichier, puis un autre fichier qui le
référence ? Possible ou pas ? Sinon truc qui devrait dans tous les cas
marcher : j'indexe un dossier, puis j'indexe un nouveau fichier du dossier,
les relations doivent se créer. »*

## 1. L'état des lieux, sans fard

**Non, pas aujourd'hui.** `codeparsers` résout **à l'intérieur d'un lot** :
`analyze(root, sources)` appelle `parse_project` avec `resolve_relationships`,
et le résolveur ne connaît que les fichiers qu'on lui donne. Une référence
vers un symbole défini ailleurs n'a pas d'extrémité :

```rust
let (Some(from), Some(to)) = (identity.get(&r.from_uuid), identity.get(&r.to_uuid))
else { analysis.relations_dropped += 1; continue };   // src/code.rs
```

Trois conséquences mesurables :

| Scénario | Ce qui se passe |
|---|---|
| Ingérer A, puis B qui référence A | la référence de B vers A est **perdue** |
| Ingérer un dossier, puis un fichier neuf du dossier | perdue **dans les deux sens** : ses références vers l'existant, et les références de l'existant vers ses définitions |
| `edit` puis `reingest_file` | le fichier est reparsé **seul** : toutes ses relations inter-fichiers tombent |

Et le compteur existe déjà — `relations_dropped` — mais personne n'en fait
rien. **Une perte comptée et non réparée, c'est une dette qui se sait.**

## 2. Pourquoi c'est structurel

Le résolveur est un **passage unique sur un ensemble clos**. Il construit sa
table de symboles à partir des sources reçues, résout, jette le reste. Rien
là-dedans ne survit au lot : une référence non résolue n'est pas *stockée*,
elle est *comptée*.

Or l'ingestion, elle, est **incrémentale par nature** — c'est tout l'intérêt
de `edit`, du `FileSource`, de la mémoire à TTL du
[16](16-le-monde-est-ouvert.md). Le résolveur suppose un monde clos ; le
reste du système suppose un monde qui grandit. C'est là que ça casse.

## 2 bis. Un quatrième cas, trouvé en relisant RAGForge

Notre identité de scope est `blake3(fichier:nom:type:hash_de_signature)`
(`codeparsers/src/relationship_resolution/relationship_resolver.rs:920`),
avec le commentaire *« Same scope = same hash even if line numbers
change »* — bonne propriété : déplacer du code ne casse rien.

**Mais changer une signature change l'identité.** Et `reingest_file`
supprime les scopes disparus. Donc, après un `edit` qui touche une
signature : l'ancien scope est effacé, ses relations **entrantes** —
venues d'autres fichiers, qui ne seront pas reparsés — partent avec lui, et
rien ne les recrée. Le graphe se dégrade **à chaque refactor**, en silence.

C'est exactement ce qui est arrivé à RAGForge (`DETACH DELETE` sur le même
déclencheur). On ne l'avait pas vu parce qu'aucun test ne renomme une
fonction *référencée depuis un autre fichier* — le nôtre (`e2e_code`)
renomme `take_results`, dont tous les appelants sont dans le même fichier.

## 3. Le principe

> **Une référence non résolue est une donnée, pas un échec.**

Si elle est stockée, l'ordre d'arrivée cesse d'avoir de l'importance : la
résolution devient une opération qu'on peut refaire, dans n'importe quel
sens, à n'importe quel moment.

## 4. La couche `Symbol`

Une entité de plus, invisible pour l'agent, avec `hashsafe = ["name"]` —
donc un uuid déterministe, obtenu **sans requête** (`Catalog::entity_uuid`) :

```
Scope ──DEFINES──▶ Symbol{name}          (ce que ce scope offre)
Scope ──MENTIONS─▶ Symbol{name}          (ce qu'il attend, en attente)
```

À chaque ingestion d'un lot, deux passes, toutes deux locales :

1. **Ce que le lot attend** : pour chaque référence non résolue dans le lot,
   `MENTIONS` vers `Symbol(nom)`. Si ce symbole a déjà un `DEFINES`, on
   matérialise `CONSUMES` / `CONSUMED_BY` tout de suite.
2. **Ce que le lot offre** : pour chaque scope défini, `DEFINES` vers
   `Symbol(nom)`, puis on relit les `MENTIONS` entrants de ce symbole — ce
   sont les scopes qui attendaient, parfois ingérés il y a des jours — et on
   matérialise.

Le cas de Lucie tombe alors tout seul, **dans les deux sens** : le fichier
neuf trouve ce qui existait (passe 1), et l'existant trouve ce que le
fichier neuf apporte (passe 2). Aucune ré-analyse du dossier.

**On garde `CONSUMES` matérialisé** plutôt que de le dériver en deux sauts à
chaque recherche : `search_expand` traverse déjà cette relation, et une
requête d'agent ne doit pas payer la dette d'ingestion.

## 5. L'ambiguïté, qui est le vrai risque

Un nom global n'est pas une identité : `len`, `new`, `execute` sont définis
cent fois. Une résolution par nom, seule, **sur-relierait** massivement — et
on a déjà payé ce prix une fois (47 relations par scope avant les options du
résolveur, doc 04 du 25 août).

Trois garde-fous, dans cet ordre :

1. **Le résolveur du lot reste la voie précise.** Il connaît les imports, la
   portée, le fichier. La couche `Symbol` ne sert qu'à ce qu'il a dû
   abandonner — les références **inter-lots**.
2. **Un seul définisseur, sinon rien.** Si `Symbol(nom)` a plusieurs
   `DEFINES`, on ne matérialise pas : on laisse le `MENTIONS`, et on
   compte. Mieux vaut une relation manquante qu'une relation fausse — un
   agent qui suit `CONSUMED_BY` doit pouvoir croire ce qu'il lit.
3. **Le chemin d'import quand il existe.** Une référence qui vient d'un
   `use a::b::c` porte son chemin ; il départage les homonymes. C'est
   l'affinage qui viendra après, pas le premier jet.

## 6. L'oracle : la ré-résolution complète

Pour savoir si l'incrémental est juste, il faut un témoin : **ré-analyser
tout le projet en un lot** donne le graphe de référence. Sur notre propre
module : 27 fichiers, 1 373 scopes, 12 418 relations, 1,2 s de parsing.

C'est aussi le **repli** honnête : une commande explicite (« reconstruis
les relations ») pour les cas où l'incrémental a dérivé, et le mode par
défaut pour une première ingestion.

Le test qui tranche, et c'est celui que Lucie a posé :

> Ingérer le dossier en un lot, noter les relations. Recommencer en ingérant
> les fichiers **un par un, dans un ordre quelconque**. Les deux graphes
> doivent être **identiques**.

S'ils le sont, l'ordre n'a plus d'importance — ce qui est exactement la
propriété qu'on cherche.

## 6 bis. Ce que RAGForge a payé pour nous

RAGForge — le moteur Neo4j précédent, avec la version **TypeScript** de
codeparsers — a buté sur exactement ces questions. Enquête faite dans son
code ; voici ce qui s'y lit, et ce qu'on en retient.

**Le résolveur TS était intra-lot, explicitement** : *« resolves cross-file
relationships without needing a database »*, et ses tables sont vidées à
chaque appel (`RelationshipResolver.ts:256`). Même conception que la nôtre,
même limite. La perte se lit dans un `continue` commenté *« might be from a
different file set »* — silencieuse, comme notre `relations_dropped`.

**Le rattrapage inter-ingestions existait, mais par chemin de fichier** :
une arête `PENDING_IMPORT` stockée en base, rejouée après coup. Deux
conséquences :

- ça ne marche que si le fichier contient un `import` **textuel** dont le
  chemin résout sur disque. Une référence par simple nom — le cas courant
  en Rust dans un même crate — est perdue définitivement ;
- **le rendez-vous se faisait par chemin, jamais par symbole.** C'est la
  différence de fond avec la couche `Symbol` proposée ici, et c'est ce qui
  a plafonné leur incrémental.

Le mécanisme avait fini en **quatre conventions incompatibles**, dont deux
mortes et une requête qui ne correspondait à aucune. Leçon : *une seule
convention pour l'arête en attente*, ou le rattrapage devient lui-même une
source de bugs invisibles.

**L'asymétrie qu'on vient de retrouver chez nous** y était aussi : les
relations sortantes d'un fichier modifié étaient supprimées puis
reconstruites depuis le lot seul (donc appauvries), et **les entrantes
n'étaient jamais réévaluées**. Il n'existait aucune notion de *dépendants
inverses* — zéro occurrence de `relink`, `affected files`, `reverse
dependency` dans tout le moteur.

**La sur-connexion était réelle et documentée.** Leur repli « nom seul,
inter-fichiers, prends le premier » a produit des garde-fous empilés au fil
du temps, dont un commentaire qui est notre cas mot pour mot : *« sinon
c'est probablement un appel de méthode sur variable locale ou un builtin
(ex : `map.get()`) »*. Un **score de confiance** sur l'arête était prévu au
design (1,0 fichier résolu, 0,8 candidat unique, 0,5 heuristique de nom) et
n'a **jamais été implémenté** ; il n'en reste qu'un booléen. Pire, une de
leurs requêtes de rattrapage fait un **produit cartésien** quand le scope
source est inconnu — *tous* les scopes du fichier importateur × *tous* les
scopes cibles portant le nom — et c'est la branche toujours prise.

**Le chiffre qui justifie tout ce document** : avant leurs nœuds
fantômes, **27 % des relations échouaient silencieusement** — 12 773 sur
46 379. C'est l'ordre de grandeur de ce qu'une résolution intra-lot laisse
sur la table quand l'ingestion est incrémentale.

### Ce que ça change dans le dessin ci-dessus

1. **Le rendez-vous est le symbole, pas le chemin** — confirmé par
   l'échec de leur approche, et c'est déjà notre §4.
2. **Les dépendants inverses deviennent une notion de premier rang.** Une
   ré-ingestion doit remettre en file les scopes qui *mentionnent* ce que
   le fichier définissait. Avec la couche `Symbol` c'est gratuit : les
   `MENTIONS` entrants du symbole *sont* la liste des dépendants.
3. **Ne jamais éventer quand la source est imprécise.** Si l'origine d'une
   référence n'est pas connue au scope près, on rattache au fichier ou on
   s'abstient — jamais un produit cartésien.
4. **Un score de confiance sur l'arête**, en plus de la règle du
   définisseur unique : `1.0` résolu dans le lot, `0.8` définisseur unique
   trouvé par symbole, et rien en dessous. Leur design l'avait vu ; ne pas
   le construire est ce qui a laissé l'ambiguïté sans recours.
5. **Rendre l'incomplétude observable** : leur compteur de `PENDING`
   restants était utile. Nos `MENTIONS` non résolus jouent ce rôle, et
   remplacent `relations_dropped`.
6. **L'identité par chemin absolu** leur a évité les doublons mais rendu le
   graphe non portable, avec une migration Cypher à chaque rattachement.
   Cela confirme le choix d'URI du [15](15-identite-d-un-fichier.md) : le
   schéma de l'URI est à la source de le choisir, `git://…@commit` pour ce
   qui doit voyager.

## 6 ter. Ce qui est fait, et ce que ça a coûté

Étapes 1 à 3 faites le 26 au matin.

- `analyze` rend désormais `pending: Vec<(clé de scope, nom)>` — ce que le
  lot attend et n'a pas trouvé. Filtres : on écarte les `Builtin` et les
  `LocalScope` (le résolveur du lot reste la voie précise), les noms
  **définis dans le lot** (s'ils n'ont pas été reliés, c'est une décision du
  résolveur, pas un manque) et les bibliothèques externes connues.
- Entité `Symbol` (`hashsafe = ["name"]`, donc uuid sans requête) avec
  `DEFINES` et `MENTIONS`. Elles sont exposées à l'agent comme les autres :
  « qui mentionne `merge_port_values` ? » est une vraie question.
- Matérialisation dans les deux sens à chaque lot, **en deux requêtes**
  (`UNWIND` sur tous les symboles du lot). Une requête par symbole coûtait
  2,5 fois le temps d'ingestion — mesuré, puis corrigé.
- Règle du définisseur unique : plusieurs `DEFINES` ⇒ on s'abstient et on
  compte (`ambiguous`).

**Le test qui tranche passe.** Deux fichiers, dont l'un référence l'autre,
ingérés ensemble puis dans les deux ordres, un par un : les trois graphes
sont identiques — **y compris « usage d'abord »**, le sens que le résolveur
intra-lot ne peut structurellement pas voir.

**Ce que ça coûte, mesuré sur `src/dataflow`** (27 fichiers, 1 387 scopes,
12 498 relations) :

| | |
|---|---|
| Symboles créés | 3 275 |
| Dont sans définisseur dans le projet | 2 765 (`Vec`, `Some`, types de la bibliothèque standard…) |
| Noms ambigus, écartés | 181 (`new`, `execute`… — exactement la sur-connexion évitée) |
| Ingestion | 17,6 s → 29,8 s |

**Et la question « qu'est-ce qui pèche ? » a une réponse chiffrée**, parce
que le rapport porte désormais le temps par phase :

| Phase | Premier passage | Second passage (identique) |
|---|---|---|
| Entités (27 fichiers, 1 387 scopes) | **14,7 s** | **24,4 s** |
| Relations (12 498) | 0,67 s | 0,69 s |
| Symboles (3 275 + ~13 k liens) | **12,1 s** | 19,7 s |

Trois faits, et aucun n'est celui qu'on aurait deviné :

1. **Ce ne sont ni les embeddings ni les relations.** `e2e_code` tourne sur
   `HashEmbedder` — aucun poids, aucun GPU — et 12 498 relations coûtent
   moins d'une seconde, soit **vingt fois moins par élément** qu'une entité.
   Le coût est le **pipeline d'entité simple** : ~10 ms pièce, pour
   l'insertion, le découpage, les chunks et l'index plein texte.
2. **Un `Symbol` paie ce pipeline complet pour un nom de vingt
   caractères.** Le rendre « titre seul » a été tenté : le catalogue le
   refuse — *« Entity 'Symbol' has no content fields »*. Toute entité est
   cherchable, donc toute entité est découpée et indexée. C'est cohérent,
   et c'est cher pour un nœud d'identité pure.
3. **Ré-ingérer un contenu identique coûte 60 % de plus que l'ingérer.**
   14,7 s → 24,4 s sur exactement les mêmes fichiers. Le `content_hash` est
   là, il sert à la péremption de `read`, mais **rien ne court-circuite une
   entité inchangée**. C'est le défaut le plus rentable des trois.

Trois leviers, par valeur décroissante :

- **court-circuiter l'inchangé** : profite à toute ré-ingestion, pas
  seulement au code. **Tenté le 26 au matin, mesuré, puis retiré** — voir
  ci-dessous : le gain est énorme et la garde était fausse ;
- **grouper les insertions** : ~5 ms par enregistrement sent l'aller-retour
  unitaire ;
- **une entité sans chunks** — ~~un troisième genre~~ **fait le 26 au
  matin**, et bien moins cher que prévu : voir ci-dessous.

### `chunked: Some(false)` — étudié avant d'être fait

L'étude a renversé l'hypothèse de départ. **Le plein texte ne dépend pas
des chunks** : son index vit sur la table **parente**
(`open_fts_handles_for` passe `target.parent_table`, et `InsertRecordNode`
porte le commentaire « Absent pour les tables de chunks — l'index vit sur
la table parente »). Une entité sans chunk reste donc cherchable en BM25,
sans rien changer.

Le verrou est **le vecteur** : la colonne `embedding` n'existe que sur la
table de chunks, et la table parente d'une entité simple n'a même pas de
colonne où en loger un. Le sparse a le même symptôme (handle ouvert sous le
nom de la table de chunks) mais un remède à une ligne, puisqu'il indexe par
offset de nœud et non par colonne.

D'où la règle retenue, qui **contourne le verrou au lieu de l'affronter** :

> Les chunks restent obligatoires pour tout ce qui a un signal vectoriel ou
> sparse. Ils deviennent facultatifs pour une entité BM25 seule.

`EntityConfig.chunked: Option<bool>`, et `validate()` **refuse**
`Some(false)` avec un signal vecteur ou sparse — mieux vaut une erreur de
configuration qu'une entité silencieusement introuvable. `Symbol` devient
`signals: BM25, chunked: Some(false)`.

**Ce qu'on découvre en passant** : `symbol_config()` héritait du défaut
`HYBRID`, donc on calculait et stockait **un embedding pour chacun des
3 275 symboles** — des noms nus. Personne ne l'avait voulu.

**Mesure** (`src/dataflow`, mêmes 27 fichiers) :

| Phase | Avant | Après |
|---|---|---|
| Symboles | 12 121 ms | **1 706 ms** |
| Ingestion totale | ~28 s | **~17,5 s** |

Soit sept fois moins, et la couche de rendez-vous du doc 17 ne coûte
presque plus rien. Le test qui garde la propriété : un `Symbol` a zéro
chunk **et** `search(target = "Symbol", query = "compute_total")` le
trouve. Le chemin « pas de chunk » était d'ailleurs déjà écrit et testé
dans la recherche (`ChunkAttributionMiss::NoChunks`, branche « No chunk
intersection ») — il n'était simplement jamais emprunté.

Les 2 765 symboles sans définisseur sont le prix du rendez-vous : ils ne
servent que si une ingestion future définit ce nom. Pour un dépôt qui ne
définira jamais `Vec`, c'est du poids mort — d'où le levier suivant, à
faire quand il gênera : **un graphe d'entretien qui élague les `MENTIONS`
vers des symboles sans définisseur au-delà d'un certain âge**, exactement
comme les racines à TTL du [16](16-le-monde-est-ouvert.md). Le compter
d'abord (`still_pending`), le couper ensuite.

### Le court-circuit : ce qu'il rapporte, et pourquoi il a été retiré

Essai : dans `InsertRecordNode`, comparer le `_content_hash` stocké et ne
pas faire descendre les enregistrements inchangés dans le reste du graphe.

**Le gain est massif** — deuxième ingestion de `src/dataflow`, contenu
identique :

| | avant | après |
|---|---|---|
| entités | 24 385 ms | **196 ms** |
| symboles | 19 667 ms | **1 270 ms** |
| ré-ingestion complète | ~44 s | **~2 s** |

**Et la garde était fausse.** La passe complète a rendu deux échecs, tous
deux instructifs :

1. `e2e_code::edit_reingests…` — après un `edit`, le fichier restait marqué
   **périmé**. Parce que `_content_hash` est calculé sur les *champs de
   contenu*, et le champ de contenu de `File` est… `path`. Le texte du
   fichier vit dans `content_hash`, un champ **utilisateur**. Un
   enregistrement peut donc changer sans que son `_content_hash` bouge :
   la condition testait la mauvaise chose.
2. `e2e_undo::undo_delete…` — après une restauration, l'entité parente
   revient **sans ses chunks**. Le hash correspond, on saute, et les chunks
   ne reviennent jamais. Un hash identique prouve que le *contenu* est le
   même ; il ne prouve pas que les **artefacts dérivés existent**.

La règle correcte, pour la prochaine fois :

> Sauter le travail dérivé **si et seulement si** le contenu est identique
> **et** que les artefacts (chunks, index, embeddings) existent déjà.

Et la place correcte n'est pas `InsertRecordNode` — qui ne connaît ni la
configuration de l'entité ni sa table de chunks — mais
`Catalog::ingest_entities`, qui a les deux et qui construit les
enregistrements. L'écriture de la ligne, elle, doit rester : elle est
bon marché, et c'est elle qui rafraîchit les champs hors contenu.

## 7. L'ordre, et ce que ça coûte

1. **Stocker les références non résolues** (`Symbol`, `DEFINES`,
   `MENTIONS`) et les compter par nom plutôt que globalement. *Test* :
   `relations_dropped` tombe à zéro, remplacé par des `MENTIONS` en attente.
2. **Les deux passes de matérialisation**, avec la règle du définisseur
   unique. *Test* : A puis B, et B puis A, donnent le même graphe.
3. **Le test d'équivalence** avec la ré-résolution complète, sur notre
   propre module. *Test* : fichier par fichier = tout d'un coup.
4. **(reste à faire) `reingest_file` passe par là**, et **réévalue les entrantes** via les
   `MENTIONS` du symbole : après un `edit` qui change une signature, les
   relations venues d'ailleurs se refont au lieu de disparaître (§2 bis).
   *Test* : renommer une fonction **référencée depuis un autre fichier**,
   et vérifier que `CONSUMED_BY` pointe le nouveau scope — le test qui
   manquait, et qui aurait montré le défaut.
5. **Le chemin d'import** pour départager les homonymes, si la mesure de
   l'étape 3 montre qu'il en manque trop.

Coût à l'ingestion : deux liens de plus par référence et par définition,
et une lecture de voisins par symbole défini. À l'échelle de notre module,
quelques milliers d'arêtes de plus — invisible à côté des 12 418 déjà là.
Coût à la recherche : **zéro**, puisque `CONSUMES` reste matérialisé.

---

## 8. Fait, le 26 août au soir : le court-circuit, version correcte

La règle du §6 est écrite, à sa place — `Catalog::split_unchanged`, appelée
par `ingest_entities`, qui connaît la configuration de l'entité et sa table
de chunks. Un enregistrement ne redescend pas dans le graphe **si et
seulement si** :

1. **sa ligne existe et tous ses champs y sont identiques** — les champs
   déclarés, `_content_hash`, et les deux colonnes de cellule. Comparer le
   seul `_content_hash` était le premier échec : il ne porte que sur les
   champs de *contenu*, et le champ de contenu de `File` est `path` ;
2. **ses artefacts dérivés existent** — au moins un chunk, et *tous*
   embarqués si l'entité a un signal vectoriel. C'était le second échec :
   après une annulation, le hash correspond et les chunks ne sont plus là.

Une entité déclarée sans chunk (`chunked: Some(false)`, comme `Symbol`) ne
demande évidemment que la première condition.

Deux requêtes en tout, par `batch_select` en `UNWIND` : la ligne parente,
et le compte de chunks par parent. En cas de doute — requête en échec,
entité participant à une base de connaissance — **tout le lot repart au
chemin complet** : refaire coûte du temps, sauter à tort donne un index
faux.

Mesure, deuxième ingestion de `src/dataflow`, contenu identique
(`reingest_is_idempotent`, qui ingère deux fois et compare) :

| phase | avant | après |
|---|---|---|
| entités | 16 892 ms | **101 ms** |
| symboles | 1 922 ms | 1 268 ms |
| relations | 729 ms | 701 ms |
| **ré-ingestion complète** | **~19,5 s** | **~2,1 s** |

Les 1 268 ms de symboles qui restent ne sont pas des entités : ce sont les
deux passes `UNWIND` de matérialisation, qui refont leur travail parce que
le graphe, lui, a pu changer.

Et le compte rendu ne bouge pas : ces enregistrements *sont* ingérés, ils
l'étaient déjà. `RAG3WEAVER_INGEST_PROFILE=1` dit combien ont été sautés,
par entité.
