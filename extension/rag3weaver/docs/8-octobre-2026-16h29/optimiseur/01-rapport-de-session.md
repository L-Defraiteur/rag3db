# Optimiseur — rapport de session

**Mis à jour le 11 octobre 2026 à 3 h 20.** Chantier G, le paquet npm de rag3weaver, depuis le matin ; cadrage :
`../orchestration/03-le-paquet-npm.md`. Le rapport précédent (envois à
tracel-ai, modèles de décision) est dans `../../3-octobre-2026-23h31/optimiseur/`.

## État

- Branche `paquet-npm` (worktree `~/git_workspaces/rag3db-paquet-npm`,
  target `~/.cache/rag3weaver-build/target-G`) : elle fond master
  97acd579b (contrôle de bâti des extensions, lot 1 du chantier I, rechargement
  des déclarations) ; tête de la séance **786dfec13**. Reste à fondre après
  la séance : master 274ce4fc5 (`RAG3DB_BUILD_ID_GIT_DIR`, dont build.rs
  pose déjà la variable sur la branche).
- **Les quatre archives `0.0.1-alpha.2` sont prêtes**, toutes de 786dfec13,
  sous `~/.cache/rag3weaver-build/paquet-npm-garde/publier/`, épreuve verte
  sur les trois plateformes, identifiant de bâti vérifié ; la démo rejouée
  dessus est verte. L'orchestration a les quatre commandes (linux, windows,
  darwin, tête) ; **rien n'est publié**, l'OTP est à Lucie.
- Les règles du chantier : `[profile.paquet]` (LTO complète, 61 Mo au lieu
  de 89), les trois correctifs Windows (chemin cité pour Cypher, fsync de
  dossier, fsync de fichier), journaux et copies de cache hors du checkout,
  identifiant de bâti vérifié dans les recettes.
- À la fusion de `paquet-npm` dans master (règle de Lucie) : effacer
  `target-G` et le worktree ici, le clone sur luciepc ; dire « effacé ».

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

1. La séance de publication (quatre `npm publish` dans l'ordre linux,
   windows, darwin, tête, depuis `paquet-npm-garde/publier/`), puis
   `npm view rag3weaver@next version optionalDependencies` et la démo sans
   `DEMO_SOURCE`.
2. Fondre master 274ce4fc5 dans `paquet-npm`, rejouer les trois bâtis (le
   cache Windows et macOS change de clé : cmake/ et CMakeLists.txt sont
   dans la clé), vérifier les identifiants.
3. Fusion de `paquet-npm` dans master quand l'orchestration le dit ; puis
   effacer target-G, le worktree, le clone luciepc.
4. Ensuite : macOS x64 (à Lucie), le vecteur en statique (amélioration
   commune), `release.yml` copié de lucivy avec porte fermée.

## Pièges

- Ne jamais corriger un script shell pendant qu'il tourne : bash le relit à
  l'offset et trébuche (vu ce matin).
- Le scratchpad est en mémoire vive : ses copies sont sous
  `~/.cache/rag3weaver-build/paquet-npm/sauvegarde-scratchpad/`.
- `gh workflow run` exige que le workflow existe sur master ; la recette
  qu'il appelle est prise sur la branche dispatchée.
- Le sous-module `third_party/fuzzy-fst` est privé et ne sert qu'à
  l'extension `fts` : ne pas l'initialiser sur un runner.
