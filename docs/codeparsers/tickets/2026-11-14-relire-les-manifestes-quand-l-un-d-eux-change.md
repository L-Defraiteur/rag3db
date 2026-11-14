# Relire les manifestes quand l'un d'eux change

- **État** : ouvert (14 novembre 2026) — limite admise par l'orchestration pour le lot de la provenance par
  les manifestes (rag3db `codeparsers/provenance-manifestes`) ; à faire plus tard.
- **Gravité** : réponse vieillie (une dépendance ajoutée à un manifeste reste `inconnue` jusqu'à la
  synchronisation suivante).
- **Atteignable en service** : oui, dès qu'on édite un `Cargo.toml`, un `package.json` ou un
  `pyproject.toml` sans resynchroniser la source.
- **Touche rag3weaver** : oui (`src/code_sync.rs`, `src/manifestes.rs`).
- **Famille** : produit

- **Pour** : session codeparsers

## Ce que c'est

`synchroniser` lit les manifestes du dépôt une fois, sur la liste complète de la source, et les note dans le
catalogue (clé méta `manifests_read`) ; la réindexation d'un fichier ou d'un lot de fichiers
(`reingest_file_retenu`, `reingest_files_retenu`) relit cette note, **sans relire les manifestes**. Un
manifeste édité seul n'est donc relu qu'à la synchronisation suivante ; `/etat` le dit à côté de
`manifests_read`.

## Ce qu'il faut pour le fermer

1. Quand le fichier réindexé est un manifeste (`manifestes::est_un_manifeste`), relire les manifestes de la
   source et renoter la clé.
2. Puis reclasser les bibliothèques et liaisons déjà indexées dont la provenance change (une dépendance
   ajoutée : `inconnue` → `dependency`), ou dire qu'il faut resynchroniser.
3. Un témoin : un `Cargo.toml` édité ajoute une dépendance, la bibliothèque passe à `dependency` sans
   synchronisation complète.
