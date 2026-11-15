# Une passe en file sous `poste lourd` disparaît sans trace à luciepc

- **État** : ouvert (15 novembre 2026, hotfixs)
- **Famille** : produit
- **Gravité** : une passe perdue sans rien dire. Le lot attend un verdict qui ne vient jamais, et la file avance sans
  lui.
- **Atteignable en service** : non (outillage des passes)
- **Touche rag3weaver** : non (l'outil `poste` et la façon de lancer une passe à luciepc)
- **Pour** : G (l'outil `poste`), avec hotfixs

## Ce qu'on voit

Deux fois de suite, le 15 novembre au soir, une passe lancée à luciepc par ssh, dans un `setsid nohup` :

```bash
cd ~/.cache/rag3weaver-build && (CIBLE=target-hotfixs-etude setsid nohup ./poste lourd timeout 7200 \
  ./passe_epinglee.sh rag3db-extension-dite ~/.cache/rag3weaver-build/journaux-hotfixs-extension-dite \
  une_extension_notee_qui_ne_se_charge_pas > ~/.cache/rag3weaver-build/journaux-hotfixs-extension-dite.lance.log 2>&1 < /dev/null &)
```

- **21 h 12** : `poste --qui` la montre « attend lourd ». À 21 h 42, il n'y a plus aucun processus, le journal de
  lancement est vide (0 octet), le dossier des journaux est vide.
- **21 h 44** : relancée à l'identique (pid 897922, « attend lourd depuis 21:44:26 »). Avant 21 h 49, plus de
  processus, mêmes journaux vides.
- **Rien n'est dit** : `journalctl --user` et `journalctl -k` de luciepc, entre 21 h 11 et 21 h 50, ne disent ni OOM,
  ni portée tuée, ni rien sur `poste` ou sur la passe.

La passe meurt **pendant qu'elle attend sa place**, avant d'avoir écrit une ligne. La piste de c7 : le ssh qui l'a
lancée se ferme, et un SIGHUP emporte la file. Mais un `setsid nohup` devrait déjà en protéger.

## Le contournement

À 21 h 49, la même passe a été lancée dans une unité, qui survit à son ssh et se suit par `journalctl --user -u` :

```bash
systemd-run --user --unit hotfixs-extension-dite --working-directory=$HOME/.cache/rag3weaver-build \
  -E CIBLE=target-hotfixs-etude bash -c "./poste lourd timeout 7200 ./passe_epinglee.sh …"
```

C'est déjà la façon d'archi (`systemd-run --user --unit archi-…`).

## La piste de logind, écartée (15 nov., 21 h 52)

c7 soupçonnait logind (KillUserProcesses). Relevé à luciepc :

- `loginctl show-user lucied -p Linger` : `Linger=yes` ;
- `KillUserProcesses` : aucun réglage dans `/etc/systemd/logind.conf` ni `logind.conf.d/`. Avec systemd 262 sous Arch,
  le défaut vaut `no` ;
- `journalctl -b` de 21 h 05 à 21 h 55 : des dizaines de sessions tty « New session cNNN » puis « Removed session »
  (une par ssh). Aucun scope arrêté, aucun processus tué, aucun OOM noyau ; `journalctl --user` est muet à 21 h 12 et
  à 21 h 49.

logind ne tue donc pas les processus d'une session fermée, et la cause reste ouverte. La règle de l'unité
`systemd-run --user` reste en vigueur.

## Pour fermer

1. La cause : qui tue un `poste lourd` en attente (le SIGHUP, la garde des worktrees, un ménage de la file), lue sur
   un cas reproduit, par exemple un `poste lourd sleep` en file lancé par un ssh puis fermé.
2. Soit `poste` écrit au moins une ligne avant d'attendre (« en file depuis … ») et dit sa mort, soit la procédure de
   push dit de lancer toute passe de luciepc dans une unité `systemd-run --user`.
