# Le cargo du bac à sable hérite de RUSTC_WRAPPER, et sccache y échoue

- **État** : ouvert (19 novembre 2026), relevé à la première passe sous sccache
- **Famille** : produit
- **Gravité** : une utilisatrice qui compile par sccache (RUSTC_WRAPPER=sccache, réglage courant) ne peut plus faire
  jouer `cargo` par l'agent dans le bac à sable ; ce soir, il a fallu retirer sccache des postes
- **Atteignable en service** : oui, dès que l'environnement de l'hôte porte RUSTC_WRAPPER
- **Touche rag3weaver** : oui

## Ce qui s'est passé

Le 19 novembre à 22 h 03, `poste` s'est mis à exporter `RUSTC_WRAPPER=sccache` (lot campagne-incremental-off). Le
premier train rust du pousseur sous cette chaîne, recherche-filtre-vide, a rougi sur deux tests de la lib :
`commande::tests::cargo_plays_its_tests_inside_the_sandbox` et
`daemon_host::teammate_tests::a_task_plays_cargo_test_through_the_gate`. Tous deux lancent `cargo` dans le bac à sable de
`commande`, qui hérite de l'environnement :

```
error: process didn't exit successfully: `sccache …/rustc -vV` (exit status: 2)
sccache: error: Permission denied (os error 13)
```

Dans le bac à sable, sccache ne peut ni écrire son cache (`~/.cache/sccache`) ni joindre son serveur.

Journal : luciepc, `~/.cache/rag3weaver-build/pusher-20261119-225609-014-recherche-filtre-vide/lib.log`.

## Ce qui a été fait

À 23 h 2x, `poste` a été remis à sa copie d'avant sccache aux deux postes, et la file mise en pause le temps de
départager. Les lots rougis par la chaîne sont redéposés.

## Pour le correctif

Le bac à sable lance `cargo` avec un environnement nettoyé de `RUSTC_WRAPPER`, `RUSTC_WORKSPACE_WRAPPER` et `SCCACHE_*`,
ou bien il ouvre à sccache son dossier et son serveur. Le premier est plus simple et ne change rien au sens des tests.
Un témoin rouge d'abord : `RUSTC_WRAPPER` pointé vers un faux enveloppeur qui échoue, puis `cargo` dans le bac à sable,
qui doit passer. Ensuite seulement, sccache se repose dans `poste`.
