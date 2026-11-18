# Le débit d'embarquement par service, pour le seuil de 6 s

- **État** : ouvert (16 novembre 2026, embarquement ; partage décidé par c7, f338855abf). **La clé est faite et
  qualifiée** (18 novembre, relu par embarquement et c7) : sur master à bef0b27cb3, dont le code est celui de
  07731af482, joué VERT le 16 à 14 h 23 sur la lib 59ac9c86b
  (`luciepc:~/.cache/rag3weaver-build/cx-rate-key-59ac-v1/resume.log`). **Restent** le point 1 (la mesure
  par passe et par adresse appelée) et le point 3 (la table `Catalog::embedding_rates()`).
- **Pour** : embarquement (la table par service et sa lecture) ; le banc (le jeu de lots fixe et la mesure) ;
  l'arbre (la conversion reste × débit par phase)
- **Famille** : produit
- **Gravité** : manque. Le point 7 de la vision de Lucie (1be007546b) traduit le seuil de 6 s d'attente de
  l'index en lots à embarquer ; il faut le débit **de chaque service**, pas d'un catalogue.
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui (`src/catalog/progress.rs`, `src/estimate.rs`, `src/dataflow/index_nodes.rs`)

## Ce qui existe

- `Catalog::note_embedding_rate(model, origin, rate)` et `embedding_rate(model, origin)`
  (`src/catalog/progress.rs`) rangent un `Rate { chars_per_second }` (`src/estimate.rs`) dans une clé de méta
  `embedding_rate:{model}:{origin}`. **`origin` ne vaut que `local` ou `service`** (`embedder_origin`) :
  deux services du même modèle, ou un service par tunnel et le même en local, partagent une seule valeur.
- `run_index` (`src/dataflow/index_nodes.rs`) note un débit mesuré à la fin d'un index de 512 morceaux et plus
  (`note_measured_embedding_rate`), et `known_embedding_rate` le relit pour l'estimation et le « reste
  environ N min » du journal.
- La **provenance par passe** (lot du 15 novembre, `Embedder::provenance_counts`, `dire_la_provenance` dans
  `index_nodes.rs`) dit déjà, après chaque passe, quel service ou quelle carte a servi combien de morceaux :
  « carte card2 », « service 127.0.0.1:7879 », « activation <nom> ».

## Ce qu'il faut faire (la part d'embarquement)

