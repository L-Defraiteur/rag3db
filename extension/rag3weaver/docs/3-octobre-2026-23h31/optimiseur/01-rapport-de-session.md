# Optimiseur — rapport de session

**Mis à jour le 3 octobre 2026 à 23 h 45.** Session à l'arrêt, aucun calcul
en cours. Sujet : le moteur d'embarquement burn et nos forks, les correctifs
à proposer à tracel-ai, et, depuis le soir du 3 octobre, les modèles de
décision pour la mémoire longue. Le rapport précédent est dans
`../../2-octobre-2026-01h07/optimiseur/`.

## Ce qui attend quelqu'un

| quoi | qui | où lire |
|---|---|---|
| **L'envoi à tracel-ai** : cinq envois prêts et vérifiés, rien n'est parti. Il faut son « envoie » dans la fenêtre de cette session, avec le rythme (d'un coup, ou étalé dans la journée) et la ligne de déclaration | Lucie | `../../optimiseur/3-octobre-2026-14h13/02-les-textes-reecrits.md` |
| Lire les fiches par correctif avant l'envoi : c'est elle qui répondra en revue, et leur règle écrite demande que l'autrice défende chaque changement | Lucie | fin du même document |
| La demande au support GitHub pour retirer les neuf anciens commits des forks, toujours servis par hash | Lucie | `.vault/demande-support-github-forks.md` (hors git) |
| Une suite pour les modèles de décision, s'il y en a une : un jeu thématique tiré de vrais sujets de la base, rangé par quelqu'un d'autre | Lucie, par l'orchestration | `../../optimiseur/3-octobre-2026-20h55/00-ou-en-est-la-question.md` |

## Ce qui a été fait le 3 octobre

### Les correctifs pour tracel-ai

1. **Les vérifications finies.** Chaque correctif a son test, rouge sans lui
   et vert avec. Sur la branche burn : `cargo run-checks` code 0 et clippy
   propre, avec Rust 1.99.0. La suite de tests de burn sur Vulkan rend les
   mêmes compteurs avant et après (1638 réussis, 5 échecs qui sont à eux sur
   cet iGPU), 1639 avec notre branche.
2. **L'étude du ton** (`01-comment-on-ecrit-chez-tracel-ai.md`, `1e1a255c2`) :
   burn autorise par écrit le code écrit avec une IA, sans exiger de
   déclaration, et exige que l'autrice comprenne et défende chaque ligne.
3. **Les textes réécrits** (`02-les-textes-reecrits.md`, `43a3cdc33`), cinq
   envois : une PR cubecl, deux PR cubek, une issue et une PR burn.

Décidé, et pourquoi :

| décision | par qui | pourquoi |
|---|---|---|
| trois dépôts d'un coup, avec une déclaration d'aide IA dans chaque envoi | Lucie, relayée par l'orchestration | leur règle l'accueille bien ; la ligne ne doit dire que ce qui est vrai le jour de l'envoi |
| les anciennes PR 2 et 4 deviennent un seul correctif | la mesure | sur leur `main`, le défaut de la 2 n'est atteignable qu'une fois la 4 appliquée, et la 4 seule fait paniquer `from_data` |
| la flash attention (ancienne PR 5) ne part pas en PR, elle devient une question dans l'issue burn | cette session, validé par l'orchestration | elle ne se teste qu'avec deux autres correctifs en place, et un mainteneur a refusé un repli du même genre |
| la PR cubecl ouvre par « is it left out on purpose? » | la mesure | burn affirme dans un test que Flex32 n'est pas supporté sur Vulkan |
| je recommande d'étaler les envois (cubecl, puis burn, puis cubek) | cette session | cinq envois d'un compte neuf le même jour ressemblent à une rafale ; à Lucie de trancher |

### Les modèles de décision

Six documents dans `../../optimiseur/3-octobre-2026-20h55/`, de la
vérification à la source à la synthèse. Le résultat : ranger un texte parmi
des sujets existants est un problème d'ordre que le cosinus de granite
résout aussi bien que le modèle de décision ; décider qu'il faut créer un
sujet ne marche par aucune voie essayée. Le détail est dans le relevé de
connaissances.

Décidé : rien pour le produit. Lucie a demandé une exploration (« faut pas
courir avec avant d'avoir trouvé si une bonne manière de s'en servir
existe ») ; l'exploration est rendue, sans recommandation.

## Trois fautes de cette session, réparées

- **Un commit sur la branche d'une autre session.** L'arbre principal était
  sur `synchronisation-par-perimetre` et non sur `master`. Le commit a été
  reposé sur `master` et retiré de là. Depuis : tout commit de document passe
  par un worktree détaché de `origin/master`.
- **`/tmp` rempli.** C'est un tmpfs de 61 Gio en mémoire vive ; deux dossiers
  de compilation l'ont rempli vers 14 h 45 et la session de l'arbre principal
  s'est arrêtée. Depuis : toute compilation sur disque, sous `~/.cache`.
- **Deux affirmations sur six cas.** « Le seuil tient » et « GLiNER range
  sans faute » ont été écrits sur six cas chacun, puis démentis sur 48 et sur
  16. Le document 03 porte un renvoi vers sa correction.

## Comment reprendre

### L'envoi à tracel-ai, au mot de Lucie

1. **Reconstituer les clones**, s'ils ont disparu (le scratchpad est en
   mémoire vive et ne survit pas à un redémarrage). Les branches vivent dans
   `.vault/forks/pr-{burn,cubek,cubecl}.bundle`, vérifiés le 3 octobre. La
   recette : cloner `tracel-ai/<dépôt>` sans blobs, **poser l'identité en
   local** (`Lucie Defraiteur`, adresse personnelle) avant tout, puis
   `git fetch <bundle> "+refs/heads/*:refs/heads/*"`.
