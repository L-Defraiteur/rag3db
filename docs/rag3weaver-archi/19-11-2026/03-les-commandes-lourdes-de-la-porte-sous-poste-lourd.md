# Les commandes lourdes de la porte sous `poste lourd` — conception du jeudi 19 novembre 2026

La demande vient de c7, sur la campagne D6. La porte des commandes (`src/commande.rs`) laisse un agent lancer
`cargo test`, `cargo build`, `cargo clippy`, `cmake --build` ou `ninja`. Elle le fait aujourd'hui à pleine
charge, hors du verrou du poste : sans deux cœurs libres, sans plafond de mémoire, sur l'écran de Lucie. Toute
commande lourde passe désormais sous `poste lourd`, avec son enveloppe, et une place refusée se dit à l'agent.

## Ce qui existe

- **`scripts/poste lourd <commande>`** prend une place lourde. Il lance la commande dans sa portée systemd, avec
  `POSTE_MEM_MAX`, en `nice` et `ionice`, et avec `CARGO_BUILD_JOBS`, `RAYON_NUM_THREADS` et
  `RUST_TEST_THREADS` ramenés à sa part des cœurs. Avec `POSTE_LOURD_ATTENTE_MAX=<s>`, il sort en **code 75**
  sans rien prendre quand la place ne vient pas.
- **La porte lance par argv**, dans son propre groupe. Le **bac à sable Landlock se pose entre fork et exec**,
  dans le fils.

## La difficulté

Un `poste` qui envelopperait la commande serait enfermé avec elle dans le bac à sable. Or il doit écrire ses
verrous (`~/.cache/rag3weaver-build/…`) et parler à systemd (`systemd-run --user`) : il ne le peut pas sous
Landlock. Il faut donc que **`poste` tourne libre et que la commande seule soit enfermée**.

## Deux formes

- **(B) recommandée : un lanceur qui enferme puis exécute.** Un sous-mode du binaire de l'hôte (par exemple
  `rag3weaver-backend sandbox-exec <règles JSON> -- <programme> <args…>`) pose les règles Landlock reçues, puis
  fait `exec` de la commande.
  - La porte lance `poste lourd <ce lanceur> <règles> -- <commande>`.
  - `poste` reste libre, la commande est enfermée, et toute l'enveloppe du poste s'applique (portée, plafond
    de mémoire, nice, cœurs).
  - Le coût : sérialiser le bac à sable, et un sous-mode de plus au binaire.
- **(C) une place tenue par l'hôte.** L'hôte lance `poste lourd sh -c 'echo tient; exec cat'`, attend
  « tient », puis lance la commande enfermée comme aujourd'hui, et ferme le porteur à la fin.
  - C'est plus simple, mais la commande n'est pas dans la portée du poste : pas de plafond de mémoire
    systemd. Elle n'aurait que les cœurs (lus dans l'environnement du porteur) et le nice.
  - Elle ne tient donc pas « mémoire bornée ».

## Le reste, commun aux deux

- **Quelles commandes** : les familles lourdes, une liste en code, celle de la porte (`cargo build|test|check|clippy|bench`,
  `cmake`, `ninja`, `make`). Les lectures (`git status`, `ls`) ne changent pas.
- **Où est `poste`** : un réglage de l'hôte, `RAG3WEAVER_POSTE` (le chemin du script), posé dans l'unité de
  l'hôte. On ne touche à aucun manifeste de Lucie.
  - Absent : rien ne change, ce qui laisse un hôte ailleurs que sur ces postes sans dépendance.
  - Posé et le script absent : la commande lourde est refusée en le disant, jamais lancée à pleine charge.
- **Attendre ou refuser** : `RAG3WEAVER_POSTE_ATTENTE_S` (60 par défaut) devient `POSTE_LOURD_ATTENTE_MAX`. Le
  code 75 rend à l'agent « the machine is busy: N heavy jobs hold it, M wait; the command did not run, try
  again later or do something else » (lu par `poste --qui`). C'est un retour d'outil en échec : l'agent le lit.
- **La mémoire** : `POSTE_MEM_MAX` vaut 32G par défaut, comme les passes.

## La preuve, rouge d'abord

Un faux `poste`, un script du témoin qui écrit son argv puis exécute la suite, est posé dans
`RAG3WEAVER_POSTE`. Une commande `cargo` de la porte passe par lui avec `lourd`. Un faux `poste` qui sort en 75
rend le message « the machine is busy » et ne lance rien. Aujourd'hui, la commande part directement : rouge.
Puis un témoin avec le bac à sable posé : la commande enfermée (forme B) ne peut pas écrire hors de son
worktree, et `poste` écrit ses verrous.

## Taille

- (B) : 1,5 jour.
- (C) : une demi-journée.
