# Mémoire longue — rapport de session

3 octobre 2026, 23 h 50. Session « mémoire longue » (lignée d'août : la
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

- **`e2e_arret_brutal`** : deux cas verts, un rouge **connu et nommé** qui
  attend un moteur postérieur à `fcd9a7882`. La bibliothèque liée datait de
  21 h 33, la garde 1 de 23 h 11.
- **La sonde du catalogue** face à un index détaché est écrite
  (`CATALOGUE=ok | erreur: … | panique`) et n'attend que ce moteur.

## 5. Ce qui attend quelqu'un

| Quoi | Qui | État |
|---|---|---|
| rebâtir `build/lecteurs-csv` dans un creux, annoncé aux sessions qui le lient | arbre principal | demandé par l'orchestration |
| l'âge de la bibliothèque imprimé par `run_e2e.sh`, et refus d'une bibliothèque plus vieille que les sources C++ | arbre principal | à proposer |
| reconnaître un index détaché à l'ouverture et le rebâtir en le disant | arbre principal | après la garde 1 |
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

1. le **réacteur** (lot 4) : lire `EntitiesChanged`, remonter les
   `ANCHORED_TO` entrantes, transitionner par `review`. **Demande un nœud
   neuf** — rien ne consomme le port `events` sauf `TraceSinkNode`, et un
   graphe-outil n'a pas de conditionnelle. En attente du mot de Lucie.
2. le crochet après outil (session recherche) et le jardinier ;
3. `Subject` en **entité dérivée** — la mesure dit que le bon classement vient
   du *texte complet* du sujet, et une description écrite une fois ne contient
   pas ce qu'on range dessous ;
4. le nœud de décision **à modèle**, en dernier.
