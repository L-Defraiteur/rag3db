# La maintenance nocturne compte les non-candidates comme des écritures perdues

- **État** : ouvert (21 novembre 2026).
- **Gravité** : fausse alarme. L'état d'index dit « des écritures ont été perdues » alors qu'il n'en manque aucune ;
  une vraie perte, qui se dirait de la même façon, s'y noie.
- **Atteignable en service** : oui, sur toute base sous la règle des non-candidats (les conteneurs repliés, règles 13),
  dès sa première maintenance nocturne.
- **Touche rag3weaver** : oui (la note des comptes de la maintenance), pas lucivy.
- **Famille** : produit
- **Pour** : lucivy (confié par c7)
- **Signalé par** : la recherche, au `daemon_status` de la tablette (21 novembre 2026, 06 h 2x), relayé par c7

## Ce qu'on voit

Sur `code-rag3db-depot` : « plein texte de « Scope » au relevé d'il y a 167 min : 102191 documents pour 113518 lignes,
dont 0 orphelin — des écritures ont été perdues ». La maintenance de 03 h 30 donne pour ScopeContent `graph_docs`
113 518 et `live_docs` 102 191.

## La cause

Deux comptes différents sous un même nom :
- à la fin d'un index, la garde compare les documents aux lignes **candidates** (`Catalog::comptes_du_plein_texte`,
  `candidate_rows_of`). Le repassage de la veille l'a vérifiée sans écart ;
- la maintenance nocturne réécrit ensuite la note (`refresh_noted_counts_from_maintenance`) avec `rows = graph_docs`,
  c'est-à-dire **toutes** les lignes de la table, non-candidates comprises. Les ~11 300 conteneurs repliés écartés par
  la règle n'ont volontairement pas de document.

Aucune écriture n'est perdue, et rien n'est purgé à tort : un écarté n'a pas de document. La maintenance suivante ne
répare pas : elle réécrit le même faux écart chaque nuit.

## Le correctif

La note de maintenance compte les lignes candidates, comme la garde de l'index. Témoin rouge d'abord : une base sous
la règle, une maintenance nocturne, et l'état ne doit dire aucun écart.
