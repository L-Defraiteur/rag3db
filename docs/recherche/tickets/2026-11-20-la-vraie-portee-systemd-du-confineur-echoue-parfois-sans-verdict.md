# Le confineur meurt en seconde victime du noyau quand sa commande sature la mémoire, et ne rend pas de verdict

- **État** : ouvert (20 novembre 2026, session recherche).
- **Gravité** : défaut produit. Une commande qui sature la mémoire de sa portée fait tuer le confineur lui-même, et
  l'appelant ne reçoit pas de `verdict.json`. Sur la tablette, chaque OOM de portée fait aussi une notification sur
  l'écran de Lucie.
- **Atteignable en service** : oui, toute passe confinée dont la commande dépasse son enveloppe mémoire.
- **Touche rag3weaver** : oui (`extension/rag3weaver/scripts/test_confiner_validation.py`).
- **Famille** : produit
- **Pour** : recherche (suivi ; toute réattribution passe par l'orchestration).
- **Signalé par** : recherche.

## Ce que c'est

`Montage.test_vraie_portee_systemd_crevee_dit_l_enveloppe` lance une vraie portée `systemd-run --user --scope`
plafonnée à 100 Mo, qui crève. Puis il lit `verdict.json` dans le dossier des preuves. À luciepc, le 20 novembre vers
19 h 30, il a échoué **une fois sur cinq sur master**, et une fois sur cinq avec le lot `confineur-grace` : le fichier
`verdict.json` n'existe pas quand le test le lit (`FileNotFoundError` dans `read_text`, l. 294). Ici, à la tablette,
il était vert.

## Pour le fermer

Lire, à l'échec, ce que contient le dossier des preuves. Soit la portée est arrêtée par systemd avant que le
confineur n'écrive son verdict (alors le test doit attendre la fin du confineur, et non lire tout de suite), soit le
dossier attendu n'est pas le bon. Le témoin est le test joué vingt fois de suite à luciepc, toujours vert.

## Le tueur, nommé par le journal (20 novembre 2026, 21 h, recherche, à luciepc)

`journalctl -k` sur un tour en échec (21 h 02 min 10 s, master, hors de `poste lourd`) :
- 21:02:10.135077 : `Memory cgroup out of memory: Killed process 3373711 (python3)`, l'enfant qui alloue (anon-rss
  79 Mo), dans la portée `run-p3373687-…scope` ;
- 21:02:10.138223, 3 ms plus tard : `Killed process 3373687 (python3)`, **le confineur lui-même**, qui donne son
  pid au nom de la portée.

Ce n'est ni systemd-oomd (`journalctl -u systemd-oomd` : aucune entrée), ni l'enveloppe de `poste`, ni l'arrêt de la
portée par systemd (`OOMPolicy=stop`, arrêt en 10 s). Le confineur vit dans la portée plafonnée, comme la commande ;
après la commande, le memcg reste au-dessus de sa limite, et le noyau continue jusqu'à lui. D'où le -9 et le dossier
sans `verdict.json` (`autorite.json`, `bus.xml`, `recette.json` seulement).

**Ce qui ne suffit pas** : monter le `oom_score_adj` de la commande à 1000 (branche `confineur-oom`, écartée). Il
choisit l'ordre des victimes, pas leur nombre : le confineur meurt encore en -9 (luciepc, 21 h 07).

## Le correctif retenu (c7, 20 novembre 2026, avec le mot de Lucie : « s'arrêter nous-mêmes quand la mémoire n'est pas dispo »)

- **`poste` pose `Delegate=yes`** sur sa portée (G).
- **Le confineur crée deux feuilles** dans la portée : `superviseur`, où il se range, et `commande`, où il lance la
  commande. Toute l'enveloppe de `poste` (CPU, IO, places, plafond) reste au-dessus des deux.
- **La feuille `commande`** reçoit `memory.high` = le budget (le noyau ralentit, ne tue pas) et `memory.max` = le
  plafond de la portée moins une marge (le garde-fou seul).
- **Le confineur surveille** `memory.current` et `memory.events` de la feuille. Au budget franchi, il arrête la
  commande lui-même, de façon ordonnée (SIGTERM, la grâce, puis SIGKILL), et écrit « mémoire dépassée » au verdict.
  L'enveloppe se lit à la feuille.
- **Les témoins**, à luciepc seulement (jamais sur un poste où une session graphique est ouverte) :
  - la vraie portée : rouge sur master (le confineur en -9, verdict absent, `journalctl -k`), vert avec le correctif,
    vingt tours ;
  - une commande qui alloue au-delà du budget est arrêtée par le confineur, verdict « mémoire dépassée » écrit, sans
    oom_kill du noyau ;
  - un OOM réel du noyau dans la feuille laisse quand même un verdict.
- Le code attend `Delegate=yes` dans `poste`.

