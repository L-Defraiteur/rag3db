# Un target neuf ne se bâtit plus sous openssl 4

- **État** : fermé (14 novembre 2026) par le lot `openssl-sys-pour-openssl-4` ; relevé par archi
- **Famille** : produit
- **Pour** : archi
- **Gravité** : bloquant — tout target neuf, ou après un `cargo clean`, échoue à bâtir
  `openssl-sys` 0.9.111 ; les targets déjà bâtis continuent
- **Atteignable en service** : non pour le binaire (le TLS y passe par rustls, rien ne lie libssl ni
  libcrypto, relevé de G par `ldd` et `/proc/<pid>/maps`) ; oui pour qui bâtit le crate sur un poste
  à openssl 4, CI comprise
- **Touche rag3weaver** : oui (`extension/rag3weaver/Cargo.toml`, `Cargo.lock`)

## Ce que c'est

La mise à jour du système des deux postes, au `pacman.log` :

```
[2026-11-14T13:51:47+0100] [ALPM] upgraded openssl (3.6.5-1.1 -> 4.1.0-1.1)
[2026-11-14T13:51:48+0100] [ALPM] installed openssl-3 (3.5.9-2.1)
```

`openssl-sys` 0.9.111 refuse alors de se bâtir : « This crate is only compatible with OpenSSL
(version 1.0.2 through 1.1.1, or 3) ». `cargo tree -i openssl-sys --all-features` dit qui le tire,
toujours par `native-tls` et sans que rien ne s'en serve :

- `hf-hub` 0.4 (optionnel, `candle-embedder` et `bge-m3`), par ses features par défaut
  (`default-tls`) ; le code n'appelle que `hf_hub::api::sync` (feature `ureq`, dont la TLS est
  rustls) ;
- `reqwest` 0.12 des dev-dependencies, par ses features par défaut (`default-tls`).

## Ce qui est attendu

Aucune dépendance ne tire `native-tls` ni `openssl-sys` : `hf-hub` sans ses features par défaut
(`features = ["ureq"]`), `reqwest` des témoins en `rustls-tls`. Rien à installer ni à poser sur le
poste, la CI et les utilisateurs compris. Un `openssl-sys` 0.9.118 connaît openssl 4, mais garder
openssl hors du bâti vaut mieux qu'en suivre les versions.

Témoin : `le_tls_ne_tire_pas_openssl::the_lockfile_pulls_neither_openssl_nor_native_tls`, et un
target neuf qui se bâtit sous openssl 4.1.0 sans variable `OPENSSL_*` ni `PKG_CONFIG_PATH`.

## Ce qui part du lock

Douze paquets, aucune version ne monte : `openssl-sys`, `openssl`, `openssl-macros`, `native-tls`,
`hyper-tls`, `tokio-native-tls`, `foreign-types`, `foreign-types-shared`, `core-foundation`,
`system-configuration`, `system-configuration-sys`, `windows-registry`.
