# L'IDE ne montre pas les dossiers du poste que les sessions tiennent

- **État** : ouvert (21 novembre 2026, session recherche, scénario 1 de la nuit).
- **Gravité** : manque de la consigne de Lucie (« savoir tous les dossiers sur le PC ouverts dans rag3weaver »)
- **Atteignable en service** : oui (hôte de service de la tablette, fed132c964 rebâti, posé à 01 h 55).
- **Touche rag3weaver** : oui (`ui/ide`, la table des dossiers, les vues).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Le pas exact

L'accueil de l'IDE de service liste deux dépôts (`rag3db`, l'atelier) ; `projet-tty` ne paraît que sous
« Disponibles » dans la barre du chat. Les worktrees des sessions (`~/git_workspaces/<session>/<jj-mm-aaaa>-<lot>`,
inscrits dans la table des dossiers) et les vues ouvertes n'y sont pas.

## Ce qui avance

Le lot `recherche-ide-ouvrir` (sur l'API d'arbre `arbre-ouvrir-un-dossier`) donne un champ « Ouvrir un dossier » à
l'accueil, et le dossier ouvert paraît parmi les dépôts. Il ne liste pas encore les dossiers des autres sessions.

## Pour le fermer

Une liste « Dossiers du poste » : chaque dossier de la table, sa session, ses worktrees et ses vues, ouverts ou non,
avec le lien vers son arbre quand l'hôte le sert.
