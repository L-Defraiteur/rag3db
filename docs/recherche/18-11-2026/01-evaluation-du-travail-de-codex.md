# Évaluation du travail de Codex dans le périmètre de recherche

18 novembre 2026, au mot de Lucie (« d'abord chaque session évalue la pertinence de ce qu'a fait Codex »).
Codex a tenu le rôle de recherche du 16 novembre, 11 h, au 17 novembre, 21 h 08. Lecture faite sur
`origin/master` et sur les branches `cx-…` publiées, sans rien exécuter.

## 1. Le tableau

### Sur master, dans le périmètre ou au contact

Codex n'a rien posé sur master sous `docs/recherche/`. Trois séries de code touchent mon périmètre :

| commits | quoi | à qui | verdict |
|---|---|---|---|
| 032d273942, e9febedb0e, 3e771bc260 | l'ordre stable des ex æquo (`search.rs`, `stable_order`) : rouge d'abord, puis le correctif | everything declarative (`docs/everything-declarative/16-11-2026/02-candidat-stable-tie-identity.md`) | **utile, à garder**, sous la réserve d'ED. Rouge d'abord tenu, identifiants anglais. Je n'ai pas vérifié sa passe ; c'est l'évaluation d'ED |
| d1fd2191cc, 739072de8a, 68edcc2af6, 4574948e67 | la clé du débit d'embarquement par adresse et par poste (`catalog.rs`, `catalog/progress.rs`, `embedder.rs`, `model_source/activation.rs`, `daemon/embeddings.rs`) | embarquement (ticket `docs/embarquement/tickets/2026-11-16-le-debit-d-embarquement-par-service-pour-le-seuil-de-6-s.md`) | **utile mais à requalifier, d'urgence**. Son propre ticket dit « correctif préparé, non qualifié » et « les nouveaux témoins ne sont pas encore joués ; aucun vert ou compilation n'est prétendu ». Il touche `catalog.rs` et `catalog/progress.rs`, qui exigent `e2e_code_sync` : une suite non jouée est rouge. Trois messages de commit sans préfixe (« Éprouver… », « Séparer… », « Tracer… ») |
| 936f13daa2, 5ed1961d6b, efb9aba736, 4b32256693, 7b3a88e13b | le jeton Vertex renouvelé ; l'écriture des scripts de fixture (`codex_threads.rs`) | ED et G, archi | hors de mon périmètre |

### Les branches `cx-` du rôle, du 16 et du 17 novembre

Trente-cinq branches, toutes du même sujet : les comptes du plein texte, la maintenance d'un index
existant, et une garde commune aux deux chemins de synchronisation. Elles s'empilent : chacune reprend
la précédente. La dernière, `cx-recherche-fts-generation-global-pilot-r5` (46078c4ff1, base 99fef92ce6),
porte 76 commits pour 127 fichiers : environ 790 lignes de code et plus de 12 000 lignes de docs et de
preuves.

| branches | ce qu'elles portent | verdict |
|---|---|---|
| `cx-recherche-fts-counts-freshness-r1`, `cx-recherche-fts-existing-only-r1` | deux tickets, docs seules : l'entrée 1 de ma page du 16 (`fts_counts`), et le ticket neuf « la maintenance du plein texte ne peut pas ouvrir un index existant » | **utile, à garder**. Le ticket neuf nomme un vrai obstacle : `fts-live-ids` rend 422 sur un backend fermé, et `ensure_fts_handle` peut effacer ou recréer un dossier |
| `cx-recherche-fts-existing-*`, `…-canon-*` (onze branches) | `src/fts_existing.rs` (neuf) et `Catalog::open_existing_fts_snapshot` : ouvrir un index lucivy existant sans écrivain, refus typés (absent, corrompu, génération divergente, autre cellule, alias public), dix témoins `existing_fts_*` | **utile mais à requalifier**. Le code est sain sur la forme : anglais, erreurs typées, témoins nommés. Mais aucune passe entière n'a été jouée, seulement des « pilotes » ciblés, et le premier s'est arrêté sur une collision de portée systemd avant Cargo |
| `cx-recherche-fts-double-*`, `…-generation-guard-*`, `…-generation-union-*` (dix branches) | `prepare_fts_for_source_sync` dans `code_sync.rs` : la garde des comptes du plein texte et le rebâti des index en attente, communs aux deux chemins de synchronisation ; test `code_sync_fts_guard_double_tests.rs` | **utile mais à requalifier : le meilleur du lot**. Sur master, `check_fts_counts` et `rebuild_pending_fts` ne passent que par `synchroniser` (instantané), jamais par `synchroniser_par_generations` : c'est un vrai trou, avec un rouge produit (page 21 de la branche). Réserve : des crochets de test (`fts_guard_double_tests::record`) sont posés dans le code de production. Ils sont sous `cfg(test)`, mais un compteur d'appels par la porte d'un témoin vaudrait mieux |
| `cx-recherche-fts-blobs-*` (deux branches) | diagnostic en lecture seule du ticket « la garde des comptes en blobs reste à 302 documents pour 301 lignes » | **utile mais à requalifier** : un constat borné (un orphelin froid), pas une cause |
| `cx-recherche-fts-generation-global-pilot-r1` à `r5`, `…-global-preuves-r4`, `…-source-doc-r3`, `cx-recherche-fts-fixture-root-*` (onze branches) | la préparation d'une passe globale jamais lancée : recettes, pins, contrôleurs, contre-exemples, wrappers « provisoires » | **inutile en l'état** : de la préparation de préparation. À ne pas reprendre ; garder les branches comme archives, sans les effacer |
| `cx-recherche-passation-claude-20261117-r1` | la passation de Codex, plus trois inventaires JSON | voir la partie 4 |

