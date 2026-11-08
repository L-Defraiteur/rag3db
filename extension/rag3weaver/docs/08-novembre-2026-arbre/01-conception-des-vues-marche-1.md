# Les vues par worktree : la conception de la marche 1

8 novembre 2026, l'arbre principal, à la demande de c7 après la question de Lucie (20 h 15) :
« ce que j'avais demandé pour qu'un worktree n'indexe que ses diffs granulaires de scope et que les
recherches soient filtrables sur tel worktree / branche / PR, ça en est où ? ». Rien n'était commencé
depuis la vision (`visions/2026-11-06-15h46-un-index-par-depot-des-vues-par-worktree.md`) et la
lecture côté moteur (`docs/07-novembre-2026-arbre/01-ce-que-le-moteur-doit-au-modele-des-vues.md`).
Cette page est la conception ; **pas de code avant sa relecture par everything declarative et
recherche**. Lu sur master 4f54e473a.

**L'essentiel, mesuré ce soir.** Sur les 5 776 scopes des fichiers de code que dix branches vivantes
changent : 5 % sont neufs, 6 % modifiés, **72 % seulement déplacés** (même contenu, d'autres lignes),
16 % inchangés. Une vue apporte **12 à 151 versions de scope** par branche, là où la réindexation par
fichier reprend **88 à 1 278 scopes** ; les déplacés ne sont **pas** réembarqués aujourd'hui (le
témoin `a_scope_that_only_moves_is_not_embedded_again` est vert : le marqueur d'embarquement survit),
mais ils sont **réécrits** — upsert, redécoupage, insertion des morceaux — et une tâche coûte une copie
de base et une seconde instance du backend. Lucie (20 h 35) : **une pyramide de couches** — une ancre
par dépôt, chaque vue posée sur sa parente, un fichier composite par scope (§ 2 bis).

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
- **Témoin rouge d'abord** : deux fichiers (ou deux vues) qui portent la même fonction → **un** appel
  à l'embarqueur pour elle. Rouge aujourd'hui : l'uuid d'un morceau dérive de son Scope, donc du
  fichier — une fonction identique dans un autre fichier, ou dans la base copiée d'une tâche qui ne
  l'avait pas, se réembarque.
- **Une inférence retirée** (la première version de cette page) : l'upsert remettrait `_embed_hash` à
  vide avant le saut par marqueur et réembarquerait les déplacés. **Fausse** : le témoin
  `backend::tests::a_scope_that_only_moves_is_not_embedded_again` (branche
  `arbre-principal/deplace-sans-reembarquer`, 021340283) insère une fonction au-dessus d'une
  inchangée et réindexe le fichier : l'embarqueur ne reçoit que la neuve. Il reste comme garde-fou.
  Ce que coûtent les déplacés aujourd'hui, c'est l'écriture (upsert, redécoupage, insertion des
  morceaux, relations), pas l'embarquement.
- **La clé stable survit-elle au déplacement d'un bloc ?** (question de c7) Oui pour l'identité : elle
  ne porte ni les lignes ni le contenu, seulement un rang entre homonymes de même parent et de même
  type, qui ne bouge que si l'ordre de ces homonymes change. **Mais les relations sont réécrites
  quand même** : la réindexation d'un fichier oublie toutes les arêtes sortantes de ses scopes
  (`oublier_les_aretes_du_fichier`) et les recrée, quelle que soit la clé. Les 72 % de déplacés
  restent donc des réécritures de relations — le chemin où les deux sens se croisent (ticket de
  l'index complet). Le versionnage doit les éviter : une occurrence dont la version et le contexte de
  résolution n'ont pas changé garde ses arêtes ; seules ses lignes se mettent à jour.
- **Mesure** : sur une réindexation de 20 fichiers édités, les écritures (lignes upsertées, morceaux
  réinsérés, arêtes oubliées et recréées) avant / après ; et les appels à l'embarqueur pour une
  fonction copiée d'un fichier à un autre.

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

## 2 bis. La pyramide de couches (Lucie, 8 novembre, 20 h 35)

Lucie : « pour tous les autres scopes tu vois dessous l'index ancre que tu as choisi, master ou main,
la référence ; sauf quand tu pars déjà d'une autre branche : là tu vois d'abord la branche parente,
granulairement sur les scopes ; une pyramide de versioning qui se voit et se navigue, et le fichier
est composite de ça ».

- **Une ancre par dépôt** (master ou main), indexée en entier une fois : c'est la vue `main`.
- **Une vue est une couche posée sur une vue parente** — l'ancre, ou la vue de la branche d'où la
  tâche est née — et ne porte que les versions de scope qu'elle change. Elle porte donc **son
  parent**, pas seulement sa base git.
- **Un fichier est composite** : par scope, la version de la couche la plus proche qui en a une ;
  sinon on descend la pile ; un tombstone d'une couche masque les couches du dessous.
- **La fusion d'une couche dans sa parente** est une opération de l'index : ses versions descendent
  d'une couche, ses tombstones s'appliquent, la couche disparaît ; les PR empilées sont le cas
  normal. **Le compte de références compte les couches** (une version citée par une couche vivante
  reste).
- **La mesure** : sur les dix branches vivantes, **2 naissent d'une autre branche que master**
  (`gabarits-vivants` et `atelier-ui`, sur `atelier-commandes`) ; sur les 743 branches distantes non
  atterries (beaucoup sont des copies rebasées), 668 sont de profondeur 1, 60 de profondeur 2, 10 de
  3, 3 de 4, 2 de 5. La pile est courte ; une résolution en quelques sauts suffit.
- **Témoin rouge** : une tâche née d'une branche qui a modifié `f` et qui modifie `g` du même fichier ;
  dans sa vue, `f` vient de la parente, `g` d'elle, `h` de l'ancre ; un tombstone de `h` dans la
  parente la masque aussi dans la tâche.

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
- **La résolution de la pile est transparente par défaut** (Lucie, 20 h 40) : read_file rend le
  composite comme un fichier ordinaire, search_code et usages filtrés par la vue rendent des résultats
  sans couche ; un agent n'est pas obligé de les voir. **La provenance par scope est à la demande** :
  un paramètre optionnel `provenance: true` sur outline, read_file et search_code ajoute à chaque
  scope sa couche d'origine (« de la vue », « de la branche parente », « de l'ancre ») — « ce bout de
  fichier vient de quelle branche ? ah, toujours de l'ancêtre » —, ou une question au chat qui la
  résout par le même chemin. **Coût en jetons nul quand on ne la demande pas.** L'IDE, lui, la montre
  toujours : c'est là que la pyramide se voit et se navigue (chaque scope dit sa couche, on remonte la
  pile).
- **Témoin rouge** : outline et usages avec `view` rendent la version de la vue ; read_file avec
  `view` lit le worktree ; sans `provenance`, la réponse est identique à celle d'un fichier ordinaire,
  octet pour octet ; avec, chaque scope porte sa couche.

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

1. **La version de scope** (§ 1) sur l'ancre seule, avec la migration (§ 6) : c'est la marche 1 de la
   vision ; le garde-fou du scope déplacé (021340283) y entre.
2. La mesure M3 (le coût du masque) ; puis **la vue en couches** et le paramètre des outils, la
   provenance à la demande (§ 2, § 2 bis, § 3).
3. **La tâche sans copie de base** (§ 4, § 5).

Chacun sur une copie des bases de Lucie avant la sienne, et rien vers master sans la passe complète.
