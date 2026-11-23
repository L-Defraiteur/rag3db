# Les commandes de fond d'une tâche survivent à l'arrêt de l'hôte

- **État** : ouvert (23 novembre 2026, 06 h), rag3weaver archi ; pour le matin (c7).
- **Famille** : produit
- **Gravité** : un serveur lancé par un équipier reste vivant après l'arrêt ou la relance de l'hôte, sans plus personne pour l'arrêter.
- **Atteignable en service** : oui.
- **Touche rag3weaver** : oui (`commande.rs`, `lancer_en_fond` ; `daemon_host/teammate.rs`, `end_task_background`).
- **Pour** : archi.
- **Signalé par** : c7, banc et archi, la nuit du terminal.

## Ce que c'est

Une commande de fond meurt avec sa tâche (fermée, fusionnée, rejetée : `end_task_background`) depuis le 23 nov. (`background-lock`, le rejeu par la voie de l'appel). Mais elle survit à son hôte, par le choix du 11 octobre (« ses flux vont droit dans leurs fichiers »). À trancher : l'hôte arrête-t-il les fonds de ses tâches à son arrêt, ou les retrouve-t-il à sa relance (pid et journaux dans le dossier de la tâche), pour que l'IDE et le terminal les disent et les arrêtent.