0. **Une page de conception d'abord**, soumise à l'orchestration avant le code :
   `docs/embarquement/<jour>/NN-debit-par-service.md`. **La clé est convenue avec le banc** (16 novembre,
   sa conception `docs/banc/16-11-2026/03-le-banc-d-etalonnage.md`, § 7) :
   `embedding_rate:{model}:{address}@{host}`, où `address` est l'adresse du fournisseur réellement appelé (pour
   une activation, celle de la prise : la carte, le service de repli, l'activation de repli), `local` pour un
   embarqueur en processus, et `host` le nom du poste du client (un tunnel compte à part, c'est voulu).
   L'ancienne clé (`embedding_rate:{model}:service` ou `:local`) se lit en repli quand la neuve manque, et ne
   s'écrit plus. Le banc écrit sa mesure sous la même clé, une seule source pour l'hôte.
1. Après chaque passe, `run_index` mesure le débit **par adresse appelée** (morceaux et caractères de la
   passe, divisés par sa durée), et le note sous la clé neuve. Un client de service simple (`DaemonEmbedder`)
   n'a pas de `provenance_counts` : il faut une lecture de l'adresse appelée (son `adresse`, ou celle de la
   prise d'une tenue). Seules les passes de 64 morceaux et
   plus comptent (une petite passe mesure surtout la latence).
2. `known_embedding_rate` lit la clé de la provenance courante de l'embarqueur (celle de sa dernière prise :
   la carte, le service de repli, l'activation de repli), puis l'ancienne clé en repli.
3. Une méthode publique `Catalog::embedding_rates()` rend la table entière (provenance, poste, modèle,
   caractères/s, textes/s, date), pour l'état d'index et pour l'arbre.

**Ce qui ne se touche pas** : le format de `Rate` (l'estimation en dépend), la ligne d'avancement du journal
(`plein texte prêt · vecteurs N sur C (P %, modèle) · reste environ …`), que `daemon_host/worktrees.rs`
analyse.

## Comment le prouver

- `the_rate_is_noted_per_provenance` (lib, `index_nodes.rs`) : un faux embarqueur qui sert deux provenances
  à des vitesses différentes (une attente par appel) ; après l'index, la table a deux débits, dans le bon
  ordre. Rouge d'abord.
- `known_embedding_rate_reads_the_current_provenance` : après un repli, le débit lu est celui du service de
  repli.
- Suites exigées par chemins : `catalog/progress.rs` → `e2e_code_sync` ; `index_nodes.rs` (le journal) →
  `e2e_lectures`.

## Taille

Une journée (conception, puis code et témoins).

## Ce qui le bloque

La conception relue par l'orchestration. La part du banc (le jeu de lots fixe) et celle de l'arbre (la
conversion) avancent à part ; la table se livre sans elles.

## Reprise de la clé seule, 16 novembre

Prise par embarquement sur `cx-rate-key-20261116-r1`, W propre local
`/home/lucied/git_workspaces/cx-rate-key`, depuis WIP rouge0aecc6b70b reposé
sur master927941a67a. Ancien W PC `rag3db-rate-key` préservé sans changement.
Contrat et limites : `docs/embarquement/16-11-2026/02-rate-key.md`.
Banc informé, sa mesure reste sans écriture au catalogue avant livraison de la
lecture ; ED confirme aucune collision sur les cinq chemins réservés.

Point explicite : `DenseTenu.distant()` vrai + adresse prise absente n'est pas
un embarqueur local ; aucune activation pour lire la clé. API historiques
conservées pour les appels explicites, mesures automatiques sous clé neuve.
Aucun élargissement à la table/mesure par passe. Aucun Cargo/test/service/GPU
avant fin de la fenêtre G/Arbre et nouvelle recette ; les rouges hérités restent
les seuls rouges déjà exécutés. Anciennes autorisations et échéances caduques.


### Correctif de clé préparé, non qualifié

*Dépassé par la qualification* : voir la section « Qualification source07731 acquise » et l'état en tête.

Branche `cx-rate-key-20261116-r1`, correctif e0c5dbfdd25a6fd70b417c2f9f3cdd3178c015e1
poussé après contrat3e3187b53d et compléments de testsff24e447b8. C7 confirme
la conservation des API historiques explicites. Cinq chemins réservés seulement,
aucune édition des chemins ED Vertex ou de index_nodes.rs. Les trois rouges
hérités ont été relus dans `rate-key-red.out` :0/3/0, causes attendues
(ancienne écriture, local absent, adresse confondue). Les nouveaux témoins ne
sont pas encore joués ; aucun vert ou compilation n'est prétendu.

Les compléments vérifient adresse inconnue distante, isolement adresse/poste,
changement de prise, relai Arc et lecture de l'adresse d'une activation non
ouverte sans la démarrer ; fixture Python et faux embarqueurs uniquement.
Aucune qualification avant « FINI G », nouvelle recette et source courante.

### Après le pin de la bascule

*Dépassé par la qualification* : voir la section « Qualification source07731 acquise » et l'état en tête.

WIP reposé sans conflit sur master68590270e21d00285b29800bd9cdecf0cb921461,
branche neuve `cx-rate-key-20261116-r2`. Diff produit r1→r2 vide ; SDK59ac
et copie de passe SHA2150fcd… relus. Aucun build ou test lancé.
Proposition bornée5400 s, neuf ciblés puis passe/suites exigées, target unique
hérité : `docs/embarquement/16-11-2026/03-rate-key-validation-proposal.md`.
La priorité Recherche/Archi aux deux créneaux Rust demeure ; cette proposition
n'est pas une admission. Attendre le cadrage courant, aucun ancien GO ou pin.


### Préparation du couple post-ED, non lancé

*Dépassé par la qualification* : voir la section « Qualification source07731 acquise » et l'état en tête.

ED intégré masterd395cb1f03f84c6abdfa023a0e3c7f87a7c0dab6 ; rebase neuf
sur `cx-rate-key-20261116-r3`, sans conflit. Cinq chemins réservés inchangés
à l'amont, mais union backend/search à qualifier réellement. Le pilote reste
une passe complète5400 s : neuf ciblés avant coordination canonique réelle,
puis code_sync/lectures/activation/F5/preuve/lib/NAV. Clarification C7 : appels
canoniques7878/7879 possibles après GO/coordination, aucun changement de services
ni base de Lucie ; slotPG fixture aussi coordonné. Source/pilote doivent être
admis ensemble. Aucun Cargo ou runtime n'a été lancé pendant la préparation.

### Réserve de profil avant admission — couple5e889 non autorisé

*Dépassé par la qualification* : voir la section « Qualification source07731 acquise » et l'état en tête.

Revue C7 du piloteb04a : le dictionnaire hérité ne fixait pas les racines XDG
state/config/data/cache ou TMP privées ; seul RAG3WEAVER_STATE_DIR était posé.
Cela ne garantit pas l'isolation du Catalog et des gabarits depuis HOME.
Aucun GO de qualification n'a été accordé et aucune passe n'a été lancée.

Correctif préparatoire dans ce même ticket avant son diff : racines privées
sous P, XDG_STATE_HOME=P/xdg-state et RAG3WEAVER_STATE_DIR=P/state-fixture
frères sans inclusion mutuelle ; TMP/config/data/cache/journaux/templates
privés. Magasin SDK absolu externe conservé. Profil non secret capturé et
invariants revérifiés en fin de qualification. Ajouter des témoins de profil
hermétiques sans Cargo et soumettre le nouveau couple à revue ; aucune barrière
manager/systemctl, aucune mutation du cache SDK partagé ou du code produit.


Réserve profil : preuve rouge réelle c0bdd7439a (5 cas,3 échecs/2 succès), puis
correctiffabcf40df5 (5/5 verts), petits CPU hermétiques uniquement. Les écritures
étaient dans un faux HOME temporaire contrôlé, jamais les racines de Lucie.
Le pilote délègue le profil corrigé, capture une liste positive non secrète et
vérifie de nouveau les racines avant son verdict final. Preuves et détails dans
03-rate-key-validation-proposal.md. Aucun Cargo/runtime ou GO de passe accordé.


### Qualification source07731 acquise, relectures avant candidat

Passe réelle terminée : neuf ciblés exacts verts, code_sync21/0/0, lectures27/0/0,
activation1/0/0, features8/8, lib2604/0/**72 ignorés non jugés**, NAV45/0/0.
Autorité validation-wm1493du VERT/restes[]/nettoye=true ; scan1803 identités,
aucun vivant ni inconnu. Créneau BGE/PG rendu explicitement à Mémoire/C7.
Rapport et manifeste : `docs/embarquement/16-11-2026/04-rate-key-validation.md`.
Action ED/Mémoire : revue indépendante du contrat clé seule, compatibilité,
provenance inconnue, getter sans activation, preuves et limites ; critères :
aucune réserve bloquante non traitée avant demande de tour. Ticket reste ouvert
pour table/mesure par passe ; aucune clôture ni atterrissage prétendu.
