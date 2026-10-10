# Le démon d'un test unitaire « n'a pas répondu » sous la charge du poste

- **État** : ouvert
- **Gravité** : test instable (un rouge sans défaut du produit)
- **Atteignable en service** : non (le délai est celui du test, pas du démon)
- **Touche rag3weaver** : oui (`src/daemon/embeddings.rs`, tests)

## Ce que c'est

`daemon::embeddings::tests::le_demon_porte_aussi_le_relecteur_et_l_ocr` a rendu « le démon
n'a pas répondu » le 10 octobre au soir. Le test lance un démon dans un fil, sur un port
pris puis relâché par `port_libre()`, et lui laisse 200 × 10 ms pour répondre
(`fn demon`, `src/daemon/embeddings.rs` ~830). Deux batteries `lourd` tournaient en même
temps.

## Recette

Pas de recette sûre. Seul, le module passe : 5 passes sur 5, 18 tests en 0,27 s. Deux causes
possibles, non départagées :
- **le délai** : 2 s, sous la charge de deux batteries ;
- **la course au port** : `port_libre()` relâche le port avant que le démon ne le prenne, et
  un autre processus peut le prendre entre-temps.

## Témoin

Le test lui-même, sous charge. Il n'a pas de témoin à part.

## Pour le fermer

- Lier le démon à `127.0.0.1:0` et rendre l'adresse qu'il a prise : plus de course au port.
- Porter l'attente à un délai qui tienne sous charge (10 s), en gardant la sortie dès la
  première réponse.
