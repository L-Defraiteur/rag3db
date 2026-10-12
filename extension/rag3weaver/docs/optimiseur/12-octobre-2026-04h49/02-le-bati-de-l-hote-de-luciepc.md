# Le bâti de l'hôte de luciepc : ce qui attend Lucie

*Session optimiseur (G), 12 octobre 2026, matin. Le script :
[`02-hote-bati7.sh`](02-hote-bati7.sh), à copier sur luciepc sous
`~/.cache/rag3weaver-build/`.*

## Où on en est

- **Rien n'est bâti.** `hote-bati6.sh` attendait sa place de poste depuis 3 h 50.
  À 6 h 40, la place s'est libérée, mais le script est mort tout de suite sur
  « configure rouge » : il écrivait son journal dans `build/`, qui n'existait pas
  dans le worktree.
- **Le binaire `~/.local/bin/rag3weaver-backend.pret` du 11 octobre à 22 h 13
  est l'ancien** (md5 251f757a…, d'avant B4). **Ne pas l'installer.**
- L'index.sh de codeparsers (pid 132918) s'est terminé de lui-même. L'hôte figé
  (pid 132823) tourne toujours : personne n'y a touché. D'après codeparsers, il
  occupe **44 Go de RSS (pic 52 Go)** et il ne reste que 18 Go de libres. Un
  index de code-principal lancé dedans à 6 h 39 est figé à 99 %. **Ne lancer le
  bâti qu'une fois l'hôte arrêté** (étape 1) : la lib à 12 fils à côté de lui
  risquerait de faire tomber luciepc en manque de mémoire.
- Le worktree `~/git_workspaces/rag3db-hote` est resté à 0117132af. Le bâti doit
  porter 3cca0ae1e (l'insertion HNSW du cœur : la lib change). Avancer ce worktree
  partagé m'a été refusé par ma permission : c'est à Lucie de le faire.

## L'ordre

1. La pile de l'hôte figé (gdb), puis son arrêt :
   `systemctl --user stop rag3weaver-daemon`.
2. Avancer le worktree à la tête de master, qui doit contenir 3cca0ae1e :
   ```
   cd ~/git_workspaces/rag3db-hote
   git fetch origin master
   git checkout --detach FETCH_HEAD
   git submodule update
   ```
3. Lancer le bâti sous le poste :
   `~/.cache/rag3weaver-build/poste lourd bash ~/.cache/rag3weaver-build/hote-bati7.sh > ~/.cache/rag3weaver-build/hote-bati7.log 2>&1`
   Compter une petite heure : la lib et l'extension en Release, puis l'hôte en
   release.
4. Lire `hote-bati7.log` : il doit finir par « prêt » et « note ». Puis suivre
   la note qu'il écrit, `~/.local/share/rag3weaver/POUR-LUCIE-hote.md` :
   installation des deux `.pret` neufs (md5 dans la note) et activation de
   `check-lib.conf`, **sans démarrer l'hôte**.
5. La section « Pour Lucie » de codeparsers (eef060968) : la montre et
   `queries_via` dans les deux manifestes, les bases d'avant B4 effacées. **Puis
   seulement** le démarrage, `/sante`, et la réindexation par `index2.sh`. Un
   hôte démarré avant ouvrirait une base d'avant B4.

## Ce que le script exige et produit

Il exige :

- une tête qui contient 3cca0ae1e. Il le vérifie et refuse sinon, code 2.
- le poste libre pour un lourd.

Il fait et produit :

- il crée `build/` lui-même ;
- il bâtit le moteur et l'extension vector (cmake Ninja Release), et les publie
  au magasin de luciepc par `rag3db_libs.py publier` ;
- il bâtit `rag3weaver-backend` et `rag3db-check-lib` par
  `scripts/cargo_magasin.sh`, avec `RAG3DB_LIB=<identifiant publié>` et non
  `courant`. Le RUNPATH vise le dossier de cet identifiant ;
- il vérifie le résultat : chaque symbole `rag3db` du binaire doit être défini
  dans la lib, et `rag3db-check-lib` doit rendre 0 ;
- il pose `rag3weaver-backend.pret`, `rag3db-check-lib.pret` (avec leurs md5) et
  `check-lib.conf.pret` (inactif). Il n'installe rien et ne relance rien.
