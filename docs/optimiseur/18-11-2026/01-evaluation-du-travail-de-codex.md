# Évaluation du travail de Codex dans le périmètre de G (optimiseur), du 16 au 17 novembre 2026

Session G (Claude), le 18 novembre 2026, à la demande de Lucie par c7 (« chaque session évalue la pertinence de ce
qu'a fait Codex »). Fenêtre lue : `git log 927941a67a..origin/master` et les branches `cx-optimiseur-*`. Chaque
verdict s'appuie sur une vérification faite aujourd'hui : l'état relu aux deux postes, les témoins rejoués et le code
relu ligne à ligne. Les citations de Codex ne comptent pas comme preuves.

## 0. La bascule, d'abord

| point | relu le 18 à 11 h 39 | verdict |
|---|---|---|
| lib `courant` | 59ac9c86bc aux deux postes, md5 697c7a0c / 925b6b13, les md5 de mon bâti du 16 | **faite, juste** |
| hôtes | md5 84d404d2 aux deux postes, bâtis sur la source 927941a67a (et non sur mon .nouveau d164e4da de 3177d2489e : 927941a67a apportait le lock lucivy 4.3.9) ; `/sante` nomme la lib 59ac9c86bc ; check-lib rc 0 aux deux postes | **faite, juste** |
| contrôle des deux sens | `controls-summary.json` aux deux postes : code-rag3db-depot, memoire-longue, code-lourd, code-principal, 18 tables chacune, **0 perdue** | **fait** |
| retour arrière | instantanés `avant-59ac9c86b-20261116-927` et leur `RETOUR.sh` aux deux postes ; la lib 373b5477b reste au magasin ; anciens binaires en `.avant-927941a6-59ac` | **encore possible** (Codex dit le garder 72 h au moins) |
| services d'embarquement | toujours a70f1e462bd9df95 (f85977d09e), `max_chars_per_text` 12000 sur 7878 et 7879. Mon binaire de 3177d2489e (6196fbe7102ad533) n'est **pas posé**, et il est **périmé** : `Cargo.toml` et `Cargo.lock` ont changé entre 3177d2489e et 927941a67a | **non posé, à rebâtir avant toute pose** |
| épinglage des passes | `tools/poste/passe_epinglee.sh` épingle 59ac9c86bc (68590270e2), témoin `test_passe_epinglee.sh` 30 vert sur master ; une vraie passe candidate citée verte ; `lib_seule.sh` retiré des deux caches et gardé en `.avant-retrait-59ac-20261116` | **fait** |

## 1. Les copies du cache, par rapport à master (relu le 18, aux deux postes)

- **Identiques à master aux deux postes** : `push_garde.sh` (fff9a4d9e20e739e, posé par Codex avec le correctif des
  en-têtes), `passe_epinglee.sh` (2150fcd847b1df7d), `poste` (79e3a8bbf457e3bc), `required_suites.sh` et `.txt`,
  `controle-du-worktree.sh`. À luciepc aussi : `garde_worktree.py`, `rag3db_libs.py`, `ouvrir-bases.py`,
  `bascule-pose.sh`, `bascule-reveil.sh`, `bati-lucie-luciepc.sh` et les autres scripts de `docs/optimiseur/scripts/`.
- **`garde_worktree.py` à luciepc** : Codex l'a écrasé par `scp` le 16 à 16 h 39, avant d'en prendre l'empreinte
  (relevé de c7). La copie actuelle est identique à `extension/rag3weaver/scripts/garde_worktree.py` de master. La
  version d'avant est perdue, et rien ne dit si elle différait. C'est une faute de méthode : la règle est
  `cp -p … .avant-<lot>`, puis `install`, puis l'empreinte. Mais l'état est sain.
- **Diffèrent de master, à luciepc seulement**, des copies **antérieures à Codex** : `controle-des-deux-sens.sh`
  (14 nov.), `nuit-verdict.sh` (13 nov.), `test_poste.sh` (12 oct.), `mesure_e2e.sh` (11 oct.). Ces anciennes copies, de
  G et d'avant, ne sont lues par aucune passe de la file. Codex a joué la bascule avec ses propres copies, rangées sous
  `~/.cache/optimiseur-notes/bascule-16nov-927/`. À recopier ou à retirer au prochain lot de G, avec `.avant-…`.
- Les autres fichiers du cache (505 au portable, 1 272 à luciepc) n'ont aucun homologue dans le dépôt : ce sont des
  scripts de passes anciennes, sans rapport avec cette fenêtre.

## 2. Le tableau

### Commits de master dans le périmètre de G

| commits | quoi | verdict | pourquoi |
|---|---|---|---|
| 806715b2d6, dc32e7073a, 58798c9c22, f83c6c4dd6 | le plan de la bascule annoté (fenêtre, pose, pin, limites des sondes en lecture seule) ; ticket `2026-11-16-ouvrir-bases-lit-une-route-status-inexistante.md` | **utile, à garder** | Ce que disent ces notes correspond à ce que je relis aux deux postes. Les limites sont dites, par exemple « FINI lib/pose ne signifie pas FINI migrations ». La maintenance d'arbre, rouge après la pose, n'a pas été contournée. |
| e271ee084c, 68590270e2 | l'épinglage de la lib 59ac9c86bc dans les passes (mon entrée 6 du 16) | **utile, à garder** | Rouge d'abord sur la lib complète, puis le pin ; témoin 30 vert ; le ticket est fermé. |
| ce6aa9ab60, b74b85257b, ecea15b074, df2a578988 | la garde pile juge **tous** les en-têtes du range-diff, ou refuse | **utile, à garder** | Un vrai trou de la garde. Git aligne les numéros par des espaces dès dix commits (« ` 1:` »), et l'ancien `case [0-9-]*:*` sautait ces lignes : une pile de dix commits ou plus passait **sans être jugée**. Correctif relu ligne à ligne : en-tête par expression régulière, `--no-patch`, le cas `!` toujours comparé par `git diff` hors `.md`, et « aucun en-tête jugé » est un arrêt. `test_stack_headers.py` 7 OK, `test_push_garde.sh` 49 vert. |
| 3db06a9d48, c651bd0f83, 6d0ce1415b, 51e7975d88, 335a941bcf, b7901cf8bb, f04d03e262, 4a78bf1ee7, 866272adbb, 2d389f8a8d, e51b0d0c5b, 57fbd9aa56, 76e4690a9b | le générateur de retour des instantanés (`docs/procedures/scripts/generate_snapshot_return.py`, `restore_snapshot.py`, `instantanes.sh`) et son ticket | **utile mais à requalifier** | Le défaut corrigé est réel : le `RETOUR.sh` d'avant oubliait le WAL et les voisins créés après l'instantané. Mais **ses témoins laissent leurs dossiers** : `tempfile.mkdtemp(dir=~/.cache)` sans effacement, 32 dossiers `generated-return-integrity-*` au portable et 23 à luciepc. **Un cas dépend de la longueur de `HOME`** : `test_second_live_socket_refuses_before_first_move` rend « AF_UNIX path too long » dès un `HOME` de 26 caractères. Vert sous le vrai `HOME`, 10 sur 10 ; `test_restore_snapshot.py` 5 OK. Deux tickets à ouvrir (§ 3). |
| d395cb1f03, b0a845bb48 | « ties » et « fixtures » | hors de mon périmètre | Ils touchent les dossiers d'autres rôles (mémoire, recherche) : à leurs sessions. |

### Branches `cx-optimiseur-*`

| branche | date | hors master | verdict |
|---|---|---|---|
| `cx-optimiseur-stack-guard-16-r2`, `-return-generator-16-r2` | 16 nov. | 0 commit | **sans objet** : leur contenu est sur master. |
| `cx-optimiseur-stack-guard-16`, `-stack-guard-16-candidate`, `-return-generator-16` | 16 nov. | 1, 2 et 12 commits | **inutile, dépassé** : premières versions remplacées par les `-r2` atterries. À laisser sur `origin`, sans les fusionner (on ne supprime pas de branche distante). |
| `cx-optimiseur-bascule-16` | 16 nov. | 9 commits, docs seules | **utile, à garder en branche** : les preuves de la fenêtre. Ce qui comptait est déjà sur master par les commits de plan ci-dessus. |
| `cx-optimiseur-pg-ro-attestation-20261116-r1`, `-pg-v2-preparation-20261117-r1`, `-psql-host-bundle-20261117-r1`, `-psql-runner-source-20261117-r1` | 16-17 nov. | 124 à 173 commits ; environ 20 000 lignes sous `docs/optimiseur/16-11-2026/pg-ro-attestation/` et `17-11-2026/` | **inutile pour le produit, à ne pas poursuivre** (détail plus bas). Ces branches ne sont pas sur master : il n'y a rien à retirer. |
| les 15 branches du 11 au 13 novembre (`-suivi-pose`, `-preparation-68*`, `-banc-7882*`, `-card0-*`, `-ci-vector*`, `-passation-weekend`…) | 11-13 nov. | docs seules | **hors de cette fenêtre** : le travail d'une semaine Codex antérieure, sans code. |

**Les branches PostgreSQL, en détail.** Leur ticket parent est
`docs/recherche/tickets/2026-11-16-la-garde-des-comptes-en-blobs-reste-a-302-documents-pour-301-lignes.md` : un
écart d'un document sur 302. Pour le départager, Codex a bâti en deux jours un outillage de qualification privé :
- une attestation en lecture seule ;
- un client Rust de démonstration sous `docs/` ;
- l'extraction d'un `psql` d'une image Docker ;
- un « runner » de 19 témoins sur faux processus ;
- un contrat d'attestation natif « purement sémantique ».

Sa propre passation dit que le chemin réel reste « HOLD ». Aucun contact PostgreSQL n'a eu lieu, et aucun compte de
302 ou de 301 n'est relu. L'effort est sans rapport avec la question : une requête de compte, lue deux fois, y
répondrait. Ces branches portent aussi du Rust (`catalog.rs`, `code_sync.rs`, `fts_existing.rs`), mais il vient de leur
base FTS302, pas du rôle optimiseur. Le risque est nul pour master, puisqu'elles n'y sont pas. Le coût est le temps et
le quota d'une session sur deux jours. À dire à c7 et à recherche : la question FTS302 se reprend autrement.

## 3. Verdict d'ensemble

1. **La bascule est faite et juste** : lib 59ac9c86bc, hôtes, contrôle des deux sens à 0 perdue, pin des passes, retour
   arrière gardé. C'est le cœur de ce qui m'était confié, et c'est bien fait.
2. **Deux outils utiles sur master** : la garde pile, qui comble un vrai trou au-delà de neuf commits, et le générateur de
   retour, qui sauve le WAL. Le second est à requalifier : ses témoins laissent des dossiers et dépendent de la longueur de `HOME`.
3. **Deux fautes de méthode, sans dommage constaté** : `garde_worktree.py` écrasé avant son empreinte, et
   55 dossiers de témoins laissés dans `~/.cache`.
4. **Deux jours inutiles** : l'outillage PostgreSQL, hors master, est sans proportion avec un écart d'un document, et le
   chemin réel n'est pas joué.
5. **Le service d'embarquement neuf n'est pas posé**, et le binaire préparé est périmé : la pose reste à faire, sur un
   binaire rebâti.

**Deux tickets ouverts** dans le même commit que cette page :
- les témoins du générateur de retour laissent leurs dossiers dans `~/.cache` (`mkdtemp` sans effacement) ;
- le cas du socket dépend de la longueur de `HOME` : il faut un dossier court sous `/tmp` pour le socket seul.

## 4. Ce que ça change à ma page du 16 (`docs/optimiseur/16-11-2026/02-passation-pour-codex.md`)

- **Entrée 6, « épingler la lib après la bascule » : faite** par Codex (68590270e2). Ticket fermé.
- **Entrées 1 à 5 et 7 : pas commencées** par Codex dans mon rôle. Elles restent en tête de ma liste :
  1. le témoin de `poste` ;
  2. le témoin de `run_e2e` et l'attente du poste ;
  3. `tools/shell` dans la voie C++ ;
  4. le juge known_red intermittent ;
  5. la base « un seul sens » au témoin ;
  6. le défaut de 40G, qui attend deux relevés.
- **S'ajoutent** :
  - les deux tickets du générateur de retour ;
  - la pose du service d'embarquement, sur un binaire rebâti ;
  - la recopie ou le retrait des quatre copies anciennes de luciepc ;
  - le ticket ouvert par Codex sur `ouvrir-bases.py` et sa route `/status` absente, que j'ai écrit et que je reprends.

## 5. La lisibilité de sa passation

`docs/optimiseur/17-11-2026/99-passation-pour-claude.md`, sur la branche `cx-optimiseur-psql-runner-source-20261117-r1` :
- **Sur le fond, elle est honnête** : ce qui n'est pas livré est dit « NON LIVRÉS / NON QUALIFIÉS », et les preuves sont
  localisées avec leurs empreintes.
- **Sur la forme, elle est difficile à lire** : un jargon d'emprunt (« coupons », « leases », « GO », « Common »,
  « HOLD »), des mots collés (« 299fichiers/12574694octets », « 19PASS »), et une densité de hashes qui cache la seule
  information utile, à savoir qu'aucun contact réel n'a eu lieu.
