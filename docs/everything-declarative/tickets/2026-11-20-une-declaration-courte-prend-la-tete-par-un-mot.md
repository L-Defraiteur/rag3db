# Une déclaration courte prend la tête d'une recherche par le sens, par un mot de la requête

- **État** : ouvert (20 novembre 2026, soir)
- **Famille** : produit (la recherche, la fusion)
- **Gravité** : une recherche par le sens rend en tête une variable de quelques dizaines de caractères, sans vecteur,
  dont le nom est un mot de la requête. Apporté par la bascule du lot `variable-not-candidate` (f8c74d0876).
- **Atteignable en service** : oui, dès qu'un hôte sert ce lot.
- **Touche rag3weaver** : oui (`FuseResultsNode`, la décision (A)).
- **Pour** : everything declarative
- **Signalé par** : le banc (juge de 03fbecf8b8, puis sa correction et sa comparaison aux références de G).

## Ce qu'on voit

Sur la copie migrée de code-principal, avec le lot :
- « parse a json manifest » rend en tête `manifest` (une variable de 51 caractères), puis un autre `manifest` ;
- « rebuild the vector index » [python] rend aux rangs 1 à 4 des variables `index = ROOT/'README.md'` (22 caractères).

Sur les références de G (hôte baccb2ea6e, l'ancienne fusion), « parse a json manifest » rend en tête une méthode
(`parse_declaration`, ou `read_manifest` sur code-lourd), et la variable courte n'est qu'au rang 3 ou 4.

## La cause

La décision (A) : une ligne sans vecteur (`sans_vecteur`) voit son score de fusion ramené au poids total, soit multiplié
par le poids total sur le poids du BM25 (environ × 2,2 avec `bm25:0.45, vector:0.55`). Une déclaration courte première
en BM25, parce qu'un mot de la requête est son nom, bat alors tout ce qui a les deux signaux, y compris ce que le sens
trouve. Ce n'est pas `exact_name` : la requête n'est pas son nom exact.

## Ce qu'il faut pour le fermer

Ne plus mettre à l'échelle : compter le signal dense d'une ligne sans vecteur au rang qui suit le dernier de sa liste
(décision (A'), c7). La recherche par le nom reste tenue par la règle du nom exact (B). Un témoin rouge (une variable
courte première en BM25 par un mot ne passe pas devant une fonction trouvée par le sens et le BM25), puis le juge du
banc sur sa base `feuille-lot-1636`.
