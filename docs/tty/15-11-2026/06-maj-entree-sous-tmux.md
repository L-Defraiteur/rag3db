# tty-15 : Maj-Entrée sous un tmux qui le permet

Session tty, 15 novembre 2026. Ce lot joue les deux pistes du ticket `docs/tty/tickets/2026-11-15-sous-tmux-maj-entree-envoie-au-lieu-d-aller-a-la-ligne.md`, à la demande de c7.

## Ce qui a été trouvé

- Un tmux par défaut (`extended-keys off`) ignore toute demande des touches étendues : rien à faire de l'intérieur, et le repli dit de tty-10 reste.
- Avec `extended-keys on` ou `always`, tmux honore la demande du mode 1 (`CSI > 4 ; 1 m`). Il rend alors Maj-Entrée dans le format de `extended-keys-format` :
  - en `xterm`, son défaut, c'est `ESC[27;2;13~`, que crossterm rejette (`parse_csi_special_key_code` ne connaît pas `27`). La demander y perdrait Maj-Entrée tout à fait ;
  - en `csi-u`, c'est `ESC[13;2u`, que crossterm lit.

## Ce qui change

- Sous tmux, le terminal lit `tmux show -gv extended-keys` et `extended-keys-format`. Si le premier est à `on` ou `always` et le second à `csi-u`, il demande le mode 1, le rend à la sortie (`CSI > 4 ; 0 m`), et Maj-Entrée passe à la ligne : l'indication dit alors « Maj-Entrée : à la ligne ».
- Sinon, rien n'est demandé, et l'indication dit les deux réglages qui l'ouvrent : « sous tmux, Maj-Entrée envoie : Alt-Entrée pour la ligne (ou, dans tmux : set -g extended-keys on ; set -g extended-keys-format csi-u) ». Ce sont des réglages à Lucie, jamais posés par le terminal : le défaut reste « rien à configurer », et ce n'en est que le remède dit.
- **Un défaut trouvé par le harnais.** L'indication, plus longue, était coupée par le début, comme une saisie, et perdait « sous tmux, Maj-Entrée envoie ». Elle garde désormais son début et finit par `…` (rouge puis vert).

## Les témoins

- Les tests du crate : la lecture du réglage de tmux, dans tous ses cas (le défaut, le format `xterm`, `csi-u` avec `on` et avec `always`, la fin de ligne de tmux, un tmux muet, un tmux trop vieux pour le format), l'indication complète, et l'indication étroite qui garde son début. Le crate reste à 97 tests avec la vue.
- Le harnais gagne le lanceur `tmux-local-ouvert`, un tmux réglé, sous lequel la séance multiligne exige que Maj-Entrée passe à la ligne et que l'indication le dise :

| Passe | Binaire | Verdict |
|---|---|---|
| `mesures/tmux-ouvert-rouge-avant.json` (tmux réglé) | tty-13 | ROUGE : Maj-Entrée envoie, aucune indication de Maj-Entrée |
| `mesures/tmux-ouvert.json` (tmux réglé) | tty-15 | VERT : `un\ndeux\ntrois` reçu, l'indication dit Maj-Entrée |
| `mesures/tmux-defaut.json` (tmux par défaut) | tty-15 | VERT avec le repli dit : `un`, puis `deux\ntrois`, et l'indication de tmux à l'écran |
