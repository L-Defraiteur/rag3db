# Le plan de 2b-B : le chargeur durable des couches

14 novembre 2026, everything declarative, à la demande de c7 ; à relire par arbre principal. Docs
seules : aucun code avant cette relecture, et les points du § 7 attendent le mot de Lucie. Le code est
lu sur origin/master 9bebb3a37, relativement à `extension/rag3weaver/` ; rien n'a été compilé ni
joué pour l'écrire.

Il s'appuie sur trois pages :
- `docs/arbre-principal/13-11-2026/28-plan-visibilite-couches-2b.md` (Codex : 2b-A, 2b-B, 2b-C et
  treize contre-exemples) ;
- `docs/arbre-principal/13-11-2026/29-contrat-de-la-version-de-scope.md` (1a) ;
- `docs/arbre-principal/13-11-2026/30-lier-branches-et-worktrees-aux-couches.md` (la liaison, et
  les décisions de Lucie du 13 novembre).

**Ce que 2b-B doit rendre possible** : ouvrir une génération d'une couche, et chercher **dans** elle
par les trois signaux, sous un seul instantané, le masque appliqué avant le top-k. 2b-C branchera
ensuite le paramètre `view` des outils.

## 1. Ce qui existe déjà, et ce qui manque

**Déjà là, et réutilisé tel quel :**
- **La version de scope (1a)** : `ScopeContent` dérivé de `Scope` par contenu (`code-scope-v1`).
  La relation `ScopeContent_DERIVED_FROM` porte l'état de rendu par occurrence (`_render_hash`, sur
  l'arête). Les morceaux et les vecteurs sont sur `ScopeContent_Chunk` (`code.rs:650-771`,
  `schema.rs:379-407`). Une base sans versions se dit « à migrer » (`code.rs:676-688`,
  `backend.rs:5458`).
- **Le masque d'occurrences** (`dataflow/generic_search_nodes.rs:2980-3005`,
  `occurrence_mask_options`) :
  - il prend les occurrences admises (`occurrence_roots_for`), lit leurs versions par la relation
    canonique et remplace le filtre par `_uuid IN [versions]` ;
  - il sert les trois signaux : dense (872), BM25 lucivy (1324), creux (1748) ;
  - la projection finale ramène chaque version à ses occurrences admises (`finish_signal`).
- **Le masque avant le top-k, par construction** :
  - dense : le filtre devient le graphe projeté (`PROJECT_GRAPH_CYPHER`), sur lequel
    `QUERY_VECTOR_INDEX` cherche (`rag3db_search_backend.rs:78-121`) ;
  - plein texte : `allowed_ids` de lucivy, par segment (`gsn.rs:1463, 3059-3113` ;
    `fts_handle.rs:154-164`) ;
  - creux : les offsets des morceaux (`gsn.rs:1759, 3071-3079`) ;
  - le BM25 natif de rag3db refuse une cible projetée (`gsn.rs:1384-1386`).
- **La lecture sous instantané (L6)** : `BEGIN TRANSACTION READ ONLY` par appel (`read_pool.rs`).
  L'index dense suit l'instantané de son lecteur ; le cœur l'a prouvé
  (`VectorSearchUnderSnapshot`, d5aa49c49).
- **La liaison branche → couche** (0672d624f, `couches.rs`) : les branches suivies, la parente
  estampillée et sa base recalculée, l'état `layers` dans `index_states`.
- **Le résolveur pur 2a** (`dataflow/migrations/publication/layers.rs`) : il compose une pile de
  révisions épinglées, applique les tombstones et rend chaque occurrence visible avec sa provenance.
  Il n'est branché à rien.

**Ce qui manque, et que 2b-B apporte :**
1. aucune **génération** n'est persistée, ni (couche, commit), ni ses occurrences, ni ses
   tombstones (`couches.rs:250-251` le renvoie à un autre lot) ;
2. le masque ne connaît que la cellule et le filtre : `occurrence_roots_for` admet toutes les
   occurrences de la cellule, pas celles d'une génération ;
3. les poignées lucivy et creuses ne sont pas épinglées avec la base : L6 ne fige que la base ;
4. L6 se dégrade sans refuser (pool plein, plafond dépassé : la suite lit hors instantané,
   `read_pool.rs:244-262`) ;
