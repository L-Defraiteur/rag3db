# Les couches de Codex face aux vues : ce qui est livré, ce qui manque

13 novembre 2026, everything declarative, à la demande de c7. Lecture du code sur origin/master
a880f61e0 ; rien n'a été compilé ni joué. Les chemins partent de
`extension/rag3weaver/src/dataflow/migrations/`.

**En une phrase** : deux briques de bibliothèque testées, qu'aucun chemin réel n'appelle encore.
Ni la recherche, ni les relations, ni L6, ni l'hôte, ni un outil ne les touche. Une vue n'est pas
interrogeable aujourd'hui.

## 1. Ce que le code fait vraiment

### Le raccord physique d'index (`physical_index.rs`, `publication.rs`)

- **Ce que c'est.** Un index **neuf, possédé, en plein texte BM25 seul** (`FORMAT =
  "owned-initial-bm25:v1"`).
  - Il refuse tout modèle dense, tout fournisseur, toute relation déclarée et toute surcharge de
    signaux (`validate_profile`).
  - Il n'adopte aucune base existante, et il ne porte qu'**un lot initial**.
- **Ce qu'il écrit.**
  - Une racine en 0700, avec un marqueur et sa base `index.rag3db`.
  - Dans `_catalog_meta`, une tête (origine, publication, descripteur, instantané) et un registre des
    déclarations.
  - Un journal des migrations qualifiées.
  - Le registre, la tête et le journal s'écrivent dans **une seule transaction**. La reprise après
    un arrêt brutal est témoignée.
- **La publication.** Une origine et des publications tirées au hasard
  (`declaration-publication-v1:<uuid>`) : ce sont des **événements**, pas des empreintes de contenu.
  - Le registre forme une chaîne et refuse d'écrire si la tête a bougé.
  - L'état physique porte deux empreintes : la logique (schéma, lignes, méta) et la matérialisation
    (les blobs du plein texte).
- **Qui l'appelle.** Personne hors de ses tests. Le reste du crate ne le voit que par **des refus** :
  `refuse_normal_storage_path` et `refuse_normal_catalog_open` empêchent qu'une ouverture ordinaire
  prenne une racine possédée. Son seul contact avec la recherche est une sonde BM25 qui vérifie que
  des uuid attendus remontent.

### Le résolveur de couches « 2a » (`publication/layers.rs`)

- **Ce que c'est.** Une résolution **pure, en mémoire**. Le module le dit lui-même : il ne calcule
  pas l'empreinte de contenu, n'ouvre aucun stockage et ne filtre aucun top-k.
- **Une couche** est `LayerGeneration { layer, origin, publication }` : un nom libre, plus une
  publication **exacte**, donc épinglée.
  - Une `LayerRevision` cite sa parente et porte des occurrences, des occurrences retirées et des
    fichiers retirés (les tombstones).
  - `PinnedLayers` valide la forêt : pas de doublon, pas de parente manquante ou d'une autre origine,
    pas de cycle.
  - `select` refuse un nom ambigu. Il n'y a ni « dernière », ni tête mobile.
- **La résolution** remonte la chaîne des parentes, puis l'applique de l'ancêtre vers la feuille.
  - À chaque couche, elle ôte les occurrences et les fichiers tombstonés.
  - Elle pose ensuite ses occurrences : **la plus proche gagne, par occurrence**.
  - Chaque occurrence visible porte sa **provenance**, la génération qui l'a gagnée.
- **Rien n'est persisté** : l'appelant fournit tout. Qui l'appelle ? Ses 4 tests.

## 2. Ce que la conception du 8 novembre voulait

Source : `extension/rag3weaver/docs/08-novembre-2026-arbre/01-conception-des-vues-marche-1.md`,
précisée le 11 par `docs/arbre-principal/11-11-2026/03-contrats-et-lots-des-vues.md`.

- **Une vue est un filtre sur un seul index**, vu par trois lunettes : le worktree, la branche, la
  pyramide d'un fichier ou d'un scope. Rien n'est copié ni réindexé.
- **La vue est nommée par sa branche.** Attacher un worktree monte la vue de sa branche, et ce qui
  n'est pas commité forme une couche de travail par-dessus.
- **La pyramide.** Il y a une ancre par dépôt. Chaque vue est posée sur sa parente et **le fichier
  est composite** : par scope, la version de la couche la plus proche, sinon on descend la pile. Un
  tombstone masque le dessous.
- **La version de scope.** L'entité est une empreinte de contenu (`ScopeVersion`) partagée entre
  occurrences, de sorte qu'un contenu identique n'est embarqué qu'une fois.
- **Le paramètre `view` sur les outils**, et le masque appliqué **avant le top-k** de chaque signal
  (dense, creux, plein texte), sous un seul instantané L6.
  - Les deux bouts d'une relation doivent être visibles.
  - Une vue inconnue est refusée.
  - La provenance n'apparaît qu'à la demande (`provenance: true`), et ne coûte rien sinon.

## 3. Où ça converge

- **La résolution est celle de la pyramide.** Elle remonte les parentes, la plus proche gagne, et un
  tombstone (d'occurrence ou de fichier) masque tout le dessous. C'est le témoin du 8 novembre, f de
  la parente, g de la fille, h de l'ancre, et les tests de 2a le jouent.
- **Les générations sont épinglées** : une ancre qui avance ne change pas une vue en douce, comme le
  contrat du 11 le demande.
- **La provenance par occurrence** est là, ce que Lucie a précisé le 11 : un scope hérité cite la
  parente, un scope remplacé cite le worktree.
- **Une vue ambiguë est refusée** au lieu de se replier sur une autre.

## 4. Où ça diverge

- **La couche n'est ni une branche ni un worktree.** C'est un nom libre, plus une publication tirée
  au hasard. Rien ne relie encore une branche git, un commit de base ou un worktree attaché à une
  `LayerGeneration`. La conception nommait la vue par sa branche.
- **L'identité n'est pas le contenu.** La publication est un événement (un uuid), pas une empreinte.
  La `ScopeVersion` par contenu, qui fait l'économie d'embarquement (12 à 151 versions par branche au
  lieu de 88 à 1 278 scopes), **n'est pas livrée** : le ticket parent
  `docs/arbre-principal/tickets/2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md`
  est toujours ouvert. 2a reçoit les occurrences comme des valeurs opaques et ne sait rien de leur contenu.
- **Le physique n'est pas l'index du code.** Le raccord ne connaît qu'un index neuf en BM25 seul,
  sans dense ni relations, à un seul lot. L'index de code réel (dense, creux, lucivy, relations) en
  est exclu par construction.
- **Pas de couche de travail.** Le non-commité d'un worktree n'a pas d'équivalent.

## 5. Ce qui manque pour qu'une vue soit interrogeable

C'est le 2b de Codex (`docs/arbre-principal/13-11-2026/28-plan-visibilite-couches-2b.md`), avec
treize contre-exemples spécifiés, non écrits, non joués :

1. **Un chargeur durable des couches** : leurs parentes, leurs occurrences et leurs tombstones,
   persistés et relus. Aujourd'hui, tout se fournit à la main.
2. **La liaison occurrence → ce que les index stockent** (morceaux, offsets lucivy, vecteurs par
   modèle), par génération.
3. **Un instantané composé** : la base et les handles lucivy et creux lus ensemble. L6 seul ne le
   garantit pas. Pas de réponse à demi-instantané.
4. **Le masque avant le top-k** de chaque signal et des lecteurs de relations. Un signal qui ne sait
   pas filtrer refuse la requête (2b-A en CPU sur fixture, 2b-B sur l'index natif).
5. **Le paramètre `view` dans les outils réels** : recherche, usages, lecture, relations (2b-C). La
   vue courante se choisit d'après le contexte, et une vue inconnue est refusée avant toute
   recherche.

S'y ajoutent, hors du 2b, ce que la conception mettait d'abord : **la version de scope par contenu
et sa migration** (lots 1a et 1b), puis **la liaison branche/worktree → couche** et la tâche sans
copie de base (lots 3a et 3b). Tant que 1a n'est pas livré, une vue interrogeable réembarquerait
encore chaque contenu identique.

## 6. Ce que j'en retiens

- Ce que Codex a livré est **sain et bien gardé**. Les invariants sont durs : pas de tête mobile,
  pas de repli, une transaction unique, des refus nommés.
- Mais il a été bâti **par le bas, sur un index jouet** (BM25 neuf, un lot). Le chemin vers la vue
  de Lucie passe par trois choses qui ne sont pas commencées :
  - la version de contenu (1a) ;
  - le branchement du masque dans les signaux réels (2b-B) ;
  - la liaison branche/worktree → couche.
- Une question à trancher avant 2b-B : **l'index de code réel adopte-t-il le raccord physique**,
  aujourd'hui interdit (« no adoption of existing storage »), ou 2b-B reste-t-il sur un index neuf ?
  Sans l'un ou l'autre, aucune base de Lucie n'aura de vue.

## 7. Ce que Lucie a tranché (13 novembre 2026, par c7)

**Les vues se bâtissent sur l'index de code réel** : les versions par contenu, puis les couches et les
générations, ajoutées lot par lot. L'index neuf de Codex (le raccord physique BM25) reste un banc
d'essai de ses mécanismes, pas la base des vues. La question du § 6 est donc close : pas d'adoption
du raccord physique par l'index réel pour les vues. Arbre principal poursuit le lot 1a dans ce sens
(`docs/arbre-principal/13-11-2026/29-contrat-de-la-version-de-scope.md`) ; everything declarative
relit les lots 1a et 1b.
