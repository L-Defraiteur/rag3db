# Le terminal n'est pas installé avec l'hôte

- **État** : ouvert (20 novembre 2026, écrit par G à la demande de c7, sur le constat de tty)
- **Pour** : G (optimiseur), avec tty
- **Famille** : produit (l'installation de rag3weaver)
- **Gravité** : moyenne. Sans le bâtir à la main, personne ne peut lancer le terminal. Lucie l'essaie le 21 novembre.
- **Atteignable en service** : oui, à l'installation
- **Touche rag3weaver** : oui (`scripts/installer-l-unite-de-l-hote.sh`, et le bâti de l'hôte)

## Ce que c'est

`rag3weaver-tty` n'est installé sur aucun des deux postes, ni dans le PATH ni dans `~/.local/bin`, où est l'hôte.
L'installation de l'hôte (`installer-l-unite-de-l-hote.sh`, puis la pose d'un binaire neuf) ne le bâtit ni ne le pose.
Or la règle de Lucie est : « les gens n'installeront rien en plus de rag3weaver ».

## Ce qui est fait le 20 novembre, hors du dépôt

Les scripts de pose de G (son bloc-notes, `bati-lucie-luciepc.sh` au cache de luciepc) bâtissent maintenant
`rag3weaver-tty` de la même tête que l'hôte (`tty/`, `cargo build --release --bin rag3weaver-tty`, sans la lib), et le
posent avec lui dans `~/.local/bin`, aux deux postes, en gardant l'ancien à côté. Cela vaut pour l'hôte neuf du
20 novembre au soir et pour celui du 21 au matin.

## Le correctif à faire dans le produit

L'installeur de l'hôte installe le terminal avec lui, de la même tête, au même endroit que `rag3weaver-backend`. Un
script de pose qui change l'hôte change le terminal dans le même geste. Témoin rouge d'abord, dans
`test_installer_filet.sh` ou un témoin voisin : après l'installation, `rag3weaver-tty --version` (ou son équivalent)
répond depuis le dossier de l'hôte, et sa tête est celle de l'hôte.
