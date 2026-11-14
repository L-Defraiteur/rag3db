# Un Scope effacé du graphe garde son document plein texte

- **État** : ouvert (14 novembre 2026, relevé par hotfixs avec la sonde de cohérence demandée par c7)
- **Pour** : recherche, premier lecteur, car c'est son chemin de réindexation (réattribution par c7)
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

## Les deux questions ouvertes

1. **Les recherches rendent-elles ces orphelins ?** Si oui, que fait l'appelant d'un `_node_id` qui ne
   mène à aucun Scope, ou à un Scope né plus tard à la même place ?
2. **Quel chemin d'effacement oublie le `.fts` ?** Trois candidats :
   - la suppression d'un fichier : le `DETACH DELETE` des Scope des fichiers disparus, en fin
     d'`index confirm` ;
   - la reprise après un arrêt, quand un index interrompu est repris ;
   - le rebâti, quand un Scope est recréé à une autre place.

## La copie où rejouer

Au portable, la copie fidèle de la mesure : `~/.cache/rag3db-locks/sonde-coherence/` (base, `.fts` et
extensions de l'instantané `avant-835d5298f/code-rag3db-depot`, pris hôte arrêté). Le `.fts` de cette
copie a été ouvert une fois par un writer lucivy : pour une nouvelle lecture, recopier depuis
l'instantané `~/.local/share/rag3weaver/instantanes/avant-835d5298f/code-rag3db-depot/`, jamais depuis
la base vivante.

Les outils :
- le graphe : `MATCH (n:Scope) RETURN offset(id(n))`, en lecture seule ;
- le `.fts` : le test lucivy `diag_node_ids_vivants` (branche lucivy `diag-mots-perdus-tous-champs`,
  1ded6a0), avec `DIAG_TABLE=<copie>/code-rag3db-depot.rag3db.fts/Lucivy_Scope` et
  `DIAG_SORTIE=<fichier>` ;
- les orphelins : les `_node_id` de la sortie absents des offsets (`comm -13`).

## Ce qu'il faut pour le fermer

1. Répondre aux deux questions.
2. Un témoin rouge : effacer un Scope par le chemin fautif, puis compter un document de plus que de
   Scope.
3. Le correctif, puis la décision de Lucie sur les bases existantes (purge des orphelins ou
   reconstruction du `.fts`).