5. un backend dense qui n'honore pas le filtre ne donne qu'un avertissement (`search.rs:1074-1080`) ;
6. le graphe (usages, callees, impact) filtre **après** le parcours (`usage_nodes.rs:143-164`,
   `neighborhood_nodes.rs:472-480`) ;
7. lucivy est en 4.3.5 (`Cargo.toml:115-117`) : les statistiques BM25 de la vue (N et df comptés
   sur la vue) ne sont justes qu'à partir de 4.3.7, publiée le 14 novembre selon c7.

## 2. Ce qu'une couche charge à l'ouverture : les versions neuves seulement

- **Ce qui s'indexe.** On part de la génération de la parente qui contient la base. On prend les
  chemins qui diffèrent de la base (`git diff --name-only <base>..<pointe>`), et on les analyse à la
  pointe, par `git show`, sans checkout. Chaque scope donne l'empreinte de son rendu (1a).
  - Une empreinte connue rattache l'occurrence à la version existante : aucun embarquement.
  - Une empreinte neuve crée sa version, embarquée une fois.
  - Ce sont les 12 à 151 versions par branche de la mesure du 8 novembre, au lieu de 88 à 1 278
    scopes.
- **Les occurrences d'une couche** sont des lignes `Scope` propres à la couche : sa clé stable
  **plus** la génération qui l'a posée. Elles portent les lignes et le fichier de la pointe, et leurs
  arêtes de code, calculées par l'analyse de ces fichiers. Une couche ne copie rien de sa parente.
- **Les tombstones** : un scope ou un fichier présent à la base et absent à la pointe s'écrit comme
  retiré dans la génération (occurrence ou fichier, les deux genres de 2a).
- **La génération se publie** à la fin de l'index de ce commit (décision 2 : à chaque commit indexé),
  dans une seule transaction. Son identité (page 30, § 2) : le commit, la couche et le commit de base,
  l'empreinte de la recette. Réindexer avec la même recette ne crée rien ; une autre recette crée une
  autre génération.
