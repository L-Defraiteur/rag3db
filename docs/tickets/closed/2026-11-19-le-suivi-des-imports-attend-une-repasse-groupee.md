# Le suivi des imports sans expression régulière attend une repasse groupée

- **État** : corrigé (19 novembre 2026) — parti avec le lot du cache des parents (branche rag3db `codeparsers/parent-cache-3`, pointeur codeparsers 828f855), pour une seule repasse
  change de toute façon la version de l'analyse
- **Famille** : produit
- **Gravité** : lenteur légère : environ 10 % de l'analyse de rag3weaver/src (4,94 → 4,45 s, une mesure par état à
  luciepc ; le moteur dans le bruit)
- **Atteignable en service** : oui, à chaque analyse de code (index et montre)
- **Touche rag3weaver** : oui, par le pointeur codeparsers (`ensure_import_references_tracked`)
- **Pour** : session codeparsers
- **Signalé par** : session codeparsers, profil par échantillonnage de l'analyse (lot du temps de l'image, B)

## Le lot

Branche rag3db `codeparsers/whole-word-imports`, tête 10672060147c1f5c2e69b2a144dd33b3897d241c ; pointeur codeparsers
`whole-word-imports` f164cb6 (correctif dcec0a5 : `find_whole_word`, la même frontière que `\b`, au lieu d'une
expression régulière compilée par scope et par import externe). Témoin `tests/import_tracking_cost.rs` (codeparsers) :
rouge 11,97 s pour 300 imports sur 300 scopes, vert 0,54 s. Sonde de marque : arêtes identiques au couple près sur
rag3weaver/src et `src/` du moteur. Passe complète VERT sur la lib 7c2b506ec (journaux `passe-mot-2` à luciepc).

## Pourquoi il attend

Le pointeur change la version de l'analyse (`parseur=<commit>`), donc **toutes les bases repassent une fois**. Pour
0,5 s d'analyse et des arêtes identiques, une repasse seule ne se justifie pas (c7, 19 novembre). Le dépôt au pousseur
a été retiré.

## Ce qu'il faudrait

Reposer cette branche avec le **prochain lot qui change la version de l'analyse** (une grammaire, une règle, la
résolution si elle la touche), en pile, pour une seule repasse ; rejouer alors la passe et déposer les deux ensemble.
Fermer ce ticket à l'atterrissage.

## Fermeture, 19 novembre 2026

Le suivi des imports part en pile sous le lot du cache des parents, qui change de toute façon la version de
l'analyse : une seule repasse pour les deux. Le chiffre de l'ensemble, l'analyse en release avant/après (deux tours
entrelacés à luciepc, arbre figé) : rag3weaver/src 4,28 et 3,88 s → 2,72 et 2,15 s ; `src/` du moteur 1,48 et
1,51 s → 0,89 et 0,88 s ; `tools/` 0,22 et 0,28 s → 0,14 et 0,16 s. Sonde de marque identique au couple près sur
rag3weaver/src, le moteur, `tools/python_api` et l'UI de rag3weaver.

