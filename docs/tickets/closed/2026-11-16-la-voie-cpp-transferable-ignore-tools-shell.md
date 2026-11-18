# La voie des suites C++ transférables ignore `tools/shell`

- **État** : fait (18 novembre 2026, branche shell-cpp-transferable) : tools/shell/ entre dans CHEMINS_MOTEUR et dans le case de cpp_transferable ; deux cas neufs au témoin (un lot du shell seul passe par la voie transférable, rouge avant ; un lot du shell avec du Rust reste arrêté), 53 vert
- **Famille** : produit (outillage : `tools/poste/push_garde.sh`)
- **Gravité** : temps perdu. Un lot du shell C++ rejoue ses suites quand seul du Rust a atterri.
- **Atteignable en service** : non (la garde du push)
- **Touche rag3weaver** : non
- **Pour** : la session qui reprend l'outillage de G (Codex)
- **Signalé par** : c7, à la passation du 16 novembre 2026

## Ce qu'on voit

`push_garde.sh` laisse passer sans rejeu un lot qui ne touche que le moteur quand master n'a apporté que du Rust
(`cpp_transferable`, ligne `CHEMINS_MOTEUR="src/ test/ extension/vector/ tools/rust_api/"`, et le `case` qui suit).
`tools/shell/` (le shell C++ : `include/`, `printer/`, les sources, et `tools/shell/test/`) n'y est pas. Un lot du shell
est donc refusé (« master a apporté du code depuis la base testée ») dès qu'un lot Rust atterrit avant lui. Le shell ne
lie que la lib du moteur, jamais le crate Rust.

## Ce qu'il faudrait

- `tools/shell/` entre dans `CHEMINS_MOTEUR` et dans le `case` de `cpp_transferable`.
- Le mode tests seuls (`TESTS_ONLY=1`, `tools/poste/tests_only_dirs.txt`) ne change pas.
- Témoin, dans `tools/poste/test_push_garde.sh`, rouge sur la garde d'avant : un lot qui ne touche que
  `tools/shell/x.cpp`, avec master qui n'apporte que `extension/rag3weaver/src/lib.rs`, passe par la voie transférable ;
  un lot `tools/shell/x.cpp` plus `extension/rag3weaver/src/y.rs` ne passe pas.
