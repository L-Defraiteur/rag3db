# Optimiseur — rapport de session

**Mis à jour le 10 octobre 2026 à 12 h 30, à la pause pour le redémarrage du
poste.** Chantier G, le paquet npm de rag3weaver, depuis le matin ; cadrage :
`../orchestration/03-le-paquet-npm.md`. Le rapport précédent (envois à
tracel-ai, modèles de décision) est dans `../../3-octobre-2026-23h31/optimiseur/`.

## État à la pause

- Branche `paquet-npm`, worktree `../rag3db-paquet-npm` ; tout est commité
  et poussé. Rien n'est publié, rien dans l'arbre principal.
- Aucun bâti local en cours (celui qui tournait a été arrêté, le target de
  cargo sur disque reprend là où il en était).
- Un essai Windows continue seul sur les runners GitHub :
  https://github.com/L-Defraiteur/rag3db/actions/runs/38044225912 (huitième).

## Ce qui est fait

Tout est dans `../../optimiseur/10-octobre-2026-11h53/01-le-premier-bati.md`,
en une phrase chacun :

- **Linux x86_64 bâti dans un Docker commité** (`tools/build-images/linux-x64-gnu/`,
  base manylinux_2_28) : binaire de 79 Mo après strip, extension vecteur de
  684 Ko, glibc 2.28 au plus, 14 à 20 min à froid sur un runner à 4 cœurs,
  10 min en natif ici. Il démarre, charge un backend de code avec Landlock,
  **y compris dans un conteneur Docker**, et refuse en le disant sans service
  d'embarquement.
- **Windows x64 sur `windows-latest`** : sept essais, la liste des accrocs
  est faite. Quatre contournés dans le workflow (link.exe de Git, drapeau GCC
  de tree-sitter-scss → clang-cl, C et C++ tous deux en clang-cl, /EHsc) ;
  quatre qui touchent nos sources, appliqués sur la branche pour l'essai et
  relayés par l'orchestration à leurs propriétaires (landlock pour Linux
  seulement ; NodeTableDeleteState sans copie ; pas de -latomic pour clang
  sous Windows ; la crate rag3db ne bâtit que la cible statique). Le moteur
  C++ compile entièrement sous clang-cl ; le huitième essai dit si le binaire
  se lie.
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

1. Lire le résultat du huitième essai Windows (`gh run view 38044225912`,
   compte `L-Defraiteur` par `GH_TOKEN`) : si le binaire existe, l'artefact
   `bati-windows-x64-journal` le porte ; essayer `--describe` sur un gabarit
   avec `"sandbox": {"mode": "off"}` ; sinon, lire l'erreur d'édition de
   liens de l'exécutable.
2. Vérifier sous Linux que `build_target("rag3db")` n'a rien changé :
   `poste lourd tools/build-images/linux-x64-gnu/build.sh` depuis le
   worktree (target sous `~/.cache/rag3weaver-build/paquet-npm/`).
3. Premier bâti dans Docker, ici : `tools/build-images/build.sh linux-x64-gnu`
   (il passe par `poste lourd`), comparer `dist/linux-x64-gnu/bati.txt` au
   relevé du runner.
4. Puis le paquet JS (`rag3weaver` + `rag3weaver-linux-x64-gnu`, sur le
   modèle de `lucivy/bindings/nodejs` : un `index.js` qui choisit le paquet
   de la plateforme, un sous-paquet par cible avec `os`/`cpu`/`libc`) : il
   lance `rag3weaver-backend backend.json` et lui parle en lignes JSON sur
   stdin/stdout (`describe`, `call`, `journal`, `journal_read`,
   `index_state`, `shutdown`) ; test « dossier vide, trois fichiers, une
   recherche » ; publication fermée.

## Pièges

- Ne jamais corriger un script shell pendant qu'il tourne : bash le relit à
  l'offset et trébuche (vu ce matin).
- Le scratchpad est en mémoire vive : ses copies sont sous
  `~/.cache/rag3weaver-build/paquet-npm/sauvegarde-scratchpad/`.
- `gh workflow run` exige que le workflow existe sur master ; la recette
  qu'il appelle est prise sur la branche dispatchée.
- Le sous-module `third_party/fuzzy-fst` est privé et ne sert qu'à
  l'extension `fts` : ne pas l'initialiser sur un runner.
