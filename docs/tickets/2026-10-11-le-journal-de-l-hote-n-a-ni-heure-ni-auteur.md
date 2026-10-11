# Le journal de l'hôte du démon n'a ni heure ni auteur

- **état** : ouvert (11 octobre 2026, en cherchant qui appelait `search_code` en rafale)
- **gravité** : diagnostic impossible (une rafale ne s'attribue pas)
- **atteignable en service** : oui, toujours
- **touche rag3weaver** : oui (et seulement)

## Ce que c'est

`~/.local/state/rag3weaver/daemon.log` reçoit les lignes de l'hôte et de ses backends
(`[démon] …`, `[rag3weaver] …`, `[crochet search_code] tu — rien à dire`) **sans heure, sans
session, sans backend**. L'orchestration a vu une rafale de 81 lignes du crochet de
`search_code` sur l'hôte de Lucie et a demandé qui bouclait : on ne peut le dire qu'en
recoupant à la main avec ce qu'on sait avoir lancé (ici, très probablement les boucles de
vérification de la montre de la session mémoire longue, qui appelaient `search_code` toutes
les 3 s entre 16 h 55 et 17 h 10 — mais rien dans le journal ne le prouve).

## Ce qu'il faut pour le fermer

Chaque ligne du journal de l'hôte porte l'heure (ISO, à la milliseconde) et, quand elle
naît d'un appel, la session et le client (`mcp (pid N)`) et le backend. Le crochet dit son
outil et son appelant plutôt que « tu ». Témoin : deux clients appellent `search_code`, et le
journal attribue chaque ligne du crochet au bon.
