# L'IDE n'ouvre ni ne crée de projet, et sa page vide ne dit pas comment

- **État** : ouvert (21 novembre 2026, session recherche, scénario 2 de la nuit).
- **Gravité** : premier pas bloquant pour un nouveau venu qui ne lit que l'IDE.
- **Atteignable en service** : oui (un hôte qui ne sert encore aucun dépôt).
- **Touche rag3weaver** : oui (`ui/ide`, la page d'accueil ; les routes de `src/ide.rs`).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Le pas exact

Un hôte neuf, l'IDE ouvert par son URL d'entrée : la page dit « L'hôte ne sert encore aucun dépôt : un dépôt paraît
ici quand un agent ou l'atelier l'ouvre. » Il n'y a ni bouton, ni lien, ni commande. L'IDE n'a aucune route pour
ouvrir un dossier ou créer un projet (`/ide/*` : chat, tâches, accords, propositions, recherches…). Un développeur qui
ne lit que l'IDE s'arrête là.

## Le contournement

Dans un terminal, depuis le dossier du projet : `rag3weaver-backend atelier`. L'atelier paraît alors dans l'IDE, et
son chat crée les fichiers, le dépôt git (sans push) et attache le worktree.

## Pour le fermer

- **Petit** : la page vide donne la commande (`rag3weaver-backend atelier`, depuis le dossier du projet) et ce que
  l'atelier fera ensuite.
- **Plus grand** (la vision de Lucie : l'IDE, environnement de travail plus complet que le terminal) : ouvrir un
  dossier ou démarrer un projet depuis l'IDE, sans terminal.
