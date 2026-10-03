# Mémoire longue — rapport de session

3 octobre 2026, puis la nuit du 4. Session « mémoire longue » (lignée d'août : la
session comme graphe, le langage de déclaration, la machine à états).
Mis à jour sur place.

## 1. Ce qui est fusionné sur master

| Commit | Ce que c'est |
|---|---|
| `66cbcf45f` | la proposition — entité, verbes, « à revoir », banc, sujets, nœud de décision |
| `4c348ea44` | le gabarit `templates/backends/memory` et le banc |
| `74a615a1c` | l'ingestion émet `EntitiesChanged` |
| `5d1a3189b`, `266c972fa`, `4e5bd8c07` | le banc passe à 72 paires de contrôle versionnées |
| `7438b2909` | la portée `person`, dette nommée avec sa condition de sortie |
| `5b3d0656f` | `e2e_arret_brutal` — le témoin produit de la reprise |

**Zéro ligne de Rust pour `Memory` et `Subject`.** Le gabarit `notebook`
avait déjà prouvé qu'une entité à machine à états, identité stable et
recherche hybride se déclare sans code ; `Memory` en est la déclinaison.

## 2. Ce qui a été décidé, et pourquoi

**`remember` n'a qu'un seul verbe**, appelé deux fois, dont le premier appel
est un refus qui **montre** les proches et dit l'appel exact à refaire.
J'allais proposer deux verbes (`remember` puis `apply_memory`) sur le modèle
plan/application. Le dépôt avait déjà mesuré le défaut : `search_expand` a été
retiré le 28 août pour **zéro appel sur quarante**, parce qu'« un agent qui
hésite entre deux outils proches en prend zéro ». Et `place.mmd` porte la
doctrine « un outil qui confirme sans montrer oblige son appelant à une
seconde requête ».

**Le premier appel n'écrit rien** : un modèle qui s'arrête là ne crée aucun
désordre. L'échec sûr est le silence — d'où « propositions jamais appliquées »
comme mesure du banc, parce qu'un échec silencieux qu'on mesure cesse de
l'être.

