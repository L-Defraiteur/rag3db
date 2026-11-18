# La tolérance de score dans le comparateur de parité

- **État** : fermé (18 novembre 2026, banc) — sur master en 5da078924a : `--score-tolerance` dans le comparateur et
  `scripts/vector_parity.py`, leurs témoins vus rouges et joués par `check_features`. Sur les vrais relevés : neuf
  contre élagué ROUGE sans option, VERT à 1e-5 ; les vecteurs de A contre B′ VERT (cosinus minimal 0,999999908).
  La preuve de parité des services reste à embarquement, qui reprend le script.
- **Famille** : produit
- **Gravité** : faible. Aujourd'hui, une parité entre deux bases embarquées à des moments
  différents dit ROUGE pour des scores qui ne diffèrent qu'à la cinquième décimale.
- **Atteignable en service** : non, c'est un outil de mesure.
- **Touche rag3weaver** : oui, `scripts/parite_search_code.py` et un script neuf.
- **Pour** : une session Codex (passation du 16 novembre), avec embarquement pour la preuve des
  services.
- **Signalé par** : banc et embarquement, 16 novembre.
- **Ouvert le** : 16 novembre 2026

## Le constat

Page `docs/banc/16-11-2026/01-la-parite-de-la-migration.md`, § 4 :
- un même texte, embarqué dans des lots différents sur GPU, rend des vecteurs qui diffèrent d'au plus
  5,86e-5, pour un cosinus d'au moins 0,99999991 ;
- seul contre seul, ou dans le même lot deux fois, les vecteurs sont identiques au bit près (sonde
  d'embarquement sur le 7878) ;
- le régime `reponses` exige l'égalité au bit près des scores : il dit ROUGE sur la paire neuf/élagué
  du § 10, alors que les 30 classements sont identiques.

## Ce qu'il faut faire

1. Dans `parite_search_code.py`, une option `--score-tolerance <float>` (0 par défaut, donc le
   comportement d'aujourd'hui). Elle vaut pour les régimes `optimisation` et `reponses` : deux
   scores sont égaux si leur écart absolu est sous la tolérance. L'ordre reste exigé au rang près.
   La dernière ligne reste VERT ou ROUGE.
2. Un script neuf, `extension/rag3weaver/scripts/vector_parity.py`, repris de
   `~/.cache/rag3weaver-build/vec-cmp.py` (luciepc). Il prend deux relevés Cypher de morceaux
   (`_text_hash`, vecteur) et rend :
   - le nombre de morceaux communs ;
   - le nombre de vecteurs identiques au bit près ;
   - le cosinus minimal et l'écart absolu maximal ;
   - VERT si cosinus ≥ `--min-cosine` (0,99999 par défaut) et écart ≤ `--max-abs` (1e-4 par
     défaut).
   La requête qui produit ses entrées est `~/.cache/rag3weaver-build/vec.cypher`.
3. Avec embarquement, reprendre cette tolérance dans sa preuve de parité des services (embarquement
   l'a annoncé pour le 17 novembre). Le banc fournit le script ; embarquement décide où il l'appelle.

## La preuve

- Les témoins d'abord, rouges : dans `scripts/test_parite_search_code.py`, une paire de relevés au
  même ordre dont les scores diffèrent de 1e-6. ROUGE sans option, VERT avec
  `--score-tolerance 1e-5`, ROUGE si l'ordre change. Dans `scripts/test_vector_parity.py`, trois
  vecteurs à la main.
- Rejouer sur les relevés réels :
  `banc-parite/sorties/preuve-bm25-master/neuf.json` contre `elague.json` doit passer à VERT avec
  `--score-tolerance 1e-5`, et rester ROUGE sans option.

## Taille

Deux à trois heures.