- **Le registre** adopte celui de Codex (chaîne de publications, refus d'une tête qui a bougé), avec
  des identifiants déterministes à la place des tirages.
- **Le rebase suit** (décision 4) : la base recalculée donne une génération neuve, sur une autre
  génération de la parente.

## 3. Ouvrir une génération : du stockage à l'ensemble visible

- **Le chargeur** lit, sous l'instantané de l'appel, la chaîne des générations : celle de la vue, sa
  parente épinglée, et ainsi de suite jusqu'à l'ancre. Il en fait les `LayerRevision` de 2a :
  occurrences, occurrences retirées, fichiers retirés. `PinnedLayers::resolve` rend l'ensemble des
  occurrences visibles, chacune avec sa provenance.
- **Le point de composition** est `occurrence_roots_for`. Sous une vue, il rend cet ensemble, et non
  plus toutes les occurrences de la cellule. Tout le reste en découle sans changement :
  - le masque des versions, par la relation canonique ;
  - le filtre `_uuid IN` du dense ;
  - `allowed_ids` du plein texte ;
  - les offsets des morceaux du creux ;
  - la projection finale vers les occurrences visibles.

  C'est le 2b-A de Codex (« admission commune »), qui devient ici la première pièce de 2b-B.
- **Sans vue**, rien ne change : la vue implicite est l'ancre, c'est-à-dire la cellule
  d'aujourd'hui. Une base sans versions refuse toute vue et le dit (« à migrer »).
- **Le coût** : une génération se résout une fois par instantané. Elle se mémorise par (génération,
  instantané), jamais au-delà d'un appel dans ce lot.

## 4. Les trois signaux sous un seul instantané

- **Le graphe de versions et le dense.** Le masque devient le graphe projeté, puis le top-k (§ 1).
  Le dense lit sous la transaction du lecteur. Un vecteur validé après le début de la lecture n'est
  jamais rendu.
  - **Réserve** (page 30) : le graphe HNSW n'est pas versionné. Le résultat est exact pour
    l'instantané, mais son rappel n'est pas celui d'un index figé. Quand le rappel compte, il faut un
    repli exhaustif sous le masque (§ 7).
- **Le plein texte.** Un seul searcher lucivy pour toute la lecture, pris au début, jamais rechargé
  au milieu ; puis `allowed_ids` par segment, avant le top-k.
  - Le classement suit la vue à partir de lucivy 4.3.7 (N et df de la vue) : le lot monte la
    dépendance à 4.3.7, et son premier témoin est celui des statistiques de la vue.
- **Le creux.** Mêmes règles, sur les offsets des morceaux.
- **L'instantané composé** (le prérequis 3 de Codex) :
  - la base est figée par L6 ;
  - les poignées lucivy et creuses sont prises au début de l'appel et tenues jusqu'à la fin ;
  - **aucun document qu'une génération lisible voit n'est effacé physiquement pendant une lecture**
    (le ramasse-miettes attend les lectures sous un bail, M5) ;
  - une version écrite après le début de la lecture n'a pas d'occurrence dans l'instantané, et tombe
    à la projection.
- **Aucun demi-instantané** (prérequis 4) : sous une vue, un pool plein, un plafond de durée dépassé
  ou une poignée d'une autre génération font **refuser** la réponse, en le disant. On ne lit pas hors
  instantané comme aujourd'hui. Sans vue, le comportement d'aujourd'hui reste.
- **Un signal qui ne sait pas masquer refuse** sous une vue : le dense dont `honore_le_filtre()` est
  faux, le BM25 natif (déjà refusé), un mode scan sans admission. Ce n'est plus un avertissement,
  même sous `strict_signals=false`.

## 5. Les relations : les deux bouts visibles

- Sous une vue, `usages`, `callees` et `impact` ne parcourent que des occurrences visibles. Une arête
  dont un bout est invisible n'existe pas pour la vue. Aujourd'hui, le parcours passe par tous les
  nœuds et filtre après (§ 1) : ce changement de parcours est le plus gros morceau du lot.
- Les arêtes d'une couche sont celles de ses occurrences. Une occurrence héritée garde les arêtes
  de la parente, sauf celles qui visent une occurrence retirée par la couche.

## 6. Les témoins, rouges d'abord

Les contre-exemples de Codex (page 28, 1 à 13), sur un petit dépôt à trois couches (ancre, parente,
fille) :
1. **f de la parente, g de la fille, h de l'ancre** ; un tombstone de h dans la parente la masque
   aussi dans la fille (la conception du 8 novembre).
2. **Le masque avant le top-k** : k = 2 face à vingt candidats invisibles mieux classés, dans chacun
   des trois signaux ; aucune fuite, aucun résultat visible perdu.
3. **Les statistiques BM25 de la vue** : le score d'un terme sous la vue est celui d'un index qui ne
   contiendrait que la vue (lucivy 4.3.7).
4. **Une version partagée** par deux occurrences, l'une masquée : la visible seule répond.
5. **Un filtre utilisateur** qui vise une occurrence retirée ne la rend pas visible.
6. **Les relations** A→B et C→A avec A invisible : aucune n'apparaît.
7. **Le témoin cruel** : une génération se publie pendant une lecture tenue, dans les trois signaux à
   la fois. Aucune réponse ne mêle deux générations : chaque résultat est une occurrence de la
   génération lue, et l'ensemble des résultats est celui d'une seule génération.
8. **Aucun demi-instantané** : pool plein, ouverture refusée, plafond dépassé, poignée d'une autre
   génération. Chaque cas est un refus nommé sous une vue, et la lecture d'aujourd'hui sans vue.
9. **Un backend qui ne sait pas masquer** refuse sous une vue.
10. **Une vue inconnue ou ambiguë** est refusée avant toute recherche.
11. **Un offset ou une empreinte identique** dans deux générations ou deux modèles ne se confond pas.
12. **Les versions neuves seulement** : ouvrir une couche qui change trois fichiers appelle
    l'embarqueur exactement autant de fois qu'il y a de versions neuves (un embarqueur compteur).

Puis la passe complète et les e2e du changement : `e2e_code`, `e2e_code_sync`, `e2e_scope_versions`,
`e2e_lectures` (lecture et instantané).

## 7. Ce qui ne se code pas sans le mot de Lucie

1. **La forme des occurrences d'une couche** dans le gabarit de code : des lignes `Scope` marquées de
   leur génération (§ 2), ou une entité à part. C'est un changement de format du gabarit, qui
   s'estampille comme la forme de 1a. Il ne touche aucune base existante avant 1b et son mot.
2. **Refuser une lecture sans instantané sous une vue** (§ 4) : sous charge, une vue répondra
   « réessayer » au lieu d'une réponse peut-être mêlée.
3. **Le parcours des relations restreint aux occurrences visibles** (§ 5) : sous une vue, `usages` et
   `impact` cessent de voir ce que la vue masque.
4. **Le repli exhaustif sous le masque** quand le rappel du dense compte (§ 4) : à mesurer avant
   d'en faire une règle (le coût d'un parcours de la vue entière, pour une vue de quelle taille).
