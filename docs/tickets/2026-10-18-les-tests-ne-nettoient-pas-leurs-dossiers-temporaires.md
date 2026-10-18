# Les tests lancés hors de run_e2e.sh laissent leurs bases dans /tmp, qui est de la mémoire vive

- **État** : ouvert (18 octobre 2026, seconde session cœur C++, à la demande de l'orchestration).
- **Gravité** : confort, mais `/tmp` est un tmpfs sur le poste principal : ce qui y reste occupe
  la RAM, puis l'échange.
- **Atteignable en service** : non (tests seulement), à une exception près : `Catalog` écrit son
  cache par défaut dans `/tmp/rag3weaver_cache`, en service aussi.
- **Touche rag3weaver** : oui (ses e2e, ses tests et `Catalog`).

## Ce que c'est

Le 11 octobre, la garde de `run_e2e.sh` a donné à chaque suite e2e son propre `TMPDIR`, effacé
derrière elle (`extension/rag3weaver/docs/11-octobre-2026-recherche/03-les-e2e-en-un-binaire.md`,
§4). Le même jour, les tests gtest du moteur ont reçu leur parent par passe (ticket fermé
`closed/2026-10-11-les-tests-gtest-du-moteur-laissent-leurs-bases-dans-tmp.md`).

Ces gardes ne couvrent qu'un test lancé par `run_e2e.sh`. Une suite jouée à la main
(`cargo test --test e2e`, une boucle de rejeu, un script de session) écrit à la racine de `/tmp`,
et ses `remove_dir_all` échouent comme avant le 11. Le 15 octobre, l'orchestration a effacé
**755 dossiers morts** de `/tmp` (familles `rag3weaver-bulk-*`, `rag3weaver_cache`,
`/tmp/rag3db`). L'échange du poste est passé de 22 à 10 Go, avec l'arrêt de l'hôte : voir
[le ticket de la mémoire de l'hôte](2026-10-18-l-hote-ne-rend-pas-la-memoire-d-un-gros-index.md).

## Ce qui reste le 18 octobre (poste principal, relevé en lecture seule)

4 124 entrées à la racine de `/tmp`. Les familles de rag3weaver :

| famille | entrées | dates | poids | qui l'écrit |
|---|---|---|---|---|
| `rag3weaver-acces-<pid>-<ns>` (+ `.wal`, `.readers`) | 2 332, soit 804 bases | 11 oct. 13 h 42 → 12 oct. 14 h 45 | 6,3 Mo | `e2e_rag3daemon::un_lecteur_choisit_son_chemin_et_le_dit` |
| `rag3weaver-rag3daemon-<pid>-<ns>` (+ `.wal`) | 1 610 | même plage | 13 Mo | `e2e_rag3daemon` |
| `rag3weaver-bulk-<pid>` | 91 | 11 oct. 10 h 54 → 12 oct. 16 h 28 | 122 Mo | `e2e_code` (`tests/e2e_code.rs:738`) |
| `rag3weaver-lignes-*` | 14 | 11 → 12 oct. | 0,2 Mo | (non cherché) |
| `rag3weaver_cache` | 1 dossier | — | 16 Mo | `Catalog` (`src/catalog.rs:643`, `cache_base` par défaut) |

`/tmp/rag3db` est vide aujourd'hui : le parent par passe du moteur tient.

Les restes de `e2e_rag3daemon` viennent d'environ 800 processus distincts (deux bases par pid),
tous **après** la garde de `run_e2e.sh` (11 octobre, 12 h 27) : ils ont donc été lancés sans elle.
On ne sait pas encore qui les a lancés (une boucle de rejeu, sans doute). Ces restes pèsent peu.
Le volume vient des bases d'index des suites `e2e_code`, et de 34 fichiers de tests ou de `src/`
qui écrivent sous `std::env::temp_dir()`.

## La cause

Elle est la même qu'au 11 octobre :

- `let _ = std::fs::remove_dir_all(&dossier)` est appelé sur une **base**, qui est un fichier
  (`e2e_rag3daemon.rs:181`, `:294`, `e2e_code.rs:739`). L'appel échoue, `let _` avale l'erreur,
  et la base reste, avec son `.wal` et son `.readers` à côté.
- Rien n'efface `/tmp/rag3weaver_cache` (le `cache_base` par défaut de `Catalog`).

La garde de `run_e2e.sh` masque ces défauts sans les corriger.

## Témoin

Aucun pour l'instant. Il faudrait :

1. jouer `e2e_rag3daemon` et `e2e_code` **sans** `run_e2e.sh`, avec un `TMPDIR` neuf ;
2. vérifier qu'il n'y reste rien.

## Pour le fermer

1. Remplacer le `remove_dir_all` sur une base par l'effacement de la base et de ses voisins
   (`.wal`, `.readers`), ou créer chaque base dans un dossier à elle et effacer ce dossier. Le
   faire partout où un test crée une base sous `temp_dir()`.
2. Faire que l'effacement ait lieu même quand le test échoue, par une garde à `Drop`, sauf avec
   `RAG3WEAVER_GARDER=1`.
3. Donner à `Catalog` un `cache_base` de test dérivé de `TMPDIR`, ou l'effacer à la fermeture
   quand il est temporaire.
4. Une fois rien de vivant dessus (vérifié par `/proc/*/cwd`, `fd` et `maps`, jamais par nom),
   effacer les restes listés plus haut. L'effacement revient à qui Lucie ou l'orchestration
   désigne.
