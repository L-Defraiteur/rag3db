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

**Fait neuf : une seconde table est touchée.** Dans code-rag3db-depot, **124 documents File** n'ont plus
de fichier dans le graphe. Le même chemin d'effacement oublie donc aussi le plein texte des File, pas
seulement celui des Scope.

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

## La purge sur une copie (hotfixs, 14 novembre 2026, 19 h 20)

Demandée par c7. C'est une copie reflink neuve du `.fts` de l'instantané `avant-835d5298f/code-rag3db-depot`
(hôte arrêté), et rien n'a touché une base de Lucie. Les listes viennent de la sonde ci-dessus :
8 809 `_node_id` Scope et 124 File. L'outil est le test lucivy `diag_purge_orphelins`, sur la branche
lucivy `purge-orphelins-depot` (2a23619, sur main fdf20d4). Ce n'est pas le repli conçu par recherche,
qui corrige `DeleteRecordNode` pour l'avenir : c'est une purge en lot.

Le test fait, dans l'ordre :
1. `index_report` avant ;
2. un banc de requêtes `contains` qui relève **tous** les `_node_id` touchés (recherche non tronquée) ;
3. `delete_by_node_id` de chaque orphelin, la validation et la fin des fusions ;
4. `index_report` après, et le banc rejoué ;
5. la réouverture de la table.

| table | documents avant → après (rouvert) | orphelins effacés | touchés par le banc | supprimés en segments | octets d'index | temps |
|---|---|---:|---:|---|---|---:|
| Lucivy_Scope | 88 471 → 79 662 (79 662) | 8 809 | 8 770 | 11 879 → 20 688 | 4 274 396 483 → 4 274 397 771 | 921 ms |
| Lucivy_File | 7 927 → 7 803 (7 803) | 124 | 124 | 9 → 133 | 7 315 005 → 7 315 257 | 811 ms |

Après la purge, les documents égalent les lignes du graphe (79 662 Scope, 7 803 File).

**Le banc** : sur chaque requête, les hits d'après sont exactement ceux d'avant moins les orphelins, ni un
de plus ni un de moins.

| Scope, `content` | hits avant | dont orphelins | après |
|---|---:|---:|---:|
| return | 33 930 | 8 412 | 25 518 |
| fn | 28 245 | 8 099 | 20 146 |
| test | 21 767 | 8 198 | 13 569 |
| self | 16 576 | 6 665 | 9 911 |
| index | 15 220 | 7 968 | 7 252 |
| impl | 11 987 | 5 980 | 6 007 |
| commit | 8 651 | 5 822 | 2 829 |
| catalog | 6 182 | 1 598 | 4 584 |
| scope | 5 579 | 2 312 | 3 267 |
| async | 5 088 | 3 004 | 2 084 |
| mutex | 1 983 | 154 | 1 829 |
| lucivy | 1 115 | 244 | 871 |

File, champ `path` : dix requêtes, de 0 à 122 orphelins chacune (`docs` 2 395 → 2 273, `md`
1 859 → 1 737), toutes identiques au reste près.

**Ce que ça dit.**
- **Question 1 répondue : oui, les recherches rendent les orphelins, et en masse.** Sur `commit`, deux
  hits sur trois sont des orphelins ; sur `index`, plus de la moitié. Le plafond de résultats de
  rag3weaver les classe avec les autres. Un hit orphelin ne mène à aucun Scope, et il peut chasser du
  top-k un Scope vivant. La gravité « réponse fausse possible » est donc vraie en service.
- **La purge ne libère pas l'espace par elle-même.** Les documents passent en supprimés dans leurs
  segments, aucune fusion ne s'est déclenchée, et les octets d'index sont restés au même chiffre. Le
  gain se lira après une fusion ou un `compact`.
- **La purge est sûre et rapide** : moins d'une seconde par table, survit à la réouverture, ne touche
  aucun document vivant.

### Le gain après compactage (19 h 34)

L'outil est le test `compacter` du même fichier (35b525f) : `compact` puis la fin des fusions, sur la copie
purgée, puis sur une copie neuve **non purgée** prise comme témoin. Le témoin sépare le gain de la purge
de celui des documents supprimés qui étaient déjà dans les segments. Le banc (4 requêtes sur Scope, 3 sur
File) rend les mêmes `_node_id` avant et après la fusion, dans les quatre cas.

| table | copie | documents | segments (supprimés) avant → après | octets d'index avant → après | index / texte stocké |
|---|---|---:|---|---|---|
| Lucivy_Scope | purgée | 79 662 | 21 (20 688) → 4 (0) | 4 274 397 771 → **1 301 350 311** | 24,92 → 7,59 |
| Lucivy_Scope | témoin non purgé | 88 471 | 21 (11 879) → 4 (0) | 4 274 396 483 → 4 076 884 835 | 5,00 → 4,77 |
| Lucivy_File | purgée | 7 803 | 15 (133) → 4 (0) | 7 315 257 → 7 228 744 | 13,70 → 13,53 |
| Lucivy_File | témoin non purgé | 7 927 | 15 (9) → 4 (0) | 7 315 005 → 7 293 365 | 13,39 → 13,35 |

La fusion a pris 15 s sur la copie purgée et 41 s sur le témoin.

**Ce que ça dit.**
- **Les 8 809 orphelins pèsent 2,78 Go sur 4,08** : 68 % de Lucivy_Scope compacté, pour 10 % des
  documents. Le compactage seul n'en rend que 0,2 Go.
- **Ce sont de très gros documents.** D'après les rapports index / texte stocké, le texte stocké vaut
  environ 171 Mo après la purge et 855 Mo dans le témoin. Les orphelins portent donc environ 684 Mo de
  texte, 80 % du texte de la table : environ 78 Ko par orphelin, contre environ 2 Ko par Scope vivant.
  C'est une bonne part de ce que mesure le ticket lucivy
  `2026-11-13-le-plein-texte-du-depot-pese-cent-fois-ses-sources`. Ce sont probablement les Scope de
  fichiers générés et minifiés, réindexés à chaque rebâti ; ce n'est pas vérifié ici.
- File ne gagne rien d'utile : 124 petits documents.

Reste la décision de Lucie pour les bases existantes. Sur la base vivante, il faudrait recalculer les
listes, hôte arrêté, avant d'effacer : les chiffres ci-dessus sont ceux de la copie du 13 novembre.

## La purge des bases de luciepc (fenêtre du 15 novembre 2026, 0 h 51 et 1 h 11)

Dans la fenêtre de la réparation des deux index de relations (ticket du cœur
`2026-11-14-les-deux-index-d-une-table-de-relations-ne-s-accordent-plus`), sous la même fermeture qui tient et
le même instantané. Les orphelins sont recomptés à froid sur la base fermée et déjà réparée :

| base | table | lignes du graphe | documents avant → après | orphelins effacés | octets d'index après compact |
|---|---|---:|---|---:|---|
| code-lourd | Scope | 82 855 | 82 958 → 82 855 | 103 | 710 744 829 → 695 544 954 |
| code-principal | Scope | 65 914 | 66 016 → 65 914 | 102 | 600 875 379 → 591 009 441 |

File, Library et Symbol n'ont aucun orphelin dans les deux bases. Le banc lucivy rend, sur chaque requête,
les mêmes documents moins les orphelins. Le banc par l'hôte ne perd aucun résultat.

**Reste code-rag3db-depot** (portable, 9 701 Scope et 127 File dans l'instantané du 14 à 13 h 40). Sa fenêtre
attend un binaire qui porte la fermeture qui tient sur l'hôte du portable.

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
