# codeparsers parse le TSX avec la grammaire TypeScript : le JSX tombe en erreur, ses usages se perdent

- **État** : corrigé — codeparsers `9d5de97` (pointeur monté le même jour). Ouvert le 18 octobre 2026
  par codeparsers, lu pendant l'étude des composants React,
  `extension/rag3weaver/docs/10-octobre-2026-18h20/codeparsers/07-un-composant-react-est-un-gabarit.md`).
  Joué : la grammaire TSX seule ne suffisait pas, la collecte des symboles locaux prenait le nom
  de la balise pour une définition et écartait la référence ; les deux sont corrigés.
- **Gravité** : réponse incomplète. Les scopes d'un `.tsx` qui suivent du JSX sont mal découpés,
  et un composant employé en balise (`<Button/>`) n'a aucun usage.
- **Atteignable en service** : oui, pour tout dépôt qui contient des `.tsx` ou des `.jsx`.
- **Touche rag3weaver** : oui (l'index du code, `usages`, le voisinage).

## Ce que c'est

`.tsx` est rangé en `SupportedLanguage::Typescript`, `.jsx` en `Javascript`
(`codeparsers/src/parallel/project_parser.rs:15-48`), et les deux reçoivent
`tree_sitter_typescript::LANGUAGE_TYPESCRIPT` (`src/scope_extraction/base_scope_extraction_parser.rs:247-251`,
de même `src/syntax_highlighting/syntax_highlighting_parser.rs:26`). Cette grammaire n'a pas de
règle JSX : les expressions JSX y deviennent des nœuds `ERROR`, ou des assertions `<T>`.
`LANGUAGE_TSX` n'apparaît que dans deux exemples de mesure (`examples/couverture.rs:32`,
`examples/banc_couverture.rs:51`), qui ne mesurent donc pas ce que fait la production. Un relevé
de couverture l'avait vu (« tsx 12 fichiers 12 en erreur »,
`docs/30-aout-2026-06h00/01-cahier-des-charges-tout-le-fichier-est-couvert.md:172`).

Le code qui transforme `<Button …/>` en référence d'usage existe
(`base_scope_extraction_parser.rs:2412-2445`) mais ne se déclenche jamais, faute de nœuds `jsx_*`.

## Recette

Un `app.tsx` avec `function Button() { return <b/> }` et `export function App() { return <Button/> }` ;
l'indexer seul, puis `usages Button` : attendu un appel depuis `App`, rendu aucun.

## Témoin

`codeparsers/tests/jsx.rs` (six cas : la balise de composant, la balise en minuscule, le `.jsx`, le
nom membre, le choix par extension, le `.ts` qui garde sa grammaire) ; `tests/grammaires.rs` charge
aussi `LANGUAGE_TSX`. Rouge avant le correctif de la collecte locale (3 cas sur 6), vert après ;
toute la suite de codeparsers verte.

## Ce que serait le correctif

Choisir la grammaire par extension : `LANGUAGE_TSX` pour `.tsx` et `.jsx` (`SupportedLanguage` ne
porte pas l'extension : une variante `Tsx`, ou le chemin passé jusqu'au parseur). Puis, sur le code
JSX réveillé, filtrer les balises en minuscules (`div`) et garder les noms membres (`Foo.Bar`).
Ajouter `LANGUAGE_TSX` à `tests/grammaires.rs` et aux mesures.

## Pour le fermer

Le témoin de la recette vert ; une fixture `.tsx` avec usages, props et imports ; les exemples de
mesure et la production sur la même grammaire.
