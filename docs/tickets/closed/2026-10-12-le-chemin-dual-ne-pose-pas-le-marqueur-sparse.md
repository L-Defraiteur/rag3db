# Le chemin dual ne posait pas le marqueur sparse : chaque passe recalculait les mêmes morceaux

- **État** : fermé dans le lot qui l'a ouvert (12 octobre 2026)
- **Gravité** : coût (du temps d'embarquement perdu à chaque passe, sans fin) et une dette qui ne
  se solde jamais
- **Atteignable en service** : oui, pour toute entité à signaux `vector` et `sparse` embarquée par un
  modèle dual (BGE-M3 : les gabarits `memory`, `notebook`, `validated-result`)
- **Touche rag3weaver** : oui (`dataflow/record_nodes.rs`)
- **Ouvert le** : 12 octobre 2026, session everything declarative, en cartographiant `EmbedNode`
  pour l'embarquement hors du verrou
- **Pour** : everything declarative

## Ce que c'était

`EmbedNode` a trois chemins : dense seul, sparse seul, et dual (un appel rend les deux). Sur le
chemin sparse seul, le vecteur creux est inséré dans son handle au décalage de la ligne, puis
`_sparse_hash` est posé. Sur le chemin dual, le vecteur creux était inséré, mais **le marqueur
n'était jamais posé**. Le morceau restait dû en sparse : chaque passe d'embarquement le réclamait,
le recalculait par le dual (dense et sparse), et le laissait dû de nouveau.

## Le témoin, rouge d'abord

`e2e_invariant_des_vecteurs::le_chemin_dual_pose_le_marqueur_sparse` : 40 fiches à trois signaux,
un embarqueur dual. La première passe embarque les 40 ; avant le correctif, la seconde les
recalculait toutes (« la seconde passe recalcule 40 morceaux »). Après, la seconde n'a plus rien à
faire.

## Le correctif

Un helper unique, `record_nodes::poser_le_creux` : relire le décalage de chaque ligne, insérer le
vecteur dans le handle, puis poser `_sparse_hash`, le marqueur après son vecteur. Les deux chemins
(sparse seul et dual) l'utilisent, ainsi que l'écriture de l'embarquement hors du verrou
(`Catalog::poser_les_vecteurs`). Le chiffre sur un vrai index (la mémoire longue) est dans la
page 04, §9.
