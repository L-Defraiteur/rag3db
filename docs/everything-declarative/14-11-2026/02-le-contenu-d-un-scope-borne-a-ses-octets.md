# Le contenu d'un scope, borné à ses octets

14 novembre 2026, everything declarative, à la demande de c7, après la mesure de lucivy (page
5fa98e131) : 844 Mo de trop dans un index, un même morceau de 133 Ko répété 484 fois, 2 588 contenus
répétés, 18 862 documents de trop.

## La cause

- **rag3weaver découpait déjà par octets.** Le texte propre d'un scope est
  `texte[scope_start_byte..scope_end_byte]`, chaque enfant direct remplacé par sa ligne de
  signature (`own_texts`, `extension/rag3weaver/src/code.rs`).
- **Mais codeparsers dérivait ces octets des lignes** (`finalize`, `src/parallel/parser_worker.rs`) :
  du début de la première ligne du scope à la fin de sa dernière. C'est voulu et documenté : c'est
  le contrat de `File`, de l'invariant de couverture (« l'union des scopes couvre le fichier ») et
  des lectures par tranche.
- Dans un bundle minifié d'une seule ligne, chacune des centaines de fonctions recevait donc **la
  ligne entière**.

## Le correctif, en deux dépôts

- **codeparsers** (branche `empan-du-noeud`) ajoute deux champs à `ScopeInfo` : `node_start_byte`
  et `node_end_byte`, l'empan exact du nœud tree-sitter.
  - Ils sont posés aux 44 créations de scope depuis un nœud. Ils valent `None` pour les trois genres
    de trous entre déclarations et pour le texte brut.
  - Les octets à la ligne ne changent pas. C'est une précision ajoutée, pas une rupture de contrat.
- **rag3weaver** (branche `scope-borne-a-ses-octets`) : `own_texts` prend l'empan du nœud quand il
  existe, sinon celui des lignes, pour le texte propre comme pour le remplacement des enfants. Les
  morceaux et l'embarquement suivent ce texte.
- **Les témoins, vus rouges puis verts :**
  - deux fonctions sur une ligne ont chacune leur seul texte (`function a(){return 1}`, et non la
    ligne) ;
  - un scope qui occupe ses lignes entières garde exactement son texte.

## Ce que ça change pour les bases

- **L'adresse d'une version suit son texte.** `ScopeContent` est adressé par l'empreinte de son
  rendu (`code-scope-v1`), et le rendu prend le `content` du scope. La recette de rendu ne change pas,
  donc le format reste `code-scope-v1` : seul le texte que l'analyse produit change.
  - Un texte inchangé garde son adresse, ses morceaux et ses vecteurs.
  - Un texte changé prend une autre adresse, comme après une édition du fichier.
- **Ce qui change, et ce qui ne change pas :**
  - **le code ordinaire, une fonction par lignes : presque rien.** L'indentation de tête était
    déjà retirée par la dé-indentation. Seule change une dernière ligne qui portait autre chose après
    la fin du nœud : `};`, `}, // fin`, `} else {`, ou un second scope sur la même ligne ;
  - **les minifiés et les fichiers d'une seule ligne : presque tout**. Chaque fonction perd le reste
    de la ligne, et c'est là que tombent les 844 Mo ;
  - **les trous entre déclarations** (`file_scope_NN`) et **les gabarits** (`code_gabarits`) : rien.
- **Rien ne change en silence dans une base existante.** Le texte d'un scope ne change qu'à la
  réindexation de son fichier : une édition, la montre, ou un index explicite. Une base garde ses
  versions jusque-là.
- **La règle (A) de 1a joue** : une base sans versions reste « à migrer », et sa migration (1b)
  reprend le `content` que la base a déjà, sans réanalyse. Après 1b, un fichier réindexé prend son
  texte borné au nœud.
- **Les versions d'un minifié deviennent orphelines** une fois son fichier réindexé : leurs
  nouveaux textes, bien plus courts, s'embarquent, et les anciens restent jusqu'à la rétention (M5),
  sous le budget de 30 Go par dépôt.
- **Pour retrouver les 844 Mo sans attendre une édition**, il faut réindexer les fichiers concernés
  (les minifiés et les lignes longues). C'est un geste à part, sur le mot de Lucie.

## Ce qui reste à Lucie

- **L'exclusion des minifiés de l'index** est une autre règle, posée à Lucie par la page de
  recherche. Aujourd'hui, seuls les noms en `.min.js` et `.min.css` sont écartés
  (`SKIPPED_NAME_PATTERNS`, `code.rs:98`). Les 844 Mo viennent donc de bundles qui ne portent pas ce
  nom (un `dist/app.js`, par exemple). Ce correctif vaut pour tout fichier d'une seule ligne que
  l'index garde : un bundle sans suffixe, un JSON d'une ligne, du code qu'on veut pouvoir chercher.
- À noter : le témoin rag3weaver se joue sur `bundle.js`. Un `bundle.min.js` serait écarté par son
  nom, et n'aurait aucun scope.
