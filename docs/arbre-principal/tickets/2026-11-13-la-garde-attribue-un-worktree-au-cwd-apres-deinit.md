# La garde attribue un worktree au cwd après deinit

- **État** : ouvert ; retrait luciepc suspendu sans force, sources et administrations archivées.
- **Famille** : produit
- **Gravité** : bloque le retrait sûr d'un lot livré et conserve son target24,4Go.
- **Atteignable en service** : non, procédure de nettoyage des worktrees.
- **Touche rag3weaver** : oui, scripts/garde_worktree.py.
- **Pour** : arbre-principal pour le garde ; embarquement pour le retrait de son lot.
- **Signalé par** : embarquement, retrait après livraison23ceed87b/7308544ce.
- **Ouvert le** : 13 novembre 2026

## Constat et preuve

Le sous-module codeparsers propre avait un git-common-dir privé sous
.git/worktrees/cx-activation-modeles/modules/extension/rag3weaver/codeparsers,
un seul worktree, aucun ignoré/local, zéro référence active W/target.
Administration entière archivée fidèlement AVANT et APRÈS git submodule deinit
--all sans force. Deinit supprime core.worktree ; git --git-dir ADMIN worktree
list rapporte alors le cwd du processus, ici le dossier d'archives externe.
Le garde le compte comme autre worktree et rend ROUGE. Aucune suppression du
worktree/target ni aucun contournement du garde n'a suivi.

Preuve PC : ~/.cache/embarquement-notes/activation/archives-fin-luciepc/garde.json.
Refus exact : « administration sert un autre worktree :
/home/lucied/.cache/embarquement-notes/activation/archives-fin-luciepc ».
Les archives avant-deinit, finale, données générées et references.json sont
conservées ; le portable a été retiré avec garde VERT et reçu indépendant.
WPC /home/lucied/git_workspaces/cx-activation-modeles, target-spans24407724032.
Ce dossier d'archives n'est pas un worktree véritable du sous-module : son
attribution change selon le cwd implicite de Git après désinitialisation.

## Suite et critères de fin

Arbre/coordination : déterminer le traitement explicite d'une administration
résiduelle désinitialisée, sans inférer de source depuis le cwd, masquer un
partage ni changer core.worktree pour faire passer la garde. Préserver les
archives et le rouge ; preuve discriminante initialisée/désinitialisée/partagée,
rapport frais hors cible et VERT avant retrait. Embarquement reprend alors son
retrait quiescent sans force. Ne pas effacer/pruner une administration à la main.
