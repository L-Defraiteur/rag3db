# La sonde des identifiants lucivy lit l'index plein texte de `Scope`

- **État** : ouvert (18 novembre 2026), arbre principal ; le reste du ticket fermé
  `docs/tickets/closed/2026-11-13-les-sondes-bancs-et-mesures-du-code-lisent-scope-chunk-en-dur.md`.
- **Famille** : produit
- **Gravité** : sur une base à versions de scope, la sonde interrogerait un index plein texte qui n'y vit
  plus. Elle ne tourne dans aucune passe ordinaire.
- **Atteignable en service** : non (une sonde).
- **Touche rag3weaver** : oui (tests).

## Ce que c'est

`tests/sonde_identifiants_lucivy.rs` prend `cat.fts_handle(SCOPE)` (ligne 191) et
`Catalog::rechercher(&reel, SCOPE, …)` (ligne 149). Sur une base à versions, le texte et son index plein texte
vivent sur `ScopeContent`, pas sur `Scope`. Le lot A1 (c399bc86a7..d57c36968e) ne l'a pas touchée : la sonde ne
lit pas de morceaux.

## Ce qu'il faut pour le fermer

La sonde choisit la table de l'index plein texte selon la forme de la base, comme
`sonde_morceaux_vides::compter_une_base`. Elle exige au moins un résultat avant de juger. Le rouge se voit
d'abord sur une base à versions.
