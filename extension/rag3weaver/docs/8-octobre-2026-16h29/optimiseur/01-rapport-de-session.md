# Optimiseur — rapport de session

**Mis à jour le 10 octobre 2026 en soirée.** Chantier G, le paquet npm de rag3weaver, depuis le matin ; cadrage :
`../orchestration/03-le-paquet-npm.md`. Le rapport précédent (envois à
tracel-ai, modèles de décision) est dans `../../3-octobre-2026-23h31/optimiseur/`.

## État

- Branche `paquet-npm` (worktree `../rag3db-paquet-npm`) : elle fond master
  et `embarqueur-absent` (c98d4ff9b) ; la porte pose `AbsentEmbedder`, plus
  aucun mock dans le produit ; `npm test` passe. Rien n'est publié ; c'est
  l'orchestration qui publie `0.0.1-alpha.1` sous `next` avec les OTP de
  Lucie, depuis `~/.cache/rag3weaver-build/paquet-npm/publier/`
  (sous-paquet d'abord).
- La démo `bindings/nodejs/demo/demo.sh` est répétée et verte (voir le
  relevé) ; les tunnels 7979-7981 vers luciepc (services 7878-7880) sont
  tenus par une autre session, ne pas les rouvrir.
- Linux x64 publié. **Les sous-paquets Windows x64 et macOS arm64 sont
  montés** (ba387459f : `npm/windows-x64`, `npm/darwin-arm64`, dépendances
  optionnelles ; dans le workflow : cache du bâti sur tout le target,
  extension vecteur par le cmake de cargo, `npm test` sur le runner ;
  `prepareManifest` pose `sandbox: off` hors Linux) ; leurs premiers runs
  (Windows 38068526690, macOS 38068584216) tournaient sur GitHub à la pause
  du nettoyage du disque — lire leur verdict en reprenant. macOS x64 est à
  Lucie. Ordre tranché par l'orchestration : Windows, macOS arm64, puis le
  vecteur en statique comme amélioration commune.
- Pause du 10 octobre au soir : le poste nettoie ses disques, tous les
  target et worktrees sauf l'arbre principal sont effacés ; les bâtis se
  refont par Docker sur luciepc (`~/git_workspaces/rag3db-paquet-npm` et
  `~/.cache/rag3weaver-build/paquet-npm/docker-cache` là-bas).

## Ce qui est fait

Tout est dans `../../optimiseur/10-octobre-2026-11h53/01-le-premier-bati.md`,
en une phrase chacun :

- **Linux x86_64 bâti dans un Docker commité** (`tools/build-images/linux-x64-gnu/`,
  base manylinux_2_28) : binaire de 79 Mo après strip, extension vecteur de
  684 Ko, glibc 2.28 au plus, 14 à 20 min à froid sur un runner à 4 cœurs,
  10 min en natif ici. Il démarre, charge un backend de code avec Landlock,
  **y compris dans un conteneur Docker**, et refuse en le disant sans service
  d'embarquement.
- **Windows x64 sur `windows-latest`** : dix essais, la liste des accrocs
  est faite. clang-cl a été une impasse isolée (le moteur compile, mais
  l'exécutable ne se lie pas : destructeurs de `std::variant` non émis avec
  la STL MSVC) → retour à `cl` pur, comme l'amont ; l'accroc qui y avait mené
  (drapeau GCC de tree-sitter-scss) est réglé chez codeparsers (copie
  locale `vendor/tree-sitter-scss`, master 4c7897c, sous-module pointé).
  Quatre changements de sources restent appliqués sur la branche pour
  l'essai (landlock Linux seulement ; NodeTableDeleteState sans copie ; pas
  de -latomic sous Windows ; la crate rag3db ne bâtit que la cible statique,
  déjà sur master 7ab374aba). **Le dixième essai a lié le binaire.**
- **Le paquet JS** (`bindings/nodejs/`, commité 2f8ce0602) : chargeur par
  plateforme, manifeste préparé depuis les gabarits, classe `Backend` en
  lignes JSON, sous-paquet `rag3weaver-linux-x64-gnu`, `scripts/preparer.sh`,
  épreuve « dossier vide, trois fichiers, une recherche ».
- **Le service d'embarquement optionnel au démarrage** (décision de
  l'orchestration) : les deux portes sont écrites (binaire et
  `PreparedBackend::open` : avertissement nommé, rendu aussi sous `warnings`
  par `describe` et `index_state`) ; le cœur (embarqueur absent, aucun
  vecteur factice dans le produit) est le lot de l'arbre principal.
- Le workflow manuel `.github/workflows/paquet-npm.yml` (Linux par l'image,
  Windows en essai, jobs choisissables, rien de publié) ; posé aussi sur
  master pour être dispatchable (9782b26a6).

## Décisions reçues

- Docker n'était pas sur ce poste : Lucie l'installe (actif après le
  redémarrage) ; luciepc l'a aussi. Le premier bâti **dans** Docker se fera
  donc ici, par `tools/build-images/build.sh linux-x64-gnu`.
- Le bac à sable : le comportement actuel reste (refus nommé sous Windows et
  sous Linux sans Landlock) ; c'est le gabarit livré par le paquet qui pose
  `"sandbox": {"mode": "off"}` sous Windows, avec le pourquoi.
- Le vecteur en statique est le geste d'après le paquet JS ; dire à l'arbre
  principal avant de toucher au manifeste ; `libvector.rag3db_extension`
  reste à côté tant que ce n'est pas prouvé.
- macOS : pas avant que Windows soit tranché.

## Comment reprendre

1. Si l'alpha est publiée : jouer `demo.sh` sans `DEMO_SOURCE` (vraie
   installation depuis npm) et lire ce que Lucie en dit ; sinon, la
   répétition par `DEMO_SOURCE=<archive>`.
2. Après la fusion de `embarqueur-absent` dans master : rebase de
   `paquet-npm` (la fusion est déjà faite dans la branche, le rebase doit
   être vide ou presque), puis fusion de `paquet-npm` dans master quand
   l'orchestration le dit.
3. Windows : l'extension vecteur dans le job, `Swatinem/rust-cache` (47 min
   → moins), l'épreuve JS sur le runner, le sous-paquet
   `rag3weaver-win32-x64-msvc`. macOS : x64 (`macos-15-large` ou
   `macos-13`), puis les sous-paquets `darwin-arm64` / `darwin-x64`.
4. Le vecteur en statique (dire à l'arbre principal avant de toucher au
   manifeste) ; `release.yml` copié de lucivy, porte fermée.

## Pièges

- Ne jamais corriger un script shell pendant qu'il tourne : bash le relit à
  l'offset et trébuche (vu ce matin).
- Le scratchpad est en mémoire vive : ses copies sont sous
  `~/.cache/rag3weaver-build/paquet-npm/sauvegarde-scratchpad/`.
- `gh workflow run` exige que le workflow existe sur master ; la recette
  qu'il appelle est prise sur la branche dispatchée.
- Le sous-module `third_party/fuzzy-fst` est privé et ne sert qu'à
  l'extension `fts` : ne pas l'initialiser sur un runner.
