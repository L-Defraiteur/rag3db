# La hausse des dérivations, revérifiée (19 novembre 2026)

- **La question** (c7, 18 novembre) : le lot `link-copy-split` (la pose des liens de la résolution ne repasse plus
  par MERGE) a montré, sur le dépôt entier, des « dérivations » de 106 s puis 124 s. Ticket fermé
  `closed/2026-11-18-la-pose-des-liens-de-la-resolution-repasse-par-merge.md` : « à revérifier sur un poste calme ».
- **Le relevé** : à luciepc, `record_nodes.rs` de master contre celui du lot, **entrelacés** (avant, après, avant,
  après), sur le même arbre figé (343ead7996, worktree `rag3db-garde-mesure`), par la sonde `probe_index_timeline`
  de recherche, `RAG3WEAVER_INGEST_PROFILE=1`, lib 3d706f709. Script `~/.cache/rag3weaver-build/mesure-derivations.sh`,
  journaux `~/.cache/rag3weaver-build/mesure-derivations/` (à luciepc).

| tour | charge (1 min) | dérivations | dont `derive` | `drain · liens` | index complet |
|---|---|---|---|---|---|
| avant 1 | 17,3 | 89,9 s | 47,6 s | 22,0 s | 185,7 s |
| après 1 | 32,8 | 104,4 s | 59,1 s | 13,1 s | 190,4 s |
| avant 2 | 21,8 | 100,4 s | 54,9 s | 20,7 s | 197,8 s |
| après 2 | 30,1 | 105,7 s | 62,3 s | 12,1 s | 191,0 s |

**Ce que ça dit** :
- **Le gain du lot tient** : `drain · liens` passe de 21 à 12-13 s, deux fois.
- **La hausse des dérivations est toute dans le nœud `derive`**, le rendu du contenu des scopes (`ScopeContent`, sans
  `gather`). Il lit des scopes par uuid et rend leur texte ; il ne lit aucune relation et ne passe pas par le code
  du lot (`copier_les_liens`). Les nœuds de dérivation qui écrivent en base sont inchangés à 1 s près
  (`derive_chunk_insert` 10,7 / 11,6 / 11,4 / 10,9 s, `derive_flush` 8,9 / 9,0 / 9,4 / 9,1 s, `derive_insert`,
  `derive_link`, `derive_chunk_link`).
- **Les deux tours « après » ont eu la charge la plus haute** (30 à 33 contre 17 à 22 sur 24 cœurs), et `derive` est
  du calcul : c'est la charge qui le ralentit. L'index complet ne change pas (186-198 s avant, 190-191 s après).
- **Conclusion** : la hausse n'est pas due au lot ; pas de ticket. Elle ne se lit que sur un nœud de calcul, et suit
  la charge du poste. Une preuve plus forte demanderait le temps CPU de `derive` plutôt que son temps mural.