5. **La rétention** : M5 et le bail qui protège les lectures épinglées (page 30, la règle proposée
   par c7, à trancher).

## 7 bis. Les décisions de Lucie (14 novembre 2026, relayées par c7)

Lucie a dit oui aux cinq points du § 7 : « au pire ce sont des décisions qui peuvent se retravailler
après pour proposer amélioration ou flexibilité ; partons dessus jusqu'à ce qu'on se rende compte d'un
truc emmerdant pour l'user ».

1. **Une entité à part pour la génération.** Une génération est un **ensemble d'identifiants**, et
   non une propriété d'un scope. C'est une entité `Generation` (identité du § 2), qui cite ses
   occurrences, ses occurrences retirées et ses fichiers retirés, et sa génération parente épinglée.
   - **Les occurrences elles-mêmes restent des lignes `Scope`** (la relecture d'arbre principal) :
     les relations de code, `ScopeContent_DERIVED_FROM` et les morceaux restent ceux d'aujourd'hui,
     sans second schéma de code.
   - Leur `_uuid` devient **hash(clé stable, génération qui l'a posée)**, et la clé stable une
     colonne à part. Sinon, deux générations d'un même scope se percutent sur `_uuid`. Les lignes de
     l'ancre reçoivent la génération de l'ancre.
   - C'est un format estampillé, comme 1a. Aucune base existante n'est touchée avant 1b, qui doit le
     connaître.
   - Ce point se lit ainsi (génération à part, occurrences en lignes `Scope`) ; si c7 ou Lucie
     voulaient des occurrences dans une entité à part elles aussi, il faut le redire avant le pas 2a′.
2. **Une lecture sans instantané sous une vue est refusée** (« réessayer »), comme le plafond retiré
   ce matin.
3. **Les relations sont restreintes aux occurrences visibles** (§ 5).
4. **Le repli exhaustif du dense se mesure d'abord** : aucune règle avant le chiffre.
5. **La rétention** telle que posée (page 30), plus un **bail** sur une génération épinglée par une
   lecture.

## 7 ter. La relecture d'arbre principal (14 novembre 2026)

- **Le chemin sans vue devient « vue = génération de l'ancre » dans le même pas que le stockage.**
  Sinon, les lignes d'une couche fuient dans toutes les lectures d'aujourd'hui : recherche, usages,
  outline, `read_view`, comptes du plein texte.
- Il y a beaucoup de lecteurs de `Scope` en dur en dehors d'`occurrence_roots_for` (la passe du 13 :
  deux passes de deux heures pour deux lecteurs oubliés). Avant le pas 2a′, il faut **relever par
  grep tous les `MATCH (… :Scope` et `Scope_` en dur, et les classer un à un**.
- **Le témoin de non-fuite**, numéroté 0 : une couche publiée ne change aucune réponse sans vue,
  dans les trois signaux et dans `usages` et `impact`.

## 8. L'ordre du lot

1. Monter lucivy en 4.3.7, et son témoin des statistiques de la vue.
2. **2a′** : le stockage des générations, l'uuid par génération, et le filtre de l'ancre pour
   tous les lecteurs sans vue, après le relevé des lecteurs en dur (§ 7 ter) : témoin 0.
   **2b′** : le chargeur et le point de composition `occurrence_roots_for` (§ 2, § 3), sur un dépôt
   de test : témoins 1, 2, 4, 5, 10, 11, 12.
3. L'instantané composé et les refus (§ 4) : témoins 7, 8, 9.
4. Les relations (§ 5) : témoin 6.

Le lot des octets d'un scope (codeparsers, puis rag3weaver) passe **avant** le pas 1 : il change
les adresses de contenu que les générations citeront (c7, 14 novembre). Arbre principal relit chaque
pas.

Chaque pas sur sa branche, rouge d'abord, et aucune base de Lucie lue ni ouverte. 2b-C (le `view`
des outils, la vue courante choisie par le contexte, la couche de travail lisible) vient après.
