# Un paramètre générique Rust garde sa contrainte dans le nom

- **État** : ouvert ; baseline observée, aucun correctif ni rejeu.
- **Pour** : Codeparsers après END de baseline.
- **Famille** : produit ; attribut structuré incorrect sur AST valide.
- **Parent** : docs/codeparsers/tickets/2026-11-16-verifier-la-couverture-sur-des-corpus-publics-multilangages.md.

Source gel02 : tokio f3e855fa55d6c1cc3e7a28e115a421b8ee5860bd,
`tokio-util/src/io/read_arc.rs`, SHA256
dab2593708bc82235202fc49c02e4af97fd8b4310baa98553c2f07fa75f7bb72.
Ligne27 : read_exact_arc déclare R contraint par AsyncRead. L'oracle avant
sorties905bf17f36 identifie la fonction générique et son ancrage.

ELFbff1cafb...5565e2, ASTvalid/completed, compact non tronqué : scope
read_exact_arc correctement nommé et localisé, retour io::Result<Arc<[u8]>>,
mais generic_parameters[0].name vaut `R: AsyncRead`, constraint vaut `: AsyncRead`.
Le type GenericParameter sépare name et constraint ; le nom de l'identifiant
est R. Le présent ticket porte sur la pollution du nom, sans imposer une
normalisation cosmétique supplémentaire de la chaîne constraint.

Source RO relue : extract_rust_generics prend le texte du type_parameter entier
comme name, puis récupère aussi trait_bounds comme constraint. Aucune
compilation ou modification du parseur n'a été effectuée pour ce diagnostic.

Minimal proposé, **non joué** : `fn read<R: AsyncRead>(read: R) {}`.
Rouge à voir avant fix : name=R, contrainte présente. Ajouter le cas sans borne,
les lifetimes et les defaults en témoins séparés selon le contrat supporté ;
ne pas annoncer leur support par ce ticket seul.

Preuve figée : oracle-review-02/sample-results.jsonl SHA256
3e3ee90db2111430e8bb9f6e27db6bccc92df673346574df7804a0375fc311ff,
`~/.cache/rag3db-banc-notes/public-corpus-20261116/` aux deux postes.
Aucun fix avant END ; ce cas ne prouve ni résolution du trait ni couverture Rust.
