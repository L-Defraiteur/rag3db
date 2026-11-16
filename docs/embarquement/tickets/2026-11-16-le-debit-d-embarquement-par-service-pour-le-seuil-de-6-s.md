# Le débit d'embarquement par service, pour le seuil de 6 s

- **État** : ouvert (16 novembre 2026, embarquement ; partage décidé par c7, f338855abf)
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
