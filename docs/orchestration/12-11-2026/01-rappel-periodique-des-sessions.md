# Rappel périodique de coordination des sessions

À la demande de Lucie, installation locale le 12 novembre 2026 sur lucie-tablette.
Ce dispositif concerne les sessions Codex existantes, pas le runtime de rag3weaver.

## Comportement

Toutes les 90 minutes, un timer systemd utilisateur appelle la commande installée
`codex queue --thread 01a1cb5c-62b9-7ea0-8b7e-cf38398ece09 --message … --resume`.
Le rappel revient dans la session d’orchestration avec son contexte. Il lui demande de
vérifier les sessions et leurs tickets, puis de reprendre les travaux déjà autorisés
quand une dépendance est levée. Veille justifiée et lot terminé restent des états normaux.
Pas de nouveau chantier pour occuper une session, ni de répétition aveugle d’un test rouge.
Les règles de tickets, de permissions et de ressources du dépôt restent applicables.

Le CLI expose explicitement `--resume` comme une tentative de reprise : un échec peut
produire un avertissement même avec un code de sortie nul. Un message mis en file n’est
pas la preuve que le tour d’agent a déjà été exécuté. Le journal du premier test confirme
la mise en file pendant que la session est active, sans avertissement visible ; le premier
passage périodique reste à observer.

## Installation et contrôle

Fichiers locaux :

- `~/.config/systemd/user/rag3db-orchestration-watch.service` ;
- `~/.config/systemd/user/rag3db-orchestration-watch.timer`.

Le service oneshot utilise des chemins absolus vers Node et le CLI installés sous
`~/.local/share/fnm/node-versions/v22.23.3/installation/`, sans dépendre du PATH d’un terminal.
Le timer est activé dans timers.target ; OnActiveSec et OnUnitActiveSec valent 90 minutes,
AccuracySec vaut une seconde. Délai maximal de la commande : 45 secondes.
Aucun changement des permissions de la session et aucun privilège système supplémentaire.

```bash
systemctl --user list-timers rag3db-orchestration-watch.timer --all
journalctl _SYSTEMD_USER_UNIT=rag3db-orchestration-watch.service -n 30 --no-pager
```

Arrêter les prochains rappels :

```bash
systemctl --user disable --now rag3db-orchestration-watch.timer
```

Réactiver :

```bash
systemctl --user enable --now rag3db-orchestration-watch.timer
```

La machine, le gestionnaire systemd utilisateur et le serveur local doivent rester
accessibles. Ce timer local ne fonctionne pas dans le cloud, ne réveille pas une machine
éteinte et ne rattrape pas les créneaux manqués. Après retrait de la version Node/CLI
référencée, il faut actualiser les chemins puis daemon-reload.

## Preuves d’installation

- Validation `systemd-analyze --user verify` des deux unités : code 0.
- Démarrage manuel du service le 12 novembre à 09:05:10 CET : journal
  `Queued message 01a1ceca-0e7e-7471-94e1-cd7d978e1541 for thread …`.
- Timer enabled/active ; premier passage annoncé à 10:35:35 CET, puis toutes les 90 minutes.
- Les sessions ne sont pas déclarées actives par le seul succès du timer ; vérifier leurs
  états et leurs résultats lors de chaque rappel.

Ticket : `docs/tickets/closed/2026-11-12-verifier-la-reprise-des-sessions-toutes-les-90-minutes.md`.