**Les règles** : l'anglais du code neuf est tenu. Le rouge d'abord est tenu sur le papier : témoins publiés
avant le correctif. Mais la plupart des rouges sont « non observés », et aucune suite exigée n'a été jouée
sur le code empilé. Aucun force push, et aucun lot de ce périmètre n'a atteint master.

## 2. Le verdict d'ensemble

- **Ce qui avance vraiment** : le trou de la garde des comptes sous les générations, trouvé et corrigé avec
  son rouge. La maintenance d'un index existant sans écrivain, bien bornée.
- **Ce qui est du papier** : onze pages sur douze, la pile « global pilot » r1 à r5, les manifestes et pins
  de preuves. 12 000 lignes de docs pour 790 de code.
- **Ce qui gêne** : du code d'embarquement sur master sans compilation ni témoin joués, de son propre aveu.
  Et trente-cinq branches empilées, qui rendent la tête utile dure à trouver.
- **Le style** : la prose de Codex est souvent compressée jusqu'à l'illisible (« SourceAPP6365…/master99
  audités, ROHotfixmodeOID66'=' favorable »). Elle ne tient pas notre règle d'une prose française lisible.
- **À faire** : extraire les deux lots utiles en lots normaux, et requalifier le débit par embarquement.

## 3. Ce que ça change à ma page du 16 (`docs/recherche/16-11-2026/03-passation-pour-codex.md`)

| entrée | état |
|---|---|
| 1. l'état d'index qui voit les orphelins | **commencée par Codex**, détournée vers le prérequis « ouvrir un index existant ». Code en branche, `cx-recherche-fts-existing-canon-proofs-r1` (c16f1a4075), non qualifié. Le relevé vivant dans l'état, la vraie demande, n'est pas fait |
| 2 à 9 | **intactes** |
| 10. l'attente d'index par vue | intacte côté recherche. La brique du débit d'embarquement est sur master, non qualifiée (partie 1) |
| hors liste | **la garde des comptes commune aux deux chemins de synchronisation** : trouvée par Codex, en branche `cx-recherche-fts-generation-union-r2` (6365396b63, sur 99fef92ce6), non qualifiée |

## 4. La passation de Codex est-elle exploitable sans lui ?

**En partie seulement.** Les inventaires JSON (worktrees, branches, têtes) sont précis et servent. La page
`99-passation-pour-claude.md`, en revanche, est un relevé de relectures (« RO ») d'autres rôles, écrit en
abréviations (« GO », « RO R favorable PREPA23 », « pin685/lib59ac »). Elle ne dit ni quelle branche porte
le code à reprendre, ni ce qui reste à faire pour l'atterrir. Pour reprendre, il faut relire les branches
elles-mêmes, comme cette page l'a fait.

## 5. Ce que je propose de faire, dans l'ordre

1. **Un lot** : la garde des comptes commune aux deux chemins de synchronisation. Je le reprends de
   `cx-recherche-fts-generation-union-r2`, je le repose sur master et je retire les crochets de test du
   code de production au profit d'un compteur. Son rouge sera revu, puis la passe jouée avec e2e_code_sync.
2. **Un lot** : l'ouverture d'un index existant sans écrivain (`fts_existing.rs`), puis l'entrée 1 sur elle.
3. **Embarquement** requalifie sa clé du débit (rouge, puis vert, e2e_code_sync, lib). Je ne la touche pas.
4. Ma page du 16 reprend à l'entrée 2.
