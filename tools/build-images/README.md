# Images de bâti du paquet npm de rag3weaver

Un dossier par cible, avec un `Dockerfile` (les outils, rien d'autre) et un
`build.sh` (la recette : le moteur en statique, le binaire
`rag3weaver-backend`, l'extension vecteur, un relevé dans `dist/<cible>/bati.txt`).
Les images ne sont pas commitées, leurs recettes si. `build.sh` à la racine
bâtit une cible dans son image ; chaque `<cible>/build.sh` se joue aussi en
natif sur un poste qui a cmake, ninja et Rust.

Ce que Docker couvre : Linux. Windows et macOS se bâtissent sur un vrai
système (runners GitHub, `.github/workflows/paquet-npm.yml`), la CI ne
publiant qu'ensuite. Cadrage et décisions :
`extension/rag3weaver/docs/8-octobre-2026-16h29/orchestration/03-le-paquet-npm.md`.

| cible | image | état |
|---|---|---|
| `linux-x64-gnu` | manylinux_2_28 (glibc 2.28), gcc 14, Rust épinglé | recette écrite le 10 octobre 2026 ; chiffres dans `bati.txt` |
