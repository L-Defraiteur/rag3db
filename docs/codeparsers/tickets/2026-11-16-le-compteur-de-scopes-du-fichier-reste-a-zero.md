# Le compteur de scopes du fichier reste à zéro

- **État** : ouvert — incohérence observée sur les résultats de worker, contrat exact à qualifier avant correction
- **Famille** : produit
- **Gravité** : métadonnée de comptage ambiguë ; les faits extraits sont présents
- **Atteignable en service** : via ScopeFileAnalysis des workers ; effet des consommateurs à vérifier
- **Touche rag3weaver** : potentiellement, dépendance codeparsers ; le banc dérive un compte séparé
- **Pour** : codeparsers
- **Signalé par** : FULL02b oracle CSharp/Go, source384165760b/parser1b35cfb

Sur la ligne CSharp5002, total_scopes=0 mais5scopes IR sont rendus ; sur la ligne
Go8546, total_scopes=0 et75scopes récursifs sont rendus. Les identités/hashes et
artefacts sont au rapport03-rapport-premiere-campagne.md, aucun reparse.
ProjectParser calcule aussi un agrégat à partir de scopes.len : vérifier si le
contrat de fichier attend les seuls scopes de tête ou tous les descendants.
Ne pas mélanger ces deux granularités pour corriger le seul nombre affiché.

Le banc conserve le compteur déclaré et son décompte dérivé ; aucun zéro n'est
interprété comme absence de parsing/capture. Témoin minimal à écrire et jouer :
parse_file sur un fichier à une fonction, puis un fichier avec enfants ; vérifier
les garanties du compteur public et ses consommateurs. Aucun témoin réduit joué
ni correction annoncée. La référence32382fichiers reste figée.
Suite de la référence close `2026-11-16-qualifier-la-couverture-sur-des-corpus-publics-par-langage.md`.
