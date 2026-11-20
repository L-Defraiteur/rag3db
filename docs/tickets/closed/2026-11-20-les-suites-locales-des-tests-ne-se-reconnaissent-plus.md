# Les suites « locales » des tests ne se reconnaissent plus : elles passent par le service, puis par le repli

- **État** : corrigé (21 novembre 2026, lot `local-suites`, master fd7a80df79) ; ouvert (20 novembre 2026, embarquement ; relevé en triant les tests qui comparent des valeurs)
- **Pour** : embarquement
- **Famille** : produit (les tests du crate)
- **Gravité** : moyenne. Des suites qui disent éprouver le modèle sur la carte d'ici éprouvent le service, ou le
  modèle chargé sur le périphérique du repli (`igpu:0` à luciepc, qui calcule faux en lot : ticket
  `2026-11-20-sur-l-igpu-granite-rend-des-vecteurs-faux-en-lot`).
- **Atteignable en service** : non, tests seulement
- **Touche rag3weaver** : oui (`tests/common/mod.rs`)

## Ce que c'est

`tests/common/mod.rs`, `suite_locale()` : une suite reste sur la carte d'ici si le nom du binaire de test commence par
l'un des préfixes de `SUITES_LOCALES` (`e2e_burn_`, `e2e_demon_embeddings`, `e2e_mesure_ingestion_code`,
`e2e_banc_bge_m3`). Or, depuis le 11 octobre, tous les `tests/e2e_*.rs` sont des modules d'un seul binaire, `e2e`
(`Cargo.toml` : `autotests = false`, `[[test]] name = "e2e"`, `path = "tests/e2e/main.rs"`). Le binaire s'appelle
`e2e-<hash>` : aucun préfixe ne correspond, jamais.

Lu dans le code au 20 novembre (origin/master), pas encore vu en passe : à confirmer par un témoin.

## Ce qu'il faudrait

- Reconnaître la suite par le **chemin du module du test** (`e2e_burn_granite::…`), pas par le binaire : le nom du test
  en cours (`std::thread::current().name()`) le porte, sous le harnais par défaut.
- Un témoin rouge d'abord : une sonde dans un module `e2e_burn_…` qui dit si elle se charge ici ou par le service.

## Fermé (21 novembre 2026)

- **Le correctif** : `suite_locale()` lit le nom du test en cours (`std::thread::current().name()`, qui porte
  `<suite>::<test>` sous libtest), et non plus celui du binaire `e2e-<hash>`. `run_e2e.sh` lance un processus par suite :
  un modèle chargé une fois appartient à la suite de ce processus.
- **Les témoins**, vus rouges puis verts à la tablette : `e2e_burn_granite::this_suite_is_recognised_as_local` (rouge :
  le fil se nommait bien `e2e_burn_granite::…`, et la suite n'était pas reconnue), et
  `e2e_aide_des_tests_repli::this_suite_is_not_recognised_as_local`.
- **Dans le même lot** : `sonde_granite_107m` ne juge plus rien hors de sa relance, comme la sonde de valeurs. Jouée
  seule par une passe de la tablette, sans repli nommé et sans service de granite-107m, elle rougissait sans défaut du
  code.
