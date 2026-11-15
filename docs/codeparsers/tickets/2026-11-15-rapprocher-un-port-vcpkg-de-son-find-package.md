# Rapprocher un port vcpkg de son `find_package`

- **État** : ouvert (15 novembre 2026) — souhait
- **Famille** : produit
- **Gravité** : basse. Rien n'est faux ; un rapprochement manque.
- **Atteignable en service** : le jour où un `CMakeLists.txt` déclare par `find_package` ce que
  `vcpkg.json` installe (`find_package(nlohmann_json)` et le port `nlohmann-json`)
- **Touche rag3weaver** : oui (`manifestes.rs`)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, avec l'orchestration, au lot de conan et CMake

## Ce qu'il faudrait

Une table port → paquet CMake, tirée une fois des `ports/<port>/usage` du registre `microsoft/vcpkg` par un
petit script livré avec elle, en données, avec sa source, son commit et sa date en tête (comme pipreqs). Le
registre la donne de façon fiable (`nlohmann-json` → `nlohmann_json`), à la différence du dossier
d'en-têtes (ticket du même jour).
