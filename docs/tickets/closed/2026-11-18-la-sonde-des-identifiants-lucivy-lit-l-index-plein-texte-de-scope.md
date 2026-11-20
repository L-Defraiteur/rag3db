# La sonde des identifiants lucivy lit l'index plein texte de `Scope`

- **État** : corrigé (20 novembre 2026), par le lot `arbre-sonde-lucivy-versions` (d009102fb5) ; témoin
  `sonde_identifiants`, suite `sonde_identifiants_lucivy`, rouge ea7354b22b puis vert 19dd3c8f3d.
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

## La fermeture (20 novembre 2026)

- **Le défaut était déjà là, pas seulement à venir** : une base neuve prend la forme à versions, et la sonde bâtit la
  sienne. Le chemin réel (`Catalog::rechercher` sur `Scope`) trouvait les dix identifiants, neuf au rang 1, car la
  recherche renvoie `Scope` au plein texte de `ScopeContent` ; lucivy interrogé en direct mourait sur
  `fts_handle(Scope)`, un index qui ne vit pas sur une base à versions.
- **Le correctif** : la table du plein texte suit `code::scope_form` (`Scope` sans versions, `ScopeContent` sinon), et
  la sonde exige au moins un attendu trouvé de chaque côté avant de juger.
- **Vu en passant, avec lucivy** : sur une base à versions, le dossier `Lucivy_Scope` n'est plus ni écrit ni lu ; c'est
  un dossier mort (839 Mo sur la copie de G), proposé au ménage d'après la bascule, pas un défaut de recherche.
