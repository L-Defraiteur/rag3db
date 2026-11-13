# Les tests lancés hors de run_e2e.sh laissent leurs bases dans /tmp, qui est de la mémoire vive

- **État** : ouvert (18 octobre 2026, seconde session cœur C++, à la demande de l'orchestration) ; le 8 novembre, le cache d'index quitte /tmp et le témoin de suite existe (session recherche), restent cinq suites qui laissent des résidus à chaque passe et trois, intermittentes, en rouge connu du témoin.
- **Gravité** : confort, mais `/tmp` est un tmpfs sur le poste principal : ce qui y reste occupe
  la RAM, puis l'échange.
- **Atteignable en service** : non (tests seulement), à une exception près : `Catalog` écrit son
  cache par défaut dans `/tmp/rag3weaver_cache`, en service aussi.
- **Touche rag3weaver** : oui (ses e2e, ses tests et `Catalog`).
- **Famille** : produit
- **Pour** : recherche (suivi et lot ; toute réattribution passe par l’orchestration).
- **Signalé par** : recherche ; sources et observations dans ce ticket et le [rapport de session](../11-11-2026/01-rapport-de-session.md).

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

**Effacés le 18 octobre** (seconde session cœur C++, à la demande de l'orchestration) :

- **Ce qui est parti** : les trois familles des 11 et 12 octobre, soit 4 033 entrées (2 332
  `rag3weaver-acces-*`, 1 610 `rag3weaver-rag3daemon-*`, 91 `rag3weaver-bulk-*`). Toutes
  appartenaient à l'uid 1000.
- **Comment** : d'abord la preuve que rien ne les tenait (rien dans `/proc/*/fd`, `cwd` et `maps`,
  ni par `fuser`, ni par `lsof`). Puis l'effacement depuis une liste écrite dans un fichier,
  jamais par un motif.
- **Leur poids** : 141 Mio au `du` de la liste, 144 Mio rendus au `df` de `/tmp`. C'est faible,
  ce qui **écarte cette piste pour l'échange** : les 10 Go relevés le 15 ne venaient pas de ces
  restes (voir le ticket de la mémoire de l'hôte).
- **Ce qui reste** : `rag3weaver-lignes-*` et `rag3weaver_cache`, hors du périmètre fixé, ainsi
  que les dossiers des sessions. `/tmp` compte maintenant 92 entrées.

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
   désigne. Ceux de `e2e_rag3daemon` et `e2e_code` ont été effacés le 18 octobre.

## Le 8 novembre 2026 (session recherche, lot residus-de-suite)

- **Le cache d'index de `Catalog` quitte /tmp, en service comme en test** : `catalog::index_cache_dir`
  rend `RAG3WEAVER_CACHE_DIR` s'il est donné, sinon `<cache XDG>/rag3weaver/cache` ; `temp_dir()`
  seulement sans dossier de cache. À l'ouverture d'un catalogue, une fois par processus et par
  dossier, les dossiers `<pid>` des processus morts y sont effacés (`sweep_dead_index_caches`,
  par `daemon_lifecycle::pid_state` : un pid à l'état illisible reste). Sous run_e2e.sh, chaque
  suite a le sien, sur le disque, effacé avec elle. Le 8 novembre à midi, `/tmp/rag3weaver_cache`
  du poste principal tenait 96 dossiers de pid, 119 Mo.
- **Le témoin de suite** : run_e2e.sh regarde le TMPDIR neuf de chaque suite avant de l'effacer et
  dit ce qu'elle y laisse ; hors de `tests/e2e/suites_qui_laissent_des_residus.txt`, la suite est
  rouge ; une suite de la liste qui finit propre (passe entière) est rouge aussi. La liste maigrit
  à chaque lot qui rend une suite propre, et ce ticket le dit.
- **La mesure de départ** (passe entière de master avec le témoin, liste vide, luciepc) : 70 suites
  laissaient des résidus, dont 64 le seul `rag3weaver_cache`. Restent, une fois le cache sorti :

  | suite | ce qu'elle laisse |
  |---|---|
  | e2e_code | `rag3weaver-bulk-<pid>` et ses `.extensions`, `.fts`, `.readers` (29 Mo) |
  | e2e_juge | deux dossiers `.tmp*` de tempfile |
  | e2e_lectures | un dossier `.tmp*` de tempfile (44 Ko) |
  | e2e_prise_atomique | 36 entrées : `rag3weaver-catalogue-lecteur-*`, `-insistant-*`, `-lecteurs-*`, `-marque-*`, `-modeles-*` (65 Mo) |
  | e2e_rag3daemon | `rag3weaver-acces-<pid>-<n>` et ses `.ecrivain`, `.readers`, `.wal` |
  | e2e_repli_de_branche | `rag3weaver-repli-hybride-<pid>` et ses fichiers (24 Mo) |
  | e2e_rouvrir | `rag3weaver-rouvrir-<pid>` et ses fichiers (8 Mo) |

  Pendant cette mesure, `e2e_banc_qualite` a paniqué faute de granite-278m sur 127.0.0.1:7878 :
  c'est la panne de ce service à luciepc, de 10 h 47 à 12 h 07, déjà en ticket (f66ad4449, cause
  illisible, journaux persistants posés). Rien à voir avec les résidus, et rien à chercher ici.

  Trois passes par suite ont montré que trois d'entre elles vont et viennent : e2e_cited_by
  3/3, e2e_juge 3/3, e2e_lectures 0/3. Leur résidu est un TempDir dont l'effacement a buté (un fil du
  backend écrit encore après son Drop) : ticket
  `2026-11-08-un-backend-ecrit-encore-dans-son-dossier-apres-son-drop.md`. Elles sont dans la liste
  avec ce ticket, en rouge connu du témoin (« résidu connu : … », propre reste propre). Les cinq
  autres (e2e_code, e2e_prise_atomique, e2e_rag3daemon, e2e_repli_de_branche, e2e_rouvrir) en
  laissent à chaque passe : propres, elles sont rouges.

  La branche de l'équipier `agent/tests-bases-dans-tmp` (relue par hotfixs, qui la complète) vise
  une partie de ces suites : chaque suite qu'elle rend propre sort de la liste dans son commit.

- **Le 9 novembre 2026** (lot arret-du-backend) : e2e_cited_by et e2e_lectures sortent de la liste,
  leur cause corrigée (un backend qu'on laisse tomber attend ses fils, ticket fermé
  `closed/2026-11-08-un-backend-ecrit-encore-dans-son-dossier-apres-son-drop.md`). e2e_juge reste en
  rouge connu, une autre cause (`2026-11-09-le-journal-des-runs-ecrit-apres-l-arret-de-son-hote.md`),
  avec les cinq suites qui laissent leurs résidus à chaque passe.

- **Le 13 novembre 2026** (lot journal-apres-arret) : e2e_juge sort de la liste, sa cause corrigée
  (l'arrêt de l'hôte écrit les journaux en attente et attend leurs fils, ticket fermé
  `closed/2026-11-09-le-journal-des-runs-ecrit-apres-l-arret-de-son-hote.md`). Restent les cinq
  suites de dette fixe et e2e_tx_par_paquet_arret en rouge connu (le CSV d'un COPY tué).
