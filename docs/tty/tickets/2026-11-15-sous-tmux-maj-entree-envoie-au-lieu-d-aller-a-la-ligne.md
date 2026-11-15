# Sous tmux, Maj-Entrée envoie au lieu d'aller à la ligne

- **État** : ouvert. Constaté par la session tty le 15 nov. 2026 (lot tty-10), par le harnais sous tmux ; repli dit, non résolu.
- **Famille** : produit
- **Gravité** : gêne. Une saisie qu'on voulait sur plusieurs lignes part à moitié : « un », Maj-Entrée, envoie « un » à l'équipier.
- **Atteignable en service** : oui, pour qui lance le terminal sous tmux, dans un émulateur qui distingue Maj-Entrée (kitty, WezTerm, foot, Ghostty).
- **Touche rag3weaver** : oui (le crate `extension/rag3weaver/tty`).

## Ce que c'est

Hors de tmux, le terminal demande le protocole clavier de kitty à un émulateur qui l'annonce, et Maj-Entrée lui arrive en CSI u (`ESC[13;2u`) : un saut de ligne. Sous tmux, c'est tmux qui répond à la demande. Dans sa configuration par défaut (`extended-keys off`), il n'annonce pas le protocole et traduit la Maj-Entrée de l'émulateur extérieur en Entrée simple. Le terminal ne peut pas distinguer cette Entrée d'une vraie, et envoie.

## Le repli, dit

Sous tmux (variable `TMUX`), la saisie vide affiche : « sous tmux, Maj-Entrée envoie : Alt-Entrée pour la ligne ». Alt-Entrée passe à la ligne partout, sous tmux compris.

## Le témoin

`extension/rag3weaver/tty/mesures/mesure.py`, séance 1e, lanceur `tmux-ssh` : la boîte reçoit `["un", "deux\ntrois"]` au lieu de `["un\ndeux\ntrois"]`. Le harnais l'admet seulement si l'indication de tmux est à l'écran (`repli_tmux` dans `docs/tty/15-11-2026/mesures/`).

## Ce qu'il faudrait pour le fermer

Que tmux passe Maj-Entrée à l'application : `set -g extended-keys on`, plus, selon l'émulateur, `terminal-features` avec `extkeys`. Ce sont des réglages de l'utilisateur, alors que rien ne doit être à installer ni à configurer en plus de rag3weaver. Deux pistes restent à essayer avant de le dire impossible :
- demander à tmux le mode `modifyOtherKeys` (`CSI > 4;1 m`), qu'un tmux réglé en `extended-keys on` honore, et voir ce que fait un tmux par défaut ;
- lire le réglage de tmux (`tmux show -gv extended-keys`) pour dire, dans l'indication, la commande exacte qui l'active.
