# Optimiseur — rapport de session

**Mis à jour le 10 octobre 2026 en fin d'après-midi.** Chantier G, le paquet npm de rag3weaver, depuis le matin ; cadrage :
`../orchestration/03-le-paquet-npm.md`. Le rapport précédent (envois à
tracel-ai, modèles de décision) est dans `../../3-octobre-2026-23h31/optimiseur/`.

## État

- Branche `paquet-npm`, worktree `../rag3db-paquet-npm` ; tête f86a225fe
  plus les deux portes du service optionnel (en bâti natif, à commiter).
  Rien n'est publié, rien dans l'arbre principal.
- **La branche ne fusionne pas avant le lot « embarqueur absent » de l'arbre
  principal** ; il est écrit (branche `embarqueur-absent`, c98d4ff9b) et le
  témoin est vert dessus, sur une branche d'essai locale (`paquet-npm-absent`,
  non poussée) — patch de la porte sous
  `~/.cache/rag3weaver-build/paquet-npm/porte-absent.patch`, à poser sur
  `paquet-npm` au rebase quand c'est sur master. `npm test` reste rouge
  attendu derrière `RAG3WEAVER_ESSAI_ROUGE_ATTENDU=1` jusque-là.
- **Windows est tranché** : le binaire se lie (dixième essai, MSVC pur,
  60 Mo) et `--describe` passe avec le bac à sable fermé (quinzième,
  https://github.com/L-Defraiteur/rag3db/actions/runs/38060705890) ; entre
  les deux, trois accrocs d'épreuve et un vrai (le chemin verbatim de
  `canonicalize` sous Windows, corrigé). Reste pour le sous-paquet Windows :
  l'extension vecteur, le cache de bâti, l'épreuve JS sur le runner.
- L'épreuve JS passe de bout en bout sur le binaire natif porteur des
  portes ; elle reste derrière sa porte (voir le relevé).

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

1. Windows, la suite (quand l'orchestration le demande) : l'extension
   vecteur dans le job Windows, `Swatinem/rust-cache`, l'épreuve JS sur le
   runner, le sous-paquet `rag3weaver-win32-x64-msvc`. macOS peut maintenant
   se décider (règle de Lucie : pas avant que Windows soit tranché).
2. Quand l'embarqueur absent est sur master : rebase de `paquet-npm`
   (conflits connus : node_table.h → prendre master, paquet-npm.yml → garder
   la branche), `git apply` du patch de la porte, rebâti, `RAG3WEAVER_BACKEND=<binaire> RAG3WEAVER_ESSAI_ROUGE_ATTENDU=1 npm test`
   dans `bindings/nodejs`, retirer la porte du rouge attendu, puis fusion.
   La branche locale `paquet-npm-absent` (c88b812ec) garde le tout déjà joué.
3. Premier bâti dans Docker, ici : `tools/build-images/build.sh linux-x64-gnu`
   (il passe par `poste lourd` ; `lucied` doit être dans le groupe docker),
   comparer `dist/linux-x64-gnu/bati.txt` au relevé du runner et de luciepc.
4. Puis le vecteur en statique ; macOS après Windows ; release.yml copié de
   lucivy, porte fermée.

## Pièges

- Ne jamais corriger un script shell pendant qu'il tourne : bash le relit à
  l'offset et trébuche (vu ce matin).
- Le scratchpad est en mémoire vive : ses copies sont sous
  `~/.cache/rag3weaver-build/paquet-npm/sauvegarde-scratchpad/`.
- `gh workflow run` exige que le workflow existe sur master ; la recette
  qu'il appelle est prise sur la branche dispatchée.
- Le sous-module `third_party/fuzzy-fst` est privé et ne sert qu'à
  l'extension `fts` : ne pas l'initialiser sur un runner.
