# L'équipier sous Gemini : le contenu ou un compte rendu (14 novembre 2026)

À la demande de c7, pour archi : la demande du ticket de tty, « Présente-toi en quelques lignes : un
titre, une liste à puces de trois choses que tu sais faire, et un mot en gras. », jouée dix fois
sans le correctif et dix fois avec.

| côté | commit | la boîte porte le contenu | un compte rendu seul |
|---|---|---|---|
| sans le correctif | 47b1955de, moins ba2994a97 | **10 / 10** | 0 / 10 |
| avec le correctif | 47b1955de | **10 / 10** | 0 / 10 |

- **Le juge** : celui de `l_equipier_rend_le_texte_demande_et_non_un_compte_rendu`
  (`tests/e2e_equipier.rs`). La boîte doit porter, de l'équipier, un titre Markdown, trois puces et
  un gras.
- **Le côté « sans »** : le correctif est retiré de la même tête, et non pris sur master, qui ne
  porte pas le cas.
- **Le montage** : à luciepc, sous `poste lourd`, le 14 novembre de 23 h 58 à 0 h 10. Vertex, le
  modèle par défaut du test, des appels en série. Aucun quota rencontré.
- **Ce qu'on voit dans la boîte, des deux côtés** : le contenu arrive en `report`, et le
  `step_done` porte la phrase de compte rendu à côté.
- **Les sorties** : `~/.cache/rag3weaver-build/banc-equipier/20261114-2358/`, à luciepc (un
  journal par tirage) ; le script est `equipier-tirages.sh`.
