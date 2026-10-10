# Les étiquettes d'un carnet peuvent avoir été échangées, sans moyen sûr de le voir

- **État** : fermé le 5 octobre 2026 — décision de Lucie : aucun carnet en usage réel, rien à faire sur l'existant ; le correctif `25b3b45dc` suffit pour la suite.
- **Gravité** : perte (une donnée fausse que rien ne rebâtit).
- **Atteignable en service** : oui, sur toute base de carnet écrite avant `25b3b45dc`.
- **Touche rag3weaver** : oui (gabarit `templates/backends/notebook`).

## Ce que c'est

Le défaut des chaînes au point de reprise (ticket
`2026-10-04-propriete-de-chaine-faussee-dans-le-sens-direct`, corrigé par `25b3b45dc`)
atteint aussi les colonnes **liste de chaînes** des nœuds. Le carnet en a deux :
`Note.labels` et `Snapshot.labels` (`schemas/note.json`, `schemas/snapshot.json`). Une
mise à jour ou un ajout de notes après un premier point de reprise, puis un point de
reprise, a pu donner à une note les étiquettes d'une autre de la même région. Le correctif
empêche les suivants ; il ne répare pas ce qui est écrit.

Ce que l'utilisatrice verrait : `search_notes` et `get_note` rendent les étiquettes d'une
autre note ; un filtre `has_any`/`has_all` rend les mauvaises notes ou en omet.

## Pourquoi rien ne le rebâtit

La source du carnet, c'est la base elle-même. Un `put_note` avec les bonnes étiquettes
réécrit la colonne (la différence est vue), mais la bonne valeur n'existe que chez
l'utilisatrice ou dans une sauvegarde. `Snapshot` est immuable (`writes.immutable`) :
aucun outil ne le corrige, seule une restauration.

## Peut-on détecter une note suspecte ?

Pas de façon sûre. Ce qui ne marche pas, et les deux indices faibles :

- **Le hachage du contenu ne voit rien** : `_content_hash` ne couvre que `text` (le seul
  champ de contenu déclaré) ; un échange d'étiquettes ne le change pas.
- **Un instantané n'est pas relié à sa note** : `Snapshot` porte sa propre clé, son texte et
  ses étiquettes, sans lien vers la note dont il viendrait. Indice faible : un instantané
  dont le **texte est identique** à celui d'une note (même clé si l'usage en a été gardé) et
  dont les étiquettes diffèrent — mais un instantané pris avant une retouche légitime des
  étiquettes donnerait le même signal.
- **Indice faible, à l'échelle de la base** : l'échange se fait entre lignes d'une même
  région de la table ; deux notes voisines dont les jeux d'étiquettes semblent intervertis
  (les étiquettes de l'une collent au texte de l'autre) se voient à la relecture, pas par
  un calcul.
- **Le programme de contrôle du moteur** (`tools/check_rel_directions`) ne voit que les
  relations, pas les nœuds.

## Pour le fermer

Savoir si des carnets sont en usage réel (Lucie). S'il y en a : les restaurer depuis une
sauvegarde antérieure, ou faire relire leurs étiquettes. Sinon : fermer, le correctif
`25b3b45dc` empêchant les nouveaux cas.

## Décision (5 octobre 2026)

Lucie n'a aucun carnet en usage réel : son seul usage réel est le constructeur de decks
MTG, dont la base sera refaite. Aucune base de carnet à sauver ni à relire. Le correctif
`25b3b45dc` empêche les nouveaux échanges ; le ticket est fermé.
