# Le mode pile de la garde n'a pas l'exception C++ transférable

- **État** : ouvert (18 novembre 2026, hotfixs, au mot de c7)
- **Famille** : produit
- **Gravité** : une pile de lots C++ ne peut pas passer en mode pile dès qu'un lot Rust a atterri
  sous elle ; il faut la pousser d'un seul coup depuis sa tête
- **Atteignable en service** : non (outillage du push)
- **Touche rag3weaver** : non (`tools/poste/push_garde.sh`)
- **Pour** : G

## Ce qu'on voit

Le 18 novembre au soir, la pile C++ de la nuit (`add-column-under-readers-2`,
`dropped-column-under-old-reader-2`, `insert-after-forced-copy-keeps-rows-scanned-2`,
`create-index-reads-its-cached-column-3`) devait passer en mode pile, un lot après l'autre, après les
lots Rust de la file.

En mode pile, `verifier_pile` exige (c) : `code_apporte(<tête d'avant>, master)` vide. Or
`code_apporte` regarde `extension/rag3weaver`, `tools/rust_api`, `extension/vector` et `Cargo.lock`.
Un lot Rust atterri après la tête d'avant le rend non vide, et la garde s'arrête sur « la pile a
changé ». Hors du mode pile, la même situation passe par `cpp_transferable` : les suites C++ ne lient
pas le Rust, et un lot qui ne touche que le moteur, sur une base où master n'a rien changé sous le
moteur, ne rejoue pas. Le contrôle (c) du mode pile n'a pas cette exception.

## Ce qui a été fait ce soir-là

La pile est passée en un seul push de sa tête, en mode normal, par la voie transférable (base
094ad1fc4, décision de c7).

## Le correctif attendu

Dans `verifier_pile`, tenir le contrôle (c) pour vide quand `cpp_transferable "$avant" "$maitre"
"$teste"` est vrai : la pile ne touche que le moteur, et master n'y a rien changé depuis la tête
d'avant, sinon par les lots d'avant eux-mêmes, que le contrôle (a) juge déjà par `range-diff`. Un
témoin : une pile C++ de deux lots, un lot Rust atterri entre les deux, le second passe en mode pile.
