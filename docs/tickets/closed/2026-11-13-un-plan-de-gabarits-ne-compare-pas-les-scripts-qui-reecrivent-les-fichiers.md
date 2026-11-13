# Un plan de gabarits ne compare pas les scripts qui réécrivent le dossier des fichiers

- **État** : corrigé le 13 novembre 2026, dans le lot qui l'ouvre (session mémoire, 96). Relevé par
  everything declarative en relisant la première tranche de M8.
- **Famille** : produit
- **Gravité** : perte possible. Une migration ou un rendu changé réécrit les fichiers sous
  `files_root` sans que le plan l'ait dit.
- **Atteignable en service** : non : la voie fermée de M8 n'a pas encore d'appelant en service.
- **Touche rag3weaver** : oui, `src/template_plan.rs` (`compatibility`).

## Ce qu'on voyait

Le plan comparait `files_format` et `entities` comme valeurs JSON, et les schémas par leur
contenu, mais pas le contenu des fichiers qu'ils citent : `files_format.migrations[].script` (un
script Rhai) et `entities.*.files.template` (un gabarit jinja). Un de ces fichiers changé seul
passait par `merge_files`, sans refus.

## Le correctif

Le contenu de chaque script de migration de format et de chaque gabarit de fichier miroir cité,
chez le local ou chez la cible, est comparé entre le local et le candidat, comme un schéma. Un
changement est une compatibilité à qualifier, nommée au chemin du fichier. Témoins :
`a_files_format_migration_script_changed_alone_is_not_compatible` et
`a_mirror_file_template_changed_alone_is_not_compatible`, rouges sur dd8111736 (plan Ok ; le paquet
de départ du second passe le plan), verts après.
