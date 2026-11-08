# Les vues par worktree : la conception de la marche 1

8 novembre 2026, l'arbre principal, à la demande de c7 après la question de Lucie (20 h 15) :
« ce que j'avais demandé pour qu'un worktree n'indexe que ses diffs granulaires de scope et que les
recherches soient filtrables sur tel worktree / branche / PR, ça en est où ? ». Rien n'était commencé
depuis la vision (`visions/2026-11-06-15h46-un-index-par-depot-des-vues-par-worktree.md`) et la
lecture côté moteur (`docs/07-novembre-2026-arbre/01-ce-que-le-moteur-doit-au-modele-des-vues.md`).
Cette page est la conception ; **pas de code avant sa relecture par everything declarative et
recherche**. Lu sur master 4f54e473a.

## Ce que l'index fait aujourd'hui, et ce qu'il coûte

- Un **Scope** a pour identité `{source}#{fichier}#{nom qualifié}:{type}` (`stable_scope_keys`,
  `src/code.rs:1733`) : ni le contenu ni la signature n'y entrent. Il porte son contenu **et sa
  position** (`start_line`…`end_byte`, champs déclarés). Ses chunks pendent de lui
  (`Scope_CHUNKED_FROM`), leurs uuids sont déterministes (`chunk_uuid(parent, champ, index)`). Les
  14 relations de code partent toutes d'un Scope.
- La réindexation est **par fichier** (`code_sync::reingest_file`). Le court-circuit de l'inchangé
  (`split_unchanged`) compare **tous** les champs déclarés, lignes comprises : un scope dont le texte
  n'a pas bougé mais que l'édition d'au-dessus décale repart au chemin complet — upsert, rechunk,
  embarquement.
- Une **tâche** copie la base entière (`CopieDeBase`, reflink) et ouvre une seconde instance du
  backend ; sa montre réindexe les fichiers dont l'empreinte diffère.

**La mesure** (ce soir, `mesure-scopes.py` à côté de cette page : codeparsers `voir_json` sur la
version de base et la version de tête de chaque fichier de code changé, scopes appariés par nom,
type et rang, lambdas exclues — rag3weaver les replie dans leur parent ; les dix branches vivantes
du dépôt que code-principal et code-lourd indexent, chacune contre sa base) :

| | scopes | part |
|---|---|---|
| dans les 54 fichiers de code changés | 5 776 | 100 % |
| **neufs** | 287 | 5 % |
| **modifiés** (dont 262 conteneurs : modules, classes, espaces de noms) | 359 | 6 % |
| même contenu, **seulement déplacés** | 4 181 | **72 %** |
| inchangés, même position | 949 | 16 % |

Par branche, une vue apporte **12 à 151** versions de scope à embarquer, là où la réindexation par
fichier en reprend **88 à 1 278**. Les 72 % de déplacés sont aujourd'hui réembarqués en entier.

## 1. La version de scope, entité par empreinte de contenu

- **`ScopeVersion`** : une ligne par empreinte de contenu, `blake3(langage, type, signature,
  contenu normalisé)`. Elle porte ce qui s'embarque (contenu, signature, docstring, titre) et **ses
  chunks et ses vecteurs** (`ScopeVersion_CHUNKED_FROM`). Deux fonctions identiques, dans deux
  fichiers ou deux branches, partagent une version et un seul embarquement.
- **La position n'est pas dans la version.** Le `Scope` d'aujourd'hui devient l'**occurrence** : son
  identité reste la clé stable (source, fichier, nom qualifié, type), il garde ses lignes et ses
  relations, et cite sa version (`OF_VERSION Scope → ScopeVersion`). Une édition au-dessus d'un scope
  change les lignes de l'occurrence, pas sa version : rien ne se réembarque.
- **La version de fichier** est la liste ordonnée de ses occurrences (déjà : les scopes d'un fichier
  par `file_path`, ordonnés par `start_byte`) ; `File.content_hash` (blake3 du texte) en est
  l'empreinte.
- **Les relations restent sur les occurrences** dans cette marche (la vision : « un scope inchangé
  peut changer de comportement » — on réutilise son embarquement, jamais ses analyses).
