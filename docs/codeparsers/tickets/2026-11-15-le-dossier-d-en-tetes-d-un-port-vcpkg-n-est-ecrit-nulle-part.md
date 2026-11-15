# Le dossier d'en-têtes d'un port vcpkg n'est écrit nulle part dans le registre

- **État** : ouvert (15 novembre 2026) — source à trouver
- **Famille** : produit
- **Gravité** : manque de rappel. `#include <boost/asio.hpp>` ne se relie pas au port `boost-asio` que
  déclare `vcpkg.json` : l'en-tête donne `boost`, le port `boost-asio`, et l'import reste `inconnue`.
  Rien n'est faux.
- **Atteignable en service** : oui, sur tout dépôt C/C++ dont un port vcpkg a un nom différent du dossier de
  ses en-têtes (`boost-*`, `nlohmann-json` → `nlohmann/`)
- **Touche rag3weaver** : oui (la provenance des en-têtes ; la table des renommages de C/C++)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, avec l'orchestration, au lot de conan et CMake

## Ce qu'on voit

Le registre `microsoft/vcpkg` ne dit pas le dossier d'en-têtes d'un port. Ses `ports/<port>/usage` disent le
paquet CMake et ses cibles (`nlohmann-json` → `find_package(nlohmann_json)`), `boost-asio` n'en a pas, et
le dossier n'apparaît qu'à l'installation du port. Les `portfile.cmake` l'emploient parfois
(`file(INSTALL … DESTINATION include/…)`), sans forme régulière.

## La règle d'ici là

Un port garde son nom tel quel : on ne coupe pas au tiret, ce serait une devinette (décision de
l'orchestration, 15 novembre).

## Ce qu'il faudrait

Une source qui dit, par port, les fichiers installés (une liste de fichiers par port publiée par un tiers,
ou une installation de référence relevée une fois avec son commit et sa date), d'où une table en données
`[distributions]` dans `cpp.toml`, comme pipreqs pour Python.
