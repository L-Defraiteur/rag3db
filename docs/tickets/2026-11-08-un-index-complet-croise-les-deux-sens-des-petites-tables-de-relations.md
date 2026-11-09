# Un index complet croise les deux sens des petites tables de relations

- **État** : ouvert (8 novembre 2026, session hotfixs ; premier chantier, devant le lot B)
- **Famille** : moteur
- **Gravité** : perte — après une réindexation complète, des relations sont fausses en service :
  rattachées au mauvais nœud, présentes dans un seul sens, ou vues deux fois
- **Atteignable en service** : oui. Un `index confirm` complet sur une base de code les produit, de
  façon déterministe. Lucie ne lance aucun index complet sur ses bases jusqu'au correctif.
- **Touche rag3weaver** : par le moteur (l'index de code)

## Ce que c'est

Une copie saine de code-lourd, prise pendant la coupure de l'hôte de luciepc le 8 novembre à
12 h 06 m 30 s, est à 0 écart sur ses 36 tables de relations (relevé ancré, lib cbc06c3e1). G y
joue un `index confirm` complet, puis son contrôle des deux sens (`check_rel_directions`, lecture
des deux rangements par les internes). Les écarts sont **les mêmes sous 17a094d12 et sous
cbc06c3e1, au chiffre près** :

- INHERITS_FROM : 650 au sens avant, 658 au sens arrière ; 1 d'un seul côté, 11 vues deux fois, 13
  aux bouts différents ;
- LOCKS : 544 / 546 ; 3 vues deux fois, 5 aux bouts différents ;
- QUERIES : 5 d'un seul côté, 6 vues deux fois, 9 aux bouts différents.

Sur code-principal, sous cbc06c3e1 : QUERIES, IMPLEMENTS (331 / 334, `line` et `resolution` à deux
valeurs selon le sens) et LOCKS (425 / 436). Les grosses tables (MENTIONS, CONSUMES, DEFINES, les
`_CHUNKED_FROM`) sont égales.

Le défaut précède donc les lots du 7 et du 8 novembre. C'est la famille du ticket fermé de LOCKS du
25 octobre : des lignes vides lues comme des relations (« la relation 0 vue deux fois », des bouts à
l'offset 0), des séries de relations consécutives rattachées au mauvais nœud vers une même
destination, des relations d'un seul côté. Le témoin (B) du 7 novembre
(`late_rel_table_rewrites_test.cpp`) ne le couvre pas.

## Ce que montre la base abîmée

Sur INHERITS_FROM, le sens avant depuis les nœuds 5356 à 5359 est **identique** dans la copie saine
et dans l'instantané d'après l'index : les relations 357 à 360 vont vers RecordBase (5354), avec les
mêmes identifiants. L'index ne les a pas recréées. Le sens arrière de RecordBase, lui, rend dans
l'abîmée une ligne vide (0|0), puis 467, 468 et 469, des relations que le sens avant envoie vers 3223,
puis 564 ; plus aucune de 357 à 361. La liste arrière d'un nœud **que l'index n'a pas touché** pointe
après le point de reprise sur la portion de rangées d'une autre destination.

Le run 2b de G, sous dadbf0041, rend les mêmes chiffres que 1b sous 17a094d12.

## L'hypothèse du débordement de région (réfutée le 8 novembre au soir)

`InMemChunkedCSRHeader::populateRegionCSROffsets` (`src/storage/table/csr_chunked_node_group.cpp`)
range les longueurs du nouvel en-tête à la suite, depuis le début gauche d'avant de la région. Puis il
force la fin du dernier nœud au bord droit d'avant (`oldRightCSROffset`). Le dépassement n'est gardé
que par un `KU_ASSERT`, absent en Release. La stabilité du bord gauche, dans
`CSRNodeGroup::checkpointInMemAndOnDisk`, ne l'est aussi que par un `KU_ASSERT`.

Si la somme neuve des longueurs dépasse l'ancienne capacité de la région, les fins des nœuds d'avant
le dernier passent dans la région suivante, et `checkpointColumnInRegion` y écrit leurs rangées. La
région suivante n'est pas réécrite et garde ses décalages : ses nœuds intacts lisent les rangées d'un
autre, et la longueur du dernier nœud débordant ne colle plus à son intervalle.

Ce qui devait l'empêcher, c'est `isWithinDensityBound`. Mais il ne lit pas les longueurs que `populate`
va ranger : il reconstitue la taille neuve comme la somme des longueurs de l'ancien en-tête plus le
`sizeChange` des feuilles, compté en deux passes (`collectInMem…` ajoute les insertions vivantes,
`collectOnDisk…` retire `getNumDeletionsForNodeInPersistentData`). Rien ne vérifie que cette somme
égale celle du nouvel en-tête. Un écart de compte ouvre le débordement.

**Les deux gardes proposées**, indépendantes de la cause :
1. calculer la densité sur les longueurs du nouvel en-tête, celles que `populate` va ranger ;
2. faire échouer le point de reprise par une erreur nommée, en Release, si un débordement reste,
   au lieu d'écraser le bord droit. Un point de reprise raté se reprend proprement ; une relation
   corrompue se lit fausse pour toujours.

**La preuve en cours** : une lib d'étude (`etude-debordement-csr-2`, à ne pas atterrir), master plus
les deux `KU_ASSERT` recopiés sur la sortie d'erreur (`[csr-déborde]`, `[csr-bord-gauche]`), avec un
`rag3weaver-backend` bâti contre elle. Un `index confirm` tracé (`RAG3WEAVER_TRACE_CYPHER=1`) tourne
sur une copie de la copie saine, avec son propre service granite-278m sur card2 (port 7882).

## Les témoins à un seul écrivain, tous verts

Branche `reindex-complet-temoin`, `late_rel_table_rewrites_test.cpp` :
- `BothDirectionsStayEqualUnderAFullReindex` (la forme d'un index confirm) ;
- `TwoFlushesOfOneNodeTableKeepBothDirectionsEqual` (deux versements d'une même table) ;
- `BothDirectionsStayEqualWhenFilesAreReindexedWithTheirNodes` (DETACH DELETE, nœuds recréés ailleurs) ;
- `AListThatGrowsLeavesItsUntouchedNeighboursInPlace` (une liste qui grossit au milieu de voisines
  intactes).

La garde de densité suit bien une croissance qu'elle voit. Le défaut demande un mélange d'opérations
que `sizeChange` compte mal.

## Les pièces (luciepc)

- la copie saine : `~/.cache/rag3db-locks/coupure-12h06/code-lourd` et `code-principal` ;
- le départage de G : `~/.local/share/rag3weaver/instantanes/departage-1b-17a` et son
  `-controle` (sous 17a094d12) ; `preuve-cbc06c3e1` et `preuve-cbc06c3e1-controle` (sous
  cbc06c3e1, 13 h 03, deux `index confirm`) ; `departage-2-dadbf` et `-2b-dadbf` (sous dadbf0041) ;
- le contrôle : `check_rel_directions` et ses journaux `*.log`, dans chaque dossier `-controle`.

## Ce que les témoins à un seul écrivain n'ont pas reproduit

Deux témoins de la branche `reindex-complet-temoin` (053d14d3a) sont verts. Le premier rejoue la
forme d'un `index confirm` : une table de liens tardive ; des nœuds neufs ; DELETE de toutes les
arêtes puis COPY dans la même transaction ; un SET sur une partie des relations copiées ; trois tours
avec points de reprise. Le second verse deux fois une même table de nœuds. Il leur manque le volume
et des groupes CSR persistés de longue date, avec des régions à réécrire.

## Le rejeu exact : c'est le point de reprise (9 novembre, nuit)

- **La trace.** Un binaire d'étude (branche `trace-exacte-9bae`, 2b73c4ff7, à ne pas atterrir : 9bae60786
  plus `RAG3WEAVER_TRACE_CYPHER_FICHIER`) écrit chaque instruction avec ses paramètres en littéraux
  Cypher, et copie le CSV de chaque COPY. L'index confirm de la copie saine, sous 9bae60786 (hôte
  7992, service 7882), donne les mêmes écarts que tous les runs précédents, compte pour compte :
  INHERITS_FROM 650 au sens avant et 658 au sens arrière, QUERIES 20 relations abîmées, LOCKS 544 et 546.
  La trace compte 3 178 instructions, aucun BEGIN, 16 COPY.
- **Le rejeu.** `rejoue` (un programme C++ lié à 9bae60786 qui reprend la fonction de contrôle de
  `check_rel_directions`) joue la trace sur une copie neuve, sans aucune erreur. Le contrôle est vert
  jusqu'à la ligne 777, et INHERITS_FROM est abîmé après la ligne 797. Ce sont les mêmes relations
  que dans la base de Lucie : 29:467 à 469 vers 11:3223, et 29:357 à 360 vers 11:5354. La ligne 797
  est un `UNWIND … MERGE (a:Scope)-[r:MENTIONS]->(b:Symbol)` de 5 000 relations, qui n'écrit pas dans
  INHERITS_FROM.
- **Le point de reprise.** « Lignes 1 à 796, puis `CHECKPOINT` » rougit de même, alors que l'état lu
  avant ce point de reprise est vert. Ce ne sont donc ni les MERGE de la table abîmée ni celui de la
  ligne 797, qui déclenchait le point de reprise automatique : c'est le point de reprise d'un état
  encore en mémoire qui écrit des listes CSR fausses.
- **Les sondes du cœur** sur l'état rouge : aucune page à la fois libre et tenue par un segment, fin
  du dernier segment 325537 sous l'étendue 326011, pas de creux. Ni `returnOwnerlessPages` ni la
  troncature.
- **Les pièces (luciepc).** La trace : `~/.cache/rag3db-tickets-notes/trace-exacte-9bae/cypher.txt`,
  avec `copies/`. La recette : `d-796-checkpoint.txt`. Le rejoueur :
  `~/.cache/rag3db-mesure-g5/trace-9bae/rejoue`. L'instantané de l'index : `hotfixs-trace-9bae` et
  `-controle`. Le cœur passe la recette sous ASAN (`~/.cache/rag3db-moteur-notes/deux-sens-asan/`).

## La suite

1. La dichotomie sur K pour « lignes 1 à K, puis `CHECKPOINT` » : la plus petite K rouge désigne
   l'écriture dont le point de reprise est fautif (en cours, le 9 novembre ; luciepc avait le disque
   saturé à 2 h).
2. Un témoin réduit, rouge, au cœur : la base d'avant cette écriture, l'écriture, `CHECKPOINT`.
3. Le correctif, dans la fusion du CSR en mémoire avec le persistant au point de reprise (piste du
   cœur : une région réécrite à côté d'une région qui ne l'est pas, ou les longueurs du sens arrière
   lues avant la compaction des lignes mortes).