- **Témoin rouge d'abord** : (a) deux fichiers qui portent la même fonction → **un** appel à
  l'embarqueur pour elle (un embarqueur compteur) ; (b) une ligne insérée au-dessus d'une fonction
  inchangée, réindexée → **zéro** appel pour elle. (b) est rouge aujourd'hui (le court-circuit
  compare les lignes) ; **à vérifier d'abord**, une inférence non jouée : l'upsert remettrait aussi
  `_embed_hash` à vide avant le saut par marqueur d'`EmbedNode`, et réembarquerait même un scope dont
  seules les lignes ont bougé.
- **Mesure** : sur une réindexation de 20 fichiers édités, le nombre d'appels à l'embarqueur avant /
  après (attendu : de « tous les scopes des fichiers » à « les neufs et modifiés »).

## 2. La vue : une table chemin → version, contre la base de la tâche

- **`View`** : son nom (`view:<tâche>`, `view:main`), **sa base** (le commit d'ouverture de la tâche,
  `depart` de `ouvrir_une_tache`), sa branche. **`ViewEntry`** : (vue, chemin) → version de fichier,
  ou **tombstone** (le chemin est supprimé dans la vue — sinon la vue retomberait sur la version du
  principal).
- **Définie contre la base, pas contre master** (la mesure du 7 novembre, tranchée par everything
  declarative) : 0 à 8 chemins contre la base pour dix branches, 5 à 53 contre le master du moment ;
  ce soir 1 à 16 chemins (0 à 13 de code). Une base qui bouge est un rebase explicite de la tâche,
  jamais un grossissement silencieux.
- **Résoudre un chemin dans une vue** : l'entrée de la vue si elle existe, sinon celle du principal.
- **Témoin rouge** : une tâche qui modifie une fonction ; `search_code` dans la vue rend la version
  de la vue, `search_code` sans vue rend celle du principal, et ni l'une ni l'autre ne voit les deux.

## 3. Le paramètre de vue sur les outils, et le filtre sous l'instantané L6

- **Le paramètre** `view` sur search_code, outline, usages, impact, callees (ils lisent la base) ;
  read_file et grep_files lisent le disque : la vue y choisit la **racine** (le worktree de la
  tâche), pas un filtre. Par défaut, la vue du backend appelant (le principal pour le backend du
  dépôt).
- **Le filtre**, au plus près de chaque index, par le chemin que search_code a déjà pour `scope`
  (`QueryScope` → `FilterCondition`, la projection `PROJECT_GRAPH_CYPHER` pour le dense, les offsets
  pour lucivy et le creux) : admettre les occurrences dont (fichier, version) est celle de la vue
  pour ce chemin. Sa forme : le principal **moins** les chemins que la vue remplace, **plus** les
  occurrences de la vue — une sélectivité proche de 1, la stratégie la plus simple du HNSW.
- **Sous L6, c'est possible depuis ce matin** : `SearchSourceNodeFactory::snapshot_safe` rend
  `true` sans condition (0e528854a, sur la lib dadbf0041 qui porte les projections sous
  `READ ONLY`, f010983bb) — la recherche filtrée lit dans l'instantané de son appel.
- **Ce qui manque, et se mesure avant d'en parler** : le masque du dense est **refait à chaque
  appel** (reparsé, replanifié, un sous-plan qui parcourt la table, un roaring de sa taille) et, dès
  que des versions de tâches vivent dans les tables, **toute** recherche est filtrée, celles du
  principal comprises. La mesure M3 de la lecture du 7 novembre (le coût du masque par appel selon la
  taille de la table) **conditionne** la marche 2 : petit devant les 200 à 750 ms d'un search_code,
  rien ne manque ; sinon, un masque gardé par vue et par génération — une question pour le cœur.
- **Témoin rouge** : outline et usages avec `view` rendent la version de la vue ; read_file avec
  `view` lit le worktree.

## 4. Le premier index d'un worktree : les seules versions neuves

