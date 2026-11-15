# `passe_epinglee.sh` emboîte en silence un chemin absolu de journaux

- **État** : ouvert (15 novembre 2026, hotfixs)
- **Famille** : produit
- **Gravité** : une passe qui rend son verdict là où personne ne le lit. Le lot paraît mort et sa session cherche une
  cause qui n'existe pas : une soirée perdue le 15 nov., et un faux ticket, fermé depuis
  (`docs/tickets/closed/2026-11-15-une-passe-en-file-sous-poste-lourd-disparait-sans-trace-a-luciepc.md`).
- **Atteignable en service** : non (outillage des passes)
- **Touche rag3weaver** : non (`passe_epinglee.sh`, recopiée dans `~/.cache/rag3weaver-build/`)
- **Pour** : G

## Ce qu'on voit

`passe_epinglee.sh <worktree> <dossier des journaux> <filtre>` fait `J=$HOME/.cache/rag3weaver-build/$2`. Le deuxième
argument est donc un nom relatif. Si on lui passe un chemin absolu
(`/home/lucied/.cache/rag3weaver-build/journaux-x`), il crée sans rien dire
`~/.cache/rag3weaver-build/home/lucied/.cache/rag3weaver-build/journaux-x/` et y écrit `resume.log`. Le dossier attendu,
lui, reste vide. Rien ne le dit, ni sur la sortie ni dans le journal de lancement.

## Pour fermer

1. Un chemin absolu en `$2` est soit refusé (code non nul, une ligne qui le dit), soit pris tel quel.
2. Dès son premier pas, la passe dit sur sa sortie où elle écrit (« journaux : <chemin> »).
3. Un témoin de script : un chemin absolu ne crée aucun dossier emboîté.
