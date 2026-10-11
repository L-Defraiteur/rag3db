# Optimiseur — rapport de session

**Mis à jour le 11 octobre 2026 à 4 h 50.** Chantier G, le paquet npm de rag3weaver, depuis le matin ; cadrage :
`../orchestration/03-le-paquet-npm.md`. Le rapport précédent (envois à
tracel-ai, modèles de décision) est dans `../../3-octobre-2026-23h31/optimiseur/`.

## État

- Branche `paquet-npm` (worktree `~/git_workspaces/rag3db-paquet-npm`,
  target `~/.cache/rag3weaver-build/target-G`), tête 9888509f6 ; la tête de
  la séance est **634c52249** (master 6d7f2a811 fondu : le filet
  `force_checkpoint_on_copy=true`, le contrôle de bâti des extensions, le
  lot 1 du chantier I, le rechargement des déclarations). Reste à fondre :
  master 60d74a4e2 (le correctif du cœur ; A retire le filet après) et
  fc636f7e6 (le refus de schéma qui ne recopie plus la donnée).
- **Les quatre archives `0.0.1-alpha.2` de 634c52249 sont prêtes** sous
  `~/.cache/rag3weaver-build/paquet-npm-garde/publier/`, épreuve et sonde
  vertes sur les trois plateformes, identifiants de bâti égaux moteur /
  extension partout, démo verte ; l'orchestration a les quatre commandes
  (linux, windows, darwin, tête). **Rien n'est publié**, l'OTP est à Lucie.
  Les archives de 786dfec13 (COPY journalisé défectueux) et d'avant restent
  à côté, étiquetées.
- Le chantier a appris cette nuit : `[profile.paquet]` (LTO complète), les
  trois correctifs Windows (chemin cité pour Cypher, fsync de dossier, fsync
  de fichier ouvert en écriture), les journaux hors du checkout,
  l'identifiant de bâti vérifié dans les recettes, la sonde du COPY (qui ne
  reproduit pas le défaut : COPY de 512 lignes, un seul groupe).
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

1. La séance de publication avec Lucie (quatre `npm publish` dans l'ordre
   linux, windows, darwin, tête, depuis `paquet-npm-garde/publier/`), puis
   `npm view rag3weaver@next version optionalDependencies` et la démo sans
   `DEMO_SOURCE`.
2. Fondre master (60d74a4e2 et la suite, fc636f7e6), rejouer les trois bâtis
   (Windows et macOS changent de clé de cache dès que cmake/ ou src/ bougent),
   vérifier les identifiants ; l'alpha suivante avec le correctif du cœur.
3. Fusion de `paquet-npm` dans master quand l'orchestration le dit ; puis
   effacer target-G, le worktree, le clone luciepc.
4. Ensuite : `fetch-depth: 0` sur les runners (identifiants alignés),
   macOS x64 (à Lucie), le vecteur en statique, `release.yml` copié de
   lucivy avec porte fermée.

## Pièges

- Ne jamais corriger un script shell pendant qu'il tourne : bash le relit à
  l'offset et trébuche (vu ce matin).
- Le scratchpad est en mémoire vive : ses copies sont sous
  `~/.cache/rag3weaver-build/paquet-npm/sauvegarde-scratchpad/`.
- `gh workflow run` exige que le workflow existe sur master ; la recette
  qu'il appelle est prise sur la branche dispatchée.
- Le sous-module `third_party/fuzzy-fst` est privé et ne sert qu'à
  l'extension `fts` : ne pas l'initialiser sur un runner.
