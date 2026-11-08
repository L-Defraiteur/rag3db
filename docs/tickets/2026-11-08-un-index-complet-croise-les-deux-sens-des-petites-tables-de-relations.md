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

## La suite

1. Capturer la suite exacte des instructions d'un `index confirm` (`RAG3WEAVER_TRACE_CYPHER=1`) sur
   une copie de la copie saine.
2. La rejouer seule contre le moteur, sur une autre copie, jusqu'aux écarts. Puis la réduire, par
   le chemin d'abord (un DELETE puis un COPY de relations dans la même transaction, sur une table
   au disque), et par les commits d'avant 17a094d12 si le chemin ne suffit pas.
3. Un témoin déterministe, rouge, puis le correctif.
