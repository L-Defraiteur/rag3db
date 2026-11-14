# Sous mosh, l'historique du terminal se perd

- **État** : corrigé le 14 nov. 2026 (lot tty-7, session tty) : un journal de séance, ouvert de lui-même sous mosh, et la commande qui le relit dite dans la zone vivante. Constaté au lot tty-6.
- **Famille** : produit
- **Gravité** : perte de lecture. La séance continue, mais les lignes sorties de l'écran ne se relisent plus.
- **Atteignable en service** : oui, pour qui suit l'équipier par mosh.
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`).

## Ce que c'est

La vue inline écrit l'historique (le texte de l'agent, les outils, la boîte) dans le défilement normal du terminal, par `insert_before`, et ne redessine que trois lignes en bas. mosh ne synchronise que l'écran : il n'a pas d'historique de défilement. Sous mosh, ce qui sort de l'écran est donc perdu pour la lecture.

## La recette

```bash
mosh <hôte> -- rag3weaver-tty --depot <dépôt> --tache <nom> --demande "…"
```

Quand plus de lignes que l'écran n'en tient ont été écrites, le défilement arrière du terminal ne les rend pas.

## Le témoin

Le harnais (`extension/rag3weaver/tty/mesures/mesure.py`, séances 1b et 1c) lance le terminal avec `--journal`, puis sans, et lit le journal sur le poste distant : il doit porter les 200 mots du pas scripté et son bloc de code. Sous mosh, l'écran doit dire `less <journal>`, et un seul journal neuf doit s'être ouvert de lui-même. Rouge contre le binaire de tty-6, vert contre celui de tty-7 : `docs/tty/14-11-2026/02-le-journal-de-seance.md`.

## Ce qui l'a fermé

Un repli dit, sans rien exiger de l'utilisateur. Deux pistes, à trancher avant le code :
- une option `--journal <fichier>` (ou un journal par défaut dans le dossier de l'hôte) où la vue écrit chaque ligne validée, à relire par `less` ;
- une vue qui détecte qu'elle tourne sous mosh et le dit sur sa ligne d'état, en indiquant le journal.

Les deux pistes sont prises ensemble (c7, 14 nov.) : `--journal <fichier>` marche partout ; sous mosh, détecté par un ancêtre `mosh-server`, un journal s'ouvre de lui-même dans `~/.local/state/rag3weaver/tty/`, et la ligne du haut de la zone vivante dit `less <journal>` quand aucun texte n'y passe. Sous tmux lancé dans mosh, l'ancêtre est le serveur tmux : rien n'est détecté, mais tmux garde son propre historique.