2. **Contrôler les adresses** sur chaque branche :
   `git log --format='%ae %ce' <base>..<branche>` ne doit rendre que
   l'adresse personnelle. Un commit poussé sur un fork est visible dans tout
   le réseau du dépôt.
3. **Vérifier que leur `main` n'a pas touché nos fichiers** depuis le 3
   octobre (`git log <base>..origin/HEAD -- <fichiers>`), et que la fusion
   reste propre.
4. **Pousser les branches sur ses forks** `L-Defraiteur/{cubecl,cubek,burn}`,
   sans force.
5. **Créer l'issue et les PR** par un script en fichier, le compte personnel
   passé par variable (`GH_TOKEN=$(gh auth token --user L-Defraiteur)`), sans
   changer le compte actif de `gh`. Les corps viennent de fichiers écrits par
   l'outil d'écriture : jamais de heredoc non protégé.
6. Remplir dans les textes les renvois croisés (numéros d'issue et de PR) au
   fur et à mesure.

| envoi | dépôt | branche | tête | base |
|---|---|---|---|---|
| A | cubecl | `vulkan-register-flex32` | `d27f6ed0` | `a6321cd4` |
| B | cubek | `attention-mask-row-stride` | `65bedd3c` | `fbed329f` |
| C | cubek | `matmul-flex32-accumulates-in-f32` | `e20293ce` | `fbed329f` |
| E | burn | `flex32-fixes` (quatre commits) | `ca25dd87` | `8942d406` |

Les patches tels qu'ils partiraient sont aussi dans
`../../optimiseur/3-octobre-2026-14h13/patches/`, rejouables par `git am`.

### Pièges

- Leur CI bâtit en `stable`, qui est Rust 1.99.0 ; la toolchain par défaut du
  poste est 1.98.1 et ne connaît pas un lint qu'ils ont ajouté. Utiliser
  `cargo +1.99.0`, installée à côté.
- `cargo run-checks` lance cargo sans limite de tâches : poser
  `CARGO_BUILD_JOBS=8`.
- burn épingle cubecl à `36314551`, deux commits avant la base de notre
  branche cubecl : pour valider contre burn, reposer notre commit sur cette
  révision.
- Sous fish, tout script va dans un fichier bash.

### Les modèles de décision

Rien à reprendre sans nouvelle demande. Pour rejouer : voir le relevé de
connaissances, section « rejouer une mesure ».
