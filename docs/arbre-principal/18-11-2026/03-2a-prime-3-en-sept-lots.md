# 2a′-3 en sept lots

18 novembre 2026, arbre principal. **À relire par Lucie, pour la forme** : sept lots au lieu d'un, chacun avec son
témoin rouge d'abord. La conception reste celle de la page du 16 novembre
(`docs/arbre-principal/16-11-2026/01-2a-prime-3-couches-par-branche-et-par-worktree.md`). Cette page en fixe
l'ordre, acceptée par c7 le 18 novembre.

## Ce que le code a aujourd'hui

Relevé sur master e475942260 :

- **Une seule couche.** Le catalogue tient une seule `Visibilite` (`Catalog::poser_les_generations`,
  `src/catalog.rs`). L'écrivain (`write_batch_by_generations`, `src/code_sync.rs`) écrit toujours dans
  `generations.couche`, c'est-à-dire `backend`. Aucune génération n'est écrite pour une autre couche.
- **Aucune publication dans le produit.** `publier_la_generation` n'a que des appelants de test, et n'écrit jamais
  de parente.
- **Les couches ne sont pas tenues au lancement.** Les suivies connaissent les couches et leurs ids stables
  (`suivies::identifiants_des_couches`), mais seule `ouvrir_une_tache` les calcule ; le backend ne les tient pas.
- **Une montre surveille une seule racine**, et chaque worktree attaché est son propre backend
  (`src/daemon_host/worktrees.rs`).
- **Aucune lecture d'objets git.** Ni `ls-tree`, ni `diff-tree`, ni `cat-file` dans `src/`.

## Les lots

Chaque lot porte son témoin rouge, joué rouge avant le correctif. La page du 16 novembre posait les sept témoins
ensemble en tête, mais un rouge qui attend trois lots ne juge rien.

1. **L'écrivain multicouche.**
   - Une `Visibilite` par couche : couche → rang en attente et rang publié.
   - L'écrivain et `publier_la_generation` prennent la couche et sa parente épinglée.
   - Témoin : deux couches écrites dans un même catalogue, chaque vue voit les siennes.
   - Suites exigées : `e2e_code_sync` et les suites de l'écrivain.
2. **La publication dans le produit.** Une réindexation finie publie sa génération (le ticket
   `2026-11-16-une-vue-sans-rang-lit-l-etat-de-la-bascule-tant-que-rien-ne-publie.md`). Sans ce lot, rien de ce qui
   suit ne se voit.
3. **Les couches d'histoire, sans checkout**, par `git ls-tree`, `git diff-tree` et `git cat-file`.
   - Témoin 2 : aucune écriture dans aucun worktree.
   - Témoin 3 : la parente épinglée au HEAD.
4. **Les couches de travail**, par `git status --porcelain=v2` et une parente épinglée au rang du HEAD.
   - Témoin 1, celui de c7 : une vue sur un autre worktree voit le fichier non commité de ce worktree.
   - **C'est ce lot qui rend l'état par vue visible à Lucie.**
5. **La montre multiracine.**
   - Un seul fil, qui note avec l'id de la couche de son worktree (`Lag::note_in`).
   - Le plafond `tracked.writes.max_work_layers`. Témoin 4.
   - Les alias à deux arguments de `Lag` meurent ici.
6. **Le sélecteur `worktree:<motif>` et la fin d'une couche.** Témoins 5 et 6. Le témoin 7, le doublon au bord haut
   d'un trou, compté dans l'état, suit le lot qui l'expose.
7. **Le sucre de nom d'une vue**, avec recherche, dans un lot à part.

## Les deux décisions de c7 (18 novembre)

- **Les couches d'histoire avant celles de travail (3 puis 4).** Poser 4 d'abord, sur la couche du backend,
  montrerait quelque chose plus tôt. Mais une vue de worktree dont le HEAD diffère du commit indexé se refuserait
  jusqu'au lot 3, ce qui serait pire qu'attendre un lot.
- **Les backends par worktree attaché restent jusqu'au lot 5**, puis un ticket les retire.
