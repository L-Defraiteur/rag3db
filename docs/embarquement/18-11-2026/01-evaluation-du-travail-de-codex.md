# Embarquement : évaluation du travail de Codex (16-17 novembre 2026)

Demande de Lucie, par c7, le 18 novembre à 11 h 3x : « chaque session évalue la pertinence de ce qu'a fait Codex ».
Périmètre : les commits de master qui touchent l'embarquement, et les branches `cx-embar-*`. Lu sur master
99fef92ce6 et à luciepc, le 18 novembre. Rien n'a été rejoué : les verdicts reposent sur la lecture du code, des
pages de preuve et des messages de commit.

## 1. Le tableau

| Quoi | Verdict | Pourquoi |
|---|---|---|
| **d1fd2191cc** — mes trois témoins rouges de la clé du débit (branche `rate-key`) | **utile, à garder** | Le rouge d'abord, vu pour la bonne raison le 16, atterri avant le correctif. |
| **429c1080cf**, **88509f2262** — le contrat de la clé (`docs/embarquement/16-11-2026/02-rate-key.md`) | **utile, à garder** | La clé convenue avec le banc, écrite : `{address}@{host}`, `local@{host}`, l'ancienne clé en repli, non écrite. |
| **739072de8a**, **68edcc2af6**, **4574948e67** — le correctif de la clé | **utile mais à requalifier** | Le code est celui que je voulais, en anglais : `called_address` (relayé par `Arc`, posé pour `DaemonEmbedder` et pour la tenue d'une activation), `embedding_rate_place`, `validate_rate_place`, `this_host` par `libc::gethostname` puis `/etc/hostname`. **Un défaut, lu dans le code et non joué** : une provenance inconnue est **refusée** (« embedding rate provenance is unknown »). Or `HttpEmbedder` (`provider: compatible`, construit par `model_source.rs:559` pour `models.embed`) se dit distant sans déclarer d'adresse. `run_index` propage ce refus par `?` (`note_measured_embedding_rate`, `index_nodes.rs:675`), tout comme la sonde de l'estimation (`probe_embedding_rate`, `index_nodes.rs:169`). Un backend dont l'embarquement passe par un fournisseur `compatible` échouerait donc à sa première estimation, ou à la fin d'un index de 512 morceaux et plus. À corriger, rouge d'abord : `called_address` pour `HttpEmbedder` (son adresse), et un débit qui ne se note pas sans faire échouer l'index (une ligne au journal). |
| **4754eade7c**, **bef0b27cb3** — la proposition de qualification et la page de preuves (`04-rate-key-validation.md`) | **utile, à garder** | Des preuves fortes et honnêtes, sur la lib basculée 59ac9c86bc : 9 ciblés, `e2e_code_sync` (21), `e2e_lectures` (27), `e2e_embarqueur_a_la_demande` (1), la lib entière (2604 passés, **les 72 ignorés expressément non jugés verts**), les témoins navigateur (45), `check_features` 8 sur 8 ; deux relectures (ED, mémoire). Les suites exigées par chemins sont jouées. Elles ne couvraient pas un fournisseur `compatible`. |
| **7b3a88e13b … b0a845bb48** — les « fixtures » (`codex_threads.rs`, `docs/memoire/16-11-2026/…`) | **hors de mon périmètre** | Lot de la session mémoire (96) : l'isolation des scripts de fixture du parent. C'est à elle d'en juger. |
| **4b32256693**, **3085ddb72d**, **798bd06a98**, **9403f53ce4**, **e3fd09dd74**, **4201b8e8cd**, **4bda23bbf4**, **e71559bc6e**, **c879e503c9**, **6a35a45925**, **47ede869c7** | **hors de mon périmètre** | Ils touchent `model_source.rs` ou des fixtures pour d'autres rôles : la source Vertex du banc, le profil privé XDG, les passes d'ED et de l'arbre. À juger par leurs sessions. |
| Branches `cx-embar-cycle-shared-health`, `-global-services-owner`, `-lifecycle-grant`, `-lifecycle-observation` (16 nov.), `-grant-watch-actor` (17 nov.) | **utile mais à requalifier, hors produit** | De l'outillage de coordination en Python (`provider_health_transport.py`, `global_services_owner.py`, `lifecycle_provider_observation.py`, `grant_watch_actor.py`, avec leurs tests sur faux processus), écrit pour les cycles de bascule de l'arbre (Q8, Q9). Ce n'est pas du code du produit. Ces branches partent d'un master ancien et mêlent d'autres lots (74 à 110 commits hors master chacune). **Ne pas les fusionner** sans une demande de l'arbre ou de l'orchestration ; s'il faut garder un outil, le reprendre seul, sur master, par un lot normal. |
| Branche `cx-embar-passation-20261117-r1` — la passation vers Claude | **utile, à garder (comme archive)** | Voir la partie 4. |

**Rien d'inutile ni de nuisible n'est sur master.** Aucun retrait n'est à faire.

## 2. Verdict d'ensemble

1. La clé du débit est faite, au contrat convenu avec le banc, et bien prouvée sur la lib basculée.
2. Un défaut reste dans la clé : un embarqueur distant sans adresse, ce qui est le cas du fournisseur `compatible`, fait échouer l'estimation et l'index. À corriger par un lot court, rouge d'abord.
3. Les règles sont tenues : rouge d'abord, code neuf en anglais, commits en gmail sans attribution, suites exigées jouées, ignorés non comptés.
4. Une grande part du travail de mon rôle a servi l'outillage de bascule de l'arbre, hors produit, et reste en branches.
5. **Le binaire neuf des services n'est pas posé** : 7878 et 7879 tournent toujours sur a70f1e46, depuis le 15 à 20 h 36 (`NRestarts=0`), sans `cut_texts`. La coupe côté client sur master suffit.

## 3. Ce que ça change à la page du 16

Dans `docs/embarquement/16-11-2026/01-passation-pour-codex.md` :
- **l'entrée 8 est faite pour la clé**, sur master à bef0b27cb3, mais **pas pour la mesure par passe** ni pour la table enrichie, qui restent au ticket. Une sous-entrée s'y ajoute : le défaut du fournisseur `compatible` ;
- **les entrées 1 à 7 et 9 à 13 ne sont pas commencées**. Codex n'en a ouvert aucune ;
- **l'état final** reste juste pour les services : binaire a70f1e46, binaire neuf non posé.

## 4. La lisibilité de sa passation

`docs/embarquement/17-11-2026/99-passation-pour-claude.md`, sur la branche `cx-embar-passation-20261117-r1` :
- **juste et honnête** sur ce qui a atterri (la clé seule), sur les comptes (les 72 ignorés non jugés), sur ses
  erreurs (« notre annonce “veille active” était incorrecte »), et sur ce qui n'a pas été rejoué ;
- **difficile à lire** : des phrases télégraphiques collées aux nombres (« lib2604 réussis », « 57CPU
  owner+helper+contrat+transport »), beaucoup de hashes sans dire ce qu'ils désignent, et un vocabulaire de
  coordination (grant, owner, ARM, RELEASE, Q8, Q9) qui n'est pas défini dans la page. Il faut lire les pages d'arbre et
  d'orchestration pour la comprendre ;
- **elle ne reprend pas la liste du 16** : on n'y lit pas, entrée par entrée, ce qui est fait et ce qui ne l'est pas.
  C'est la partie 3 ci-dessus qui le fait.
