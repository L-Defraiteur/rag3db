# La voie C++ de la garde ignore `tools/shell` et le reste de `tools/` compilé

- **État** : ouvert (16 novembre 2026, hotfixs, au mot de c7)
- **Famille** : produit
- **Gravité** : un lot qui touche le code du shell (`tools/shell/*.cpp`) n'entre dans aucune voie de la garde. Le mode
  tests seuls le refuse, puisque ce n'est pas `tools/shell/test`, et la voie C++ ne compte pas `tools/shell` parmi
  ses chemins. Il ne passe que si master n'a apporté aucun code depuis sa base.
- **Atteignable en service** : non (outillage du push)
- **Touche rag3weaver** : non (`tools/poste/push_garde.sh`)
- **Pour** : G

## Ce qu'on voit

Le 16 nov., le lot `shell-pipe-input-keeps-bytes` (`tools/shell/linenoise.cpp` et sa suite) est passé par la garde
ordinaire. Master n'avait rien apporté de code depuis sa base, et un journal de la passe nommait la lib : le BUILD_ID
que le shell joué a écrit lui-même (`CALL BUILD_ID()`). Si master avait apporté du Rust entre-temps, la garde
aurait demandé un rejeu, alors que le shell ne lie pas le Rust.

## Pour fermer

`tools/shell/` (et le reste de `tools/` qui se compile avec le moteur) entre dans les chemins de la voie C++
transférable, et la lib d'un tel lot est le BUILD_ID qu'écrit le shell joué. Un témoin de script : un lot qui ne
touche que `tools/shell/`, sur un master qui n'a changé que du Rust, passe sans rejeu.
