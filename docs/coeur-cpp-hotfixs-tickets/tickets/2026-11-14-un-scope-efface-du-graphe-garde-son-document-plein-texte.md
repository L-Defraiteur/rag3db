# Un Scope effacé du graphe garde son document plein texte

- **État** : ouvert (14 novembre 2026, session hotfixs, relevé par la sonde de cohérence demandée par c7)
- **Famille** : produit
- **Gravité** : réponse fausse possible. Un document orphelin peut remonter dans une recherche avec un
  `_node_id` qui ne mène à rien, ou, si l'offset est repris plus tard par une nouvelle ligne, qui mène à
  une autre ligne (remarque de lucivy). Il pèse aussi dans l'index.
- **Atteignable en service** : oui. 8 809 orphelins dans la base de dépôt de Lucie (copie du 13 nov.).
- **Touche rag3weaver** : oui (le plein texte de l'index de code : `extension/rag3weaver/src/fts_handle.rs`
  et le chemin qui efface les Scope)

## Ce que c'est

rag3weaver range chaque Scope dans le plein texte (`Lucivy_Scope`) sous un `_node_id` qui vaut l'offset
de sa ligne dans le graphe. Sur la copie `avant-835d5298f/code-rag3db-depot` (13 nov. 2026, 20 h 07,
hôte arrêté, base et `.fts` ensemble) :

- les 79 662 Scope du graphe ont chacun leur document ;
- **8 809 documents vivants désignent un offset où aucun Scope n'existe**, tous dans des places vides
  du graphe ;
- aucun `_node_id` ne dépasse l'offset maximal (93 718), aucun n'est en double.

Ces documents sont donc ceux de Scope effacés du graphe, dont le document n'a pas été supprimé. Le
détail et la méthode sont dans le ticket de lucivy
`docs/lucivy/tickets/2026-11-13-le-plein-texte-du-depot-pese-cent-fois-ses-sources.md`, section « Le
départage des comptes de Scope ».

## Ce qu'il faut pour le fermer

1. Trouver le chemin d'effacement qui oublie le plein texte. Pistes : le `DETACH DELETE` des Scope des
   fichiers disparus en fin d'`index confirm`, une réindexation qui recrée un Scope sous une autre
   place, un nettoyage d'absents (`_absent_since`).
2. Un témoin rouge : effacer un Scope par ce chemin, puis compter un document de plus que de Scope.
3. Le correctif, puis la décision de Lucie sur les bases existantes (purge ou reconstruction du `.fts`).
4. Vérifier si une recherche rend ces orphelins aujourd'hui, et ce qu'elle en fait.
