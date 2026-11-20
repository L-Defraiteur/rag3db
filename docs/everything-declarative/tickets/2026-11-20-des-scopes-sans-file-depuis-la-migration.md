# Des scopes sans `File` depuis la migration : la cause n'est pas trouvée

- **État** : ouvert (20 novembre 2026)
- **Famille** : produit (l'écrivain du code, la migration vers les versions)
- **Gravité** : une base garde des scopes qu'aucune synchronisation ne retirait. Le lot `orphan-scopes-retired` les
  retire désormais ; ce ticket porte la **cause**, qui reste inconnue.
- **Atteignable en service** : oui, constaté sur la base vivante du dépôt.
- **Touche rag3weaver** : oui.
- **Pour** : everything declarative
- **Signalé par** : G et codeparsers (relevés du 20 novembre 2026), lu par ED.

## Ce qu'on voit

Sur la base du dépôt (copie de codeparsers de 15 h 38, `mesures/poids-app-js-20261120-1538`), pour
`extension/rag3weaver/templates/ide/app/app.js` :
- **aucune ligne `File`**, sous aucune version ;
- **160 `Scope`**, tous `_depuis = 0`, `_jusqu_a` nul, `_couche = 'backend'`, `source = 'file'` : 37 `variable`
  et 123 `method`. Les 37 variables portent chacune la ligne minifiée entière de 133 Ko, soit 4,5 Mo ;
- **aucun `DEFINED_IN`** vers une `File` ;
- `_snapshot` : 85 sans instantané, et 75 répartis sur 13 instantanés différents.

Ses voisins du même dossier (`app.css`, `index.html`, `bundle.json`, `flux-worker.js`) ont bien leurs `File`
versionnées. La règle des minifiés (`index_exclude.minified_line_max_bytes: 5000`) est au manifeste depuis le
14 novembre, et le dernier index complet date du 20 novembre, 07 h 11.

## Ce qui est corrigé, et ce qui ne l'est pas

**Corrigé dans le lot** : la synchronisation complète lisait les fichiers indexés dans les seules lignes `File`. Un
chemin qui a des scopes visibles mais pas de `File` n'était donc ni écarté ni parti, à jamais. Elle lit maintenant
aussi les chemins des scopes visibles de la source (`chemins_orphelins`). Un chemin sans `File` que la source ne
retient plus part avec ses scopes.

**Pas trouvé** : **comment la `File` est partie sans ses scopes**. Les marques d'instantanés et le rang 0 pointent
vers l'époque des instantanés, puis la migration vers les versions. Les pistes, à départager sur une base d'avant la
migration :
1. le chemin des instantanés écarte un fichier devenu minifié (règle du 14 novembre) : sa `File` est retirée, mais
   ses scopes, marqués par d'autres instantanés, restent ;
2. la migration vers les versions copie des `Scope` dont la `File` n'existe plus ;
3. la réindexation de la montre (`reingest_files`) quand l'admission refuse un fichier modifié.

## Ce qu'il faut pour le fermer

Un témoin rouge qui fabrique des scopes sans `File` par le seul chemin du produit, puis le correctif de ce chemin.