**L'ancre vers n'importe quelle entité** : `ANCHORED_TO` engendrée à
l'enregistrement d'une entité qui se déclare `anchorable`, ni relation par
couple écrite à la main, ni pivot à champs — un pivot jette précisément ce
pour quoi on a un graphe. La traversée est **déjà** polymorphe
(`FetchRelatedNode` lit `label(m)` à l'exécution) ; seule la déclaration ne
l'était pas.

**Les ancres sont nues.** Une propriété d'arête ne se cherche ni ne se filtre,
et `RelationBatchNode` refuse une relation à propriétés en indiquant le
remède. Origine et date sont des propriétés de la **mémoire**.

**« À revoir » a deux sources, pas une** : la synchronisation sait ce qui
**manque**, l'ingestion sait ce qui a **changé** — et c'est le cas courant,
un fichier édité. C'est le désaccord avec la vision qui a changé le plus de
travail.

**La création d'un sujet se confirme, et seulement s'il y a un proche** (Lucie,
nuit du 4 octobre, relayé par l'orchestration). C'est la forme du verbe unique,
précisée sur son point le plus important : le refus ne se déclenche **que si un
proche existe** — au-dessus d'un seuil de similarité contre le *texte complet*
du sujet, celui que la mesure désigne. Sans proche, la création passe du premier
coup, sans friction. Avec un proche, le refus nomme le sujet, **montre un
extrait de son contenu** et donne les deux appels exacts : rejoindre celui-là,
ou confirmer (`confirm: true`).

Ce qui se décide là, et qui vaut d'être dit : **c'est l'agent appelant qui est
le modèle de décision.** C'est ce que la mesure de l'optimiseur a montré de
mieux, et il décide en voyant le contenu, pas un nom — exactement ce que mes
chiffres disaient (ranger par le cosinus : 14/16 sur le texte complet, 8/7/15
sur les noms seuls).

Et **les deux à la fois**, le harnais comme garantie et la consigne (« cherche
d'abord un sujet lié », dans la description de l'outil et le prompt du backend)
comme incitation. La consigne seule ne suffit pas : les deux passes d'agent du
3 octobre ont montré qu'un modèle faible comme un modèle fort ignorent une
ligne qu'on ne les oblige pas à lire, et qu'un modèle faible ne tient pas deux
tours. D'où un refus qui porte l'appel exact à refaire, lisible en un tour.

Cela remplace « dans le doute, créer et demander après » : dans le doute,
**demander à l'appelant avant**, par le refus. Le jardinier reste pour ce qui
passe quand même. Même forme pour une mémoire proche d'une mémoire existante.
Ce que le banc devra mesurer : confirmations déclenchées à tort (la friction),
doublons créés malgré tout, sujets rejoints à tort.

**L'étage 2 du nœud de décision reste sur « demander toujours »**, aucun
modèle branché, critère et liste de choix en **données du graphe**. Trois
raisons : Lucie garde la découpe ouverte (« ça reste une expérimentation,
faut pas courir avec avant d'avoir trouvé si une bonne manière de s'en servir
existe ») ; aucune piste mesurée ne sait décider qu'il faut **créer**, sur
aucun des trois jeux ; et un diagnostic de formulation est en cours.

## 3. Ce que j'ai retiré après l'avoir écrit

Noté ici parce qu'un rapport qui ne dit que ses succès ment par omission.

- **« Variante sort des choix »** — tranché sur un 85 % contre 64 %, et
  prématuré : la mesure dit qu'une découpe est difficile, pas laquelle la
  remplace.
- **« Quatre candidats » et « nom et description suffisent »** — adoptés, puis
  retirés : rejoués ailleurs, 15 sur 16 devient 13 sur 32.
- **« L'index vectoriel est nécessaire »** (arrêt brutal) — bonne observation,
  mauvaise explication. Le BM25 survit parce que **rien n'a replié son
  journal** après le chargement d'extension, pas parce qu'il n'a pas d'index.
- **« Les suites dans `temp_dir()` ne prouvent pas ce qu'on croit »** — faux
  pour un SIGKILL : le cache du noyau survit au processus. Cela ne compte qu'à
  une coupure de courant. Corrigé avant que le cœur C++ ne le propage.
- **« Le témoin sans point de reprise réfute la cause »** — mon témoin était
  mal conçu, pas la cause mal dite.

**La règle qui sort de là** : une mesure est un fait sur le jeu qui l'a
produite. Ce qu'elle autorise, c'est d'**éliminer**, pas de choisir.

## 4. En cours

### `e2e_arret_brutal` : trois cas verts, et un vert qui ne prouve pas ce qu'on croirait

Rejoué contre `librag3db.so` et `libvector` du 3 octobre 23 h 50 (garde 1
`fcd9a7882` comprise), âge imprimé pour les deux : **3 passed**. Chaque cas
porte sa condition de validité à côté de son verdict — journal non vide avant
la mort (63 987 / 65 030 / 65 509 octets), `REPLI=non` pour le témoin sans
point de reprise.

**Et le cas vectoriel a imprimé sa propre réserve** :
« le montage ne voit plus d'index en retard : rebâti de lui-même ? »

Deux défauts du témoin, trouvés après coup, qui empêchent de lire ce vert
comme une preuve :

1. **Il redéclare le schéma** (`register_entity("Fiche", …)` après
   l'ouverture), et c'est ce `CREATE_VECTOR_INDEX` qui recrée l'index. Le vert
   ne distingue pas « la garde a réparé » de « je l'ai recréé moi-même ».
2. **Son journal portait encore le `LOAD EXTENSION`.** Le rejeu a donc chargé
   l'extension et l'index est resté juste — c'est le cas où le détachement
   *n'a pas lieu*, et c'est exactement l'équivalence que les quatre cas de la
   veille avaient établie. Le témoin mesurait le cas complémentaire du sien.

### Le lot « index vectoriel détaché » (orchestration, nuit du 4)

Reconnaître à l'ouverture un index vectoriel absent ou détaché, le rebâtir, et
le dire. Code posé, `cargo check` en cours, pas encore commité :

| Où | Quoi |
|---|---|
| `src/catalog.rs` | `INDEX_BEHIND_ITS_TABLE`, `CatalogError::IndexDetache`, `all_vector_indexes`, `rebuild_detached_vector_indexes`, et le `DROP` devant le `CREATE` dans `rebuild_vector_index` |
| `src/catalog.rs` (`initialize`) | étape « 10 quater », après la restauration par drapeau |
| `src/search.rs` | le refus **nommé** au lieu d'une `DbError` opaque |
| `tests/e2e_arret_brutal.rs` | les trois sondes brutes du relecteur, avant toute redéclaration |

**La décision de conception qui porte le lot** : détecter par **sonde** et non
par drapeau. Le drapeau `vector_index_dropped:` ne connaît que les index que
*nous* avons retirés ; le rejeu du moteur n'en pose aucun. La sonde, c'est le
`CREATE … skip_if_exists` lui-même — muet sur un index sain, réparateur sur un
absent, refusé par le fragment sur un détaché. Une seule question qui est
aussi sa réponse. Et elle balaie **tous les modèles enregistrés**, pas
seulement le courant : l'index d'un modèle d'avant n'est recréé par personne.

**Ce que je ne sais toujours pas, et que la prochaine passe doit dire.** Je
n'ai **jamais vu** d'index détaché. Je ne sais donc pas si la garde 1 laisse un
index *détaché* ou *retiré* — les deux demandent une réparation différente.
D'où les trois sondes brutes ajoutées au relecteur (`INDEX_VUS`,
`INDEX_CIBLE`, `VECTEUR=`, `SONDE_CREATE=`), dans cet ordre précis : la sonde
`CREATE` **répare** un index absent, donc elle vient après les observations,
sinon elle efface ce qu'elles allaient voir.

**La recette qui fabrique l'état à coup sûr** (orchestration, d'après les dix
cas du cœur C++) : l'index vient d'une session **précédente, fermée
proprement** ; la session suivante ouvre — donc enregistre un `LOAD EXTENSION`
—, fait un **`CHECKPOINT` explicite** qui vide le journal de cet
enregistrement, écrit dans la table indexée, et meurt. Et surtout : **ne pas**
essayer « ouvrir sans l'extension puis écrire », le moteur refuse cette
écriture hors rejeu.

## 5. Ce qui attend quelqu'un

| Quoi | Qui | État |
|---|---|---|
| rebâtir `build/lecteurs-csv` | `rag3db-50` | **fait** — master `a5cef54cc`, 3 oct. 23 h 50 |
| l'âge de la bibliothèque imprimé par `run_e2e.sh`, et refus d'une bibliothèque plus vieille que les sources | `rag3db-50` | **fait** — `2914d470e`, avec `RAG3WEAVER_MOTEUR_ANCIEN=1` pour passer outre en le disant |
| reconnaître un index détaché à l'ouverture et le rebâtir en le disant | **moi** | lot en cours (§4) |
| les deux cas du chargement en masse interrompu | **moi** | scénario reçu de `rag3db-50`, à écrire |
| le crochet après outil (`context` : identités résolues, source, cellule, lignes) | recherche | contrat acté, code après la passe Gemini |
| un modèle de décision, quand une bonne manière de s'en servir existera | optimiseur | diagnostic en cours |

## 6. Comment reprendre

```sh
# Worktree, branche, target sur disque (jamais /tmp : tmpfs de 61 Gio)
cd ~/git_workspaces/rag3db-memoire          # branche memoire-longue
export CARGO_TARGET_DIR=$HOME/.cache/rag3weaver-build/memoire
export RAG3DB_ROOT=$HOME/git_workspaces/rag3db          # le C++ bâti est là
export RAG3DB_BUILD=$RAG3DB_ROOT/build/lecteurs-csv
export CARGO_BUILD_JOBS=8 LUCIVY_SCHEDULER_THREADS=8    # Lucie travaille

# Le banc (service d'embarquement pour la session 3)
RAG3WEAVER_EMBED_SERVICE=127.0.0.1:7878 RAG3WEAVER_EMBED_CHAR_BUDGET=4096 \
  RAG3WEAVER_GPU_DUTY=70 python3 scripts/test_banc_memoire.py

# Les suites du changement
./run_e2e.sh --test e2e_simple_entity --test e2e_arret_brutal --summary
```

**Après chaque rebase qui amène du code** : vérifier le sous-module, parce que
`git status` ne montre rien.

```sh
git ls-tree HEAD extension/rag3weaver/codeparsers | awk '{print $3}'
git -C extension/rag3weaver/codeparsers rev-parse HEAD   # doivent coïncider
```

**Jamais `git stash` ici** : la pile est partagée entre worktrees, et un `pop`
dépile celui d'une autre session sans que rien ne s'en voie chez elle.
`git diff > fichier.patch`.

## 7. L'ordre de la suite

1. le lot **« index vectoriel détaché »** — une base qui se rouvre passe avant
   le reste (arbitrage de l'orchestration, nuit du 4) ;
2. le **réacteur** (lot 4) : lire `EntitiesChanged`, remonter les
   `ANCHORED_TO` entrantes, transitionner par `review`. **Demande un nœud
   neuf** — rien ne consomme le port `events` sauf `TraceSinkNode`, et un
   graphe-outil n'a pas de conditionnelle. En attente du mot de Lucie.
3. le crochet après outil (session recherche) et le jardinier ;
4. `Subject` en **entité dérivée** — la mesure dit que le bon classement vient
   du *texte complet* du sujet, et une description écrite une fois ne contient
   pas ce qu'on range dessous ;
5. le nœud de décision **à modèle**, en dernier.
