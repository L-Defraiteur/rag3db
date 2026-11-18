# La dette de 469 morceaux après 1b est normale

- **État** : fermé (18 novembre 2026) : ce n'est pas un défaut. Le constat de Codex du 16 novembre est requalifié par
  arbre principal.
- **Famille** : produit
- **Gravité** : aucune ; dette d'embarquement attendue, payée par le repassage.
- **Atteignable en service** : oui, à toute migration 1b d'une base dont le découpage a changé depuis son indexation.
- **Touche rag3weaver** : non.
- **Pour** : arbre principal (consigné pour la bascule).
- **Signalé par** : la session Codex d'arbre principal, le 16 novembre (branche
  `cx-arbre-bascule-bases-20261116-r1`, ticket « la migration de scopes laisse 469 morceaux sans vecteur transféré »).

## Ce que c'est

Sur une copie de code-principal, `migration versions --base` a rendu 65 913 occurrences, 61 223 versions et
97 247 morceaux : 96 778 vecteurs repris, 469 morceaux en dette (`morceaux_en_dette: granite_278m`). La matrice de
Codex montre qu'aucun de ces 469 n'a de morceau ancien au même texte : ce sont des découpages neufs. 1b ne reprend un
vecteur qu'à texte identique, et laisse le reste en dette, comme prévu (`src/migration_des_versions.rs`). Le
rattrapage, ou le repassage de l'écrivain, les embarque : sur la copie, Codex les a vus tous couverts (« v9 »,
97 247 vecteurs, dette 0).

C'est le même phénomène que l'écart de découpage relevé par le banc le 16 novembre (environ 9 % des morceaux d'une
base ont un découpage neuf), vu ici avant le repassage.

## Pour la bascule

La fenêtre fermée n'a rien à faire de cette dette : le repassage, base ouverte, la paie. Le juge d'après repassage
exige une dette nulle.
