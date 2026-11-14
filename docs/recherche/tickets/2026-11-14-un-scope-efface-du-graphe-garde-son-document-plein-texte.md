# Un Scope effacé du graphe garde son document plein texte

- **État** : ouvert (14 novembre 2026, relevé par hotfixs avec la sonde de cohérence demandée par c7)
- **Pour** : recherche, après sa page des règles d'exclusion. C'est son chemin de réindexation (DETACH DELETE de fin d'index, déplacement d'un Scope, nettoyage des absents) ; réattribution par c7
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

## Les bases de Lucie (hotfixs, 14 novembre 2026)

C'est la même sonde, sur les quatre bases, toutes tables indexées. Pour chaque base : une copie
fidèle neuve de l'instantané `avant-835d5298f`, pris par G hôtes arrêtés le 13 novembre (20 h 06 à
luciepc, 20 h 07 au portable), avec la base, les extensions et le `.fts` ensemble. Le graphe est lu
en lecture seule par la lib 835d5298f, et le `.fts` par `diag_node_ids_vivants`, sous `poste lourd`.
Le script est `sonde-coherence-tables.sh` (au portable dans `~/.cache/rag3db-locks/`, à luciepc dans
`~/.cache/rag3db-tickets-notes/`).

| base | table | lignes du graphe | documents vivants | supprimés en segments | **documents sans ligne** | lignes sans document |
|---|---|---:|---:|---:|---:|---:|
| code-rag3db-depot | Scope | 79 662 | 88 471 | 11 879 | **8 809** | 0 |
| code-rag3db-depot | File | 7 803 | 7 927 | 9 | **124** | 0 |
| code-rag3db-depot | Symbol | 86 152 | 86 152 | 0 | 0 | 0 |
| code-rag3db-depot | Library | 448 | 448 | 0 | 0 | 0 |
| code-lourd | Scope | 82 855 | 82 958 | 4 572 | **103** | 0 |
| code-lourd | File | 7 278 | 7 278 | 862 | 0 | 0 |
| code-lourd | Symbol | 84 928 | 84 928 | 0 | 0 | 0 |
| code-lourd | Library | 464 | 464 | 0 | 0 | 0 |
| code-principal | Scope | 65 914 | 66 016 | 2 310 | **102** | 0 |
| code-principal | File | 6 314 | 6 314 | 0 | 0 | 0 |
| code-principal | Symbol | 73 373 | 73 373 | 0 | 0 | 0 |
| code-principal | Library | 398 | 398 | 0 | 0 | 0 |
| memoire-longue | Apport, Fiche, Memory, Ref, RefType, Source, Subject | 172, 19, 1, 102, 6, 45, 1 | autant | 0 (Ref : 2) | 0 | 0 |

**Lecture.**
- **Aucune ligne du graphe n'est sans document**, dans aucune table : le plein texte ne manque
  jamais, il est en trop.
- **Les orphelins ne touchent que Scope et File**, dans les trois bases de code ; memoire-longue est
  propre.
- Dans code-lourd et code-principal, il y a **autant d'orphelins Scope que de places vides de Scope**
  dans le graphe (103 et 102). Ce sont les Scope des fichiers disparus, que l'`index confirm` efface
  en fin de passe : la trace exacte du 8 novembre montre ce `DETACH DELETE` sur 103 Scope dans
  code-lourd. Le journal de l'index le signalait déjà : « plein texte : écart entre documents et lignes
  — Scope : 82958 documents pour 82855 lignes ».
- code-rag3db-depot accumule bien davantage (8 809 Scope et 124 File), comme une base que l'hôte a
  réindexée au fil de nombreux redémarrages. C'est le chemin que recherche a reproduit : un Scope
  retiré par une session qui ne l'a pas écrit, le cache uuid → offset étant vide après le
  redémarrage.

**La portée de la purge** : 8 809 + 124 documents dans code-rag3db-depot, 103 dans code-lourd, 102
dans code-principal, rien dans memoire-longue. Ce sont les chiffres de ces copies du 13 novembre ;
les bases vivantes ont pu bouger depuis.

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