- À l'ouverture d'une tâche, **pas de copie de base** : les chemins qui diffèrent de la base
  (`git diff --name-only <base>..HEAD`, plus l'arbre de travail) sont découpés ; chaque scope donne
  son empreinte ; une empreinte connue rattache l'occurrence à la version existante, une empreinte
  neuve crée une version et l'embarque.
- **Mesure (ci-dessus)** : 12 à 151 versions à embarquer par branche au lieu de 88 à 1 278 scopes ;
  la copie de base disparaît, et avec elle la seconde instance du backend par tâche (sa base
  ouverte, son cache, sa montre ; la copie reflink de code-lourd, 1,2 Go apparents, ne coûte presque
  rien en place, l'instance si).
- **Témoin rouge** : trois fichiers édités dans une tâche → trois entrées de vue, zéro copie de base,
  et exactement autant d'appels à l'embarqueur que de versions neuves (un embarqueur compteur).

## 5. Le drain par empreinte, le compte de références, la fermeture

- **La montre du worktree** compare l'empreinte de chaque fichier modifié ; une empreinte connue est
  une version déjà là (rien à faire), une neuve crée sa version de fichier et ses versions de scope
  neuves seulement.
- **Le compte de références** d'une version : les entrées de vue et du principal qui la citent, et
  les **fiches** (un apport `diff` qui la cite la garde, la vision) ; le ramasse-miettes ne retire
  jamais une version citée, ni une version épinglée par une génération en cours de lecture.
- **La fermeture** : tâche fusionnée → ses entrées passent au principal (la version de la vue devient
  celle du principal, sans réembarquement) ; tâche rejetée → ses entrées partent ; les versions
  orphelines partent avec leurs chunks et vecteurs.
- **Ce que le moteur coûte ici** (la lecture du 7 novembre) : un effacement HNSW ≈ 5 ms par chunk ;
  **une ligne effacée n'est jamais compactée** et la sélectivité compte les mortes — l'histoire
  pèse sur la recherche. Mesure M5 (N vues ouvertes puis fermées, la latence de search_code après) à
  faire avant de fixer la politique de ramasse-miettes.
- **Témoin rouge** : la fusion d'une tâche ne laisse ni entrée de vue ni version orpheline ; une
  version partagée par le principal et la tâche reste ; un chemin supprimé dans la vue (tombstone)
  n'apparaît plus dans les recherches de la vue et reste dans celles du principal.

## 6. La migration du gabarit de code et des bases existantes

- **Le format du gabarit** monte (le manifeste le dit) : `ScopeVersion`, `OF_VERSION`,
  `ScopeVersion_CHUNKED_FROM`, `View`, `ViewEntry`.
- **Pas par `index confirm`** : un index complet croise les deux sens des petites tables de
  relations, de façon déterministe (`docs/tickets/2026-11-08-un-index-complet-croise-les-deux-sens-des-petites-tables-de-relations.md`)
  — Lucie ne lance aucun index complet sur ses bases jusqu'au correctif. La migration est **un
  autre chemin** : une migration déclarée (`src/dataflow/migrations.rs`) qui **ajoute** des tables et
  ne réécrit aucune table de relations de code existante : chaque Scope donne son empreinte, les
  versions se créent, ses chunks et vecteurs sont **re-rattachés** à la version (pas recalculés) ; les
  occurrences gardent leurs relations telles quelles ; la vue `main` reçoit les versions du principal.
- **Toujours sur une copie d'abord** : une copie reflink de chaque base, la migration, puis
  `tools/check_rel_directions` (aucun écart avant ni après), les comptes de Scope, de chunks et de
  vecteurs égaux, et un jeu fixe de requêtes search_code qui rend les mêmes résultats avant et après.
- **Témoin rouge** : une base du format d'avant migrée rend les mêmes résultats sur ses requêtes, sans
  aucun appel à l'embarqueur, et son contrôle des deux sens est sans écart.

## L'ordre des lots

1. Le témoin rouge 1(b) et la vérification du marqueur, puis **la version de scope** (§ 1) sur le
   principal seul, avec la migration (§ 6) : c'est la marche 1 de la vision.
2. La mesure M3 (le coût du masque) ; puis **la vue** et le paramètre des outils (§ 2, § 3).
3. **La tâche sans copie de base** (§ 4, § 5).

Chacun sur une copie des bases de Lucie avant la sienne, et rien vers master sans la passe complète.
