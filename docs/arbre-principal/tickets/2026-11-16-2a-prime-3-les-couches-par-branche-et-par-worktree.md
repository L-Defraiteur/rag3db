# 2a′-3 : écrire les couches par branche et par worktree

- **État** : ouvert (16 novembre 2026), arbre principal ; conception posée, code à écrire.
- **Famille** : produit
- **Gravité** : fonctionnalité attendue ; sans elle, seule la couche `backend` a des générations, et toute vue
  sur une branche ou un worktree se refuse (« couche sans génération »).
- **Atteignable en service** : oui, dès qu'un outil reçoit `view` sur une autre couche que `backend`.
- **Touche rag3weaver** : oui (`src/suivies.rs`, `src/code_sync.rs`, `src/catalog.rs`, `src/generations.rs`,
  la montre `src/watch.rs`, le manifeste `tracked.writes`).
- **Pour** : arbre principal ; à faire par Codex à partir de la passation du 16 novembre.
- **Signalé par** : arbre principal et c7, 16 novembre 2026.

## Ce que c'est

La conception est dans `docs/arbre-principal/16-11-2026/01-2a-prime-3-couches-par-branche-et-par-worktree.md`,
qui prolonge `docs/arbre-principal/14-11-2026/03-2a-prime-3-ce-que-les-suivies-apportent.md`. Elle demande :
1. une couche d'histoire par branche suivie, écrite par les objets git (`git ls-tree`, `git diff-tree`,
   `git cat-file`), sans checkout ;
2. une couche de travail par worktree : ses fichiers non commités, sa parente épinglée au rang du HEAD ;
3. un seul fil de montre, et le plafond `tracked.writes.max_work_layers` (32 par défaut), refus nommé au-delà ;
4. le sélecteur `worktree:<motif>` dans `src/suivies.rs` ;
5. la fin d'une couche quand son worktree disparaît ;
6. la publication de la couche du backend à chaque « indexation terminée » (le ticket
   `2026-11-16-une-vue-sans-rang-lit-l-etat-de-la-bascule-tant-que-rien-ne-publie.md`).

## Ce qu'il faut pour le fermer

Les sept témoins de la page, rouges d'abord, puis verts ; la passe épinglée avec `e2e_code_sync` (exigée par
`tools/poste/required_suites.txt`).
