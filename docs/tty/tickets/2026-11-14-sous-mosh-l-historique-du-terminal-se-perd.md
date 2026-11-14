# Sous mosh, l'historique du terminal se perd

- **État** : ouvert. Constaté par la session tty le 14 nov. 2026 (lot tty-6), sans repli encore.
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

Aucun pour l'instant : le harnais (`extension/rag3weaver/tty/mesures/mesure.py`) lit les octets envoyés et ne sait pas défiler. Un témoin possible : une option de journal (ci-dessous) dont le fichier contient les lignes que l'écran a perdues.

## Ce qu'il faut pour le fermer

Un repli dit, sans rien exiger de l'utilisateur. Deux pistes, à trancher avant le code :
- une option `--journal <fichier>` (ou un journal par défaut dans le dossier de l'hôte) où la vue écrit chaque ligne validée, à relire par `less` ;
- une vue qui détecte qu'elle tourne sous mosh et le dit sur sa ligne d'état, en indiquant le journal.
