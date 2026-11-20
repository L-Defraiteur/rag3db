# Le suivi des imports décale la ligne d'un trou qui commence par des lignes vides

- **État** : ouvert (20 novembre 2026)
- **Famille** : produit
- **Gravité** : basse : la ligne d'un site est fausse (trop tôt), le lien lui-même est juste
- **Atteignable en service** : oui, quand un nom importé n'est relevé que par le suivi des imports dans un scope de
  fichier qui commence par des lignes vides (les lignes d'import JavaScript, que la relève par le texte saute)
- **Touche rag3weaver** : oui, par codeparsers (la ligne d'un site `usages`)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, au lot des noms intégrés du premier niveau

## Constat

`ensure_import_references_tracked` (`base_scope_extraction_parser.rs`) calcule la ligne d'un nom importé depuis
`scope.content`, mais le contenu d'un scope de fichier est rogné de ses lignes vides en tête (`extract_file_scopes`,
`gap_content.trim()`) alors que `scope_start_line` reste au début du trou. Le site sort donc trop tôt d'autant de
lignes vides. Dans `fn a() {}\n\n\nuse std::sync::Arc;\n`, `Arc` sortait à la ligne 2 au lieu de 4. La relève par
le texte, elle, lit le texte non rogné avec sa vraie ligne de base (le correctif du 3 octobre, `references: (&str,
usize)`) ; le suivi des imports n'en a pas profité. Il était masqué tant que la relève par le texte gardait le nom.

## Ce qu'il faudrait

Donner au suivi des imports la même ligne de base que la relève par le texte (le texte non rogné du trou), plutôt que
de changer les bornes des scopes de fichier (l'invariant de couverture). Témoin rouge d'abord : une ligne d'import
JavaScript après deux lignes vides, le site du nom importé à sa vraie ligne.