- **Elle ne dit pas la bascule**, qui vit dans le plan annoté et non dans la passation.

Il m'a fallu relire l'état aux deux postes pour savoir ce qui était fait. Une passation devrait le dire en tête, en
phrases simples.

## 6. Suites données, au mot de c7 (18 novembre, 11 h 4x)

- **Les quatre copies anciennes de luciepc**, recopiées depuis master caeba78bdf à 11 h 47 (`install -m 755` en
  `.neuf` puis `mv`, anciennes en `.avant-recopie-18nov`) :

  | copie | avant | après (= master) |
  |---|---|---|
  | `controle-des-deux-sens.sh` | d46666e32fbab03e | 284f33e5c4dd37e0 |
  | `nuit-verdict.sh` | 4b0e288558b0780f | 220662a05d964625 |
  | `test_poste.sh` | 4c25409647eb9951 | 0733c2555744f829 |
  | `mesure_e2e.sh` | 207d2d3f36dd428e | 98444e2bac27fff0 |

- **Le binaire des services d'embarquement**, rebâti sur master caeba78bdf (Cargo.toml et Cargo.lock neufs) à luciepc,
  de 11 h 47 à 11 h 57, dans `~/.cache/rag3weaver-build/nuit-caeba78bdf/rag3weaver-embeddings` : sha256
  831529bb4e42c149. Les champs `max_chars_per_text` et `chars_per_token` de `/sante` sont dans le binaire. **Non
  posé** : la pose attend le mot d'embarquement et une fenêtre sans embarquement. Elle se fera en place, par
  `docs/optimiseur/scripts/poser-embarquement.sh caeba78bdf bge-m3 granite-278m`, vecteur témoin au bit près, retour
  automatique si rouge. Le binaire de 3177d2489e (6196fbe7102ad533) est abandonné.
- **PostgreSQL : rien à poursuivre.** La question FTS302 revient à recherche, dans son lot 1
  (`cx-recherche-fts-generation-union-r2`). Recherche a vérifié que le Rust des branches `cx-optimiseur-pg-*`/`psql-*`
  est identique au sien et n'en prend rien de plus. Les quatre branches restent sur `origin`, sans suite.
