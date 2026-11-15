# Embarquement — rapport de session du 14 novembre 2026 (et de la nuit du 15)

Session « embarquement » (Claude), sous l'orchestration de c7. Les règles de l'activation et de la
résidence des modèles sont dans [`../13-11-2026/01-activation-et-residence-livre-et-manquant.md`](../13-11-2026/01-activation-et-residence-livre-et-manquant.md).

## Ce qui est livré (sur master)

| Lot | Atterri | Ce qu'il fait |
|---|---|---|
| La démo Node.js dit son embarqueur | b7a026f16 | sans service, la démo retire toutes les variables de service ; chaque passe nomme son embarqueur (modèle et adresse au /sante) |
| L'épreuve « sans service » du paquet | 1b22409b3 | `test/essai.js` retire les variables de service ; ses assertions en exigences et rouges connues |
| Le vrai jeu de l'embarqueur à la demande par l'hôte de luciepc | 5286273ef (docs) | vert : activation au premier index, bail rendu à 30 s, éviction à 60 s, réactivation |
| Les nouveautés et le README de l'activation | 970176633 | les lignes « à la demande » et « activation distante », le README à jour |
| La reprise après plantage d'un fournisseur sous bail | 67dabc121 | relance unique, `Maintenir` compris ; refus dit en deux temps ; `relances` au /sante |
| Les activations nommées et le groupe à la demande | 441f28e08 | `activations: { <nom>: {…} }` cité par nom ; creux, relecteur et OCR à la demande, un processus, un bail par capacité |
| Le dense porté sur `Tenue<C>` | e14b32ad1 | un seul mécanisme pour les quatre capacités (`ALaDemande`, `Maintenu` retirés) |
| Le dual d'un creux à la demande | 92b7a09fa | le dual se déclare par `models.sparse.dimensions`, sur la même tenue ; une autre dimension que le dense est refusée au chargement |
| Le repli de l'aide des tests | 8cba8638e, f315a1563 | sans service qui sert le modèle, l'aide charge sur le périphérique **nommé** par `RAG3WEAVER_TESTS_REPLI_PERIPHERIQUE`, sinon rouge nommé ; `igpu:0` à luciepc (posé par G dans `passe_epinglee.sh`) |

**Les vrais jeux**, tous verts, par des hôtes isolés à luciepc (l'hôte de Lucie jamais appelé, 7878 et
7879 jamais touchés) :
- le dense à la demande par l'hôte (midi) ;
- le groupe bge-m3, relecteur et OCR (18 h 46) ;
- le dense sur `Tenue<C>` (21 h 39).

Les tableaux sont aux tickets et à la page des règles.

## Ce que la session a appris (les règles de la page et de la mémoire)

- **Un groupe bge-m3 à côté des résidents de card2** ne part qu'avec **13 Gio libres**, ou le 7879
  arrêté. Le groupe a tenu 11,4 Gio, et card2 est descendue à 2 Gio libres au pic du 14 novembre.
- **Le target d'une session gonfle** en une soirée de rejeux (34 Go le 14 au soir) : on fait le ménage
  avant une passe entière, et on ne relève jamais le plafond sans preuve.
- **Pas de docs hors file sur une page que mon lot en pile modifie** : sinon le dernier pas bute, et il
  faut reposer à la main puis passer par `PILE_TESTE` (deux fois cette nuit).
- **Dans `activation.rs`, tout item neuf prend `#[cfg(feature = "daemon")]`** ; jouer `check --tests
  --no-default-features` dès les témoins verts (deux passes perdues le 14).
- **Les cartes de luciepc** (relevé du 15, 00 h 2x) : seat1 n'existe plus, et les trois cartes sont sur
  seat0. card1 (Intel ARL, i915) ne porte aucun écran ; les écrans sont sur card0 et card2. Aucun poste
  n'a d'adaptateur Vulkan CPU (lavapipe), donc le `Cpu` de burn ne s'ouvre nulle part.

## Reprendre

- **Worktrees vivants de cette session** : aucun, aux deux postes. Tous effacés par
  `garde_worktree.py`, archives et rapports dans `~/.cache/rag3weaver-build/<lot>/`. Les journaux
  ignorés retirés des worktrees sont gardés dans `…/<lot>/reste-du-worktree/`.
- **Branches distantes** (gardées, rien ne s'efface sans Lucie) : `activations-nommees` et ses
  `-pile`, `-pile-2` ; `dense-sur-tenue` et ses `-pile`, `-pile-2` ; `dual-du-creux` et ses `-pile`,
  `-pile-2` ; `repli-de-l-aide`, `repli-de-l-aide-pile` ; `reprise-apres-plantage` et ses `-pile` ;
  `essai-sans-service-2`, `-3` ; `demo-dit-son-embarqueur`. Toutes ont atterri.
- **À luciepc** :
  - `~/.cache/rag3weaver-build/target-activation`, 14 Go, rebâti depuis l'effacement du 14 à 23 h 59 ;
  - les dossiers des vrais jeux : `vrai-jeu/`, `groupe-vrai-jeu/`, `dense-vrai-jeu/`, avec leurs
    scripts (`mesure.py`, `sonde.sh`, `hote.py`, `manifeste.py`), leurs binaires et leurs journaux.
- **La mesure en attente** : à **03 h 00 le 15 novembre**, la mesure à part des 904 s du premier index
  du groupe bge-m3 (décision de c7), quand card2 est sans autre charge, les services de Lucie intacts.
  - **Avant** : relire `poste --qui`, `rocm-smi --showuse` et la VRAM de card2 (13 Gio libres au moins).
  - **Le jeu** : rejouer `groupe-vrai-jeu/` avec une base et un dépôt neufs (hôte isolé 7971,
    fournisseur à part 7891).
  - **Ce qu'elle dit** : lent même carte calme, c'est le modèle ; rapide, c'était la carte pleine.
  - Un rappel est posé à 02 h 57 dans la session.
- **Tickets ouverts à moi** :
  - `2026-11-08-granite-278m-a-luciepc-s-est-arrete-huit-fois-cause-illisible`, qui attend la
    prochaine sortie de l'unité ;
  - `2026-11-12-gerer-activation-et-residence-des-modeles`, qui garde le préchauffage et les
    services de card2 déclarés par une activation (au mot de Lucie) ;
  - `2026-11-14-le-refus-est-mort-avant-de-repondre-nomme-le-type-de-service` ;
  - `2026-11-15-un-repli-cpu-des-tests-sans-vulkan`, un souhait ;
  - à faire selon c7 : la fixture `--dual` élargie au creux de granite.
- **Question à Lucie**, posée par c7 : lavapipe aux deux postes (sudo), pour un vrai `cpu` de repli.
