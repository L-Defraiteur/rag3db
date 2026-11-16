# Sous tmux, Maj-Entrée envoie au lieu d'aller à la ligne

- **État** : ouvert pour le tmux par défaut, résolu pour un tmux réglé (lot tty-15). Constaté par la session tty le 15 nov. 2026 (lot tty-10), par le harnais sous tmux.
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

## Ce que tty-15 a fait des deux pistes

- **Demander `modifyOtherKeys`** : un tmux par défaut (`extended-keys off`) l'ignore, sa page le dit (« when set to off, this feature is disabled »). Avec `extended-keys on` (ou `always`), il honore le mode 1. Il rend alors Maj-Entrée dans son format `extended-keys-format` : en `xterm` (le défaut, `ESC[27;2;13~`), que crossterm ne lit pas, de sorte que la demander y perdrait Maj-Entrée tout à fait ; en `csi-u` (`ESC[13;2u`), que crossterm lit.
- **Lire le réglage** : le terminal lit `tmux show -gv extended-keys` et `extended-keys-format`.
  - Si les deux sont ouverts, il demande le mode 1 (`CSI > 4 ; 1 m`), le rend à la sortie, et Maj-Entrée passe à la ligne sous tmux.
  - Sinon, rien n'est demandé, et l'indication dit les deux réglages : « sous tmux, Maj-Entrée envoie : Alt-Entrée pour la ligne (ou, dans tmux : set -g extended-keys on ; set -g extended-keys-format csi-u) ». Trop étroite, elle garde son début.

Le harnais le joue sous `tmux-local-ouvert` : un tmux réglé avec ces deux options et `terminal-features 'xterm*:extkeys'`, rouge contre tty-13, vert contre tty-15. Sous `tmux-local`, le tmux par défaut, le repli reste dit, et vert (`docs/tty/15-11-2026/mesures/tmux-*.json`).

## Ce qui reste ouvert

Que tmux passe Maj-Entrée à l'application : `set -g extended-keys on`, plus, selon l'émulateur, `terminal-features` avec `extkeys`. Ce sont des réglages de l'utilisateur, alors que rien ne doit être à installer ni à configurer en plus de rag3weaver. Deux pistes restent à essayer avant de le dire impossible :
- demander à tmux le mode `modifyOtherKeys` (`CSI > 4;1 m`), qu'un tmux réglé en `extended-keys on` honore, et voir ce que fait un tmux par défaut ;
- lire le réglage de tmux (`tmux show -gv extended-keys`) pour dire, dans l'indication, la commande exacte qui l'active.

## Observation RO de la continuation alias du16 novembre

Sourceproduit6d/contrôleur9d, tmux-local-ouvert : harness.log signale
`demande_kitty=false`, `aide_maj_entree=false`, `aide_alt_entree=false`,
`aide_tmux=false` mais `textes_recus=['un\ndeux\ntrois']` **correct**.
Ce n'est donc pas une preuve que Maj-Entrée envoie au lieu de sauter une ligne.
Sous multiplexeur, demande_kittyfalse seule n'est pas un échec attendu ; le
rouge porte sur aide_maj non détectée avec la donnée de saisie correcte.

L'oracle recherche une phrase contiguë dans les octets de Terminal.tout,
pas dans un écran VT reconstruit. La vue choisit pourtant l'aide Maj-Entrée
si le tmux lu permet CSIu ; aucun diff de tui/vue/saisie entre927 et6d.
Sans capture persistée, on ne peut trancher affichage réellement absent,
lecture du réglage ou fragmentation des mises à jour du terminal.
Ne pas attribuer ce constat aux nouveaux alias ni modifier le terminal
par hypothèse. Aucun réglage de Lucie ou serveur tmux partagé touché.

Preuves/limites et plan d'oracles :
`2026-11-16-le-harnais-perd-les-traces-des-seances-et-juge-la-fin-sur-les-octets.md`.
Suite à proposer : capture privée pendant la séance et rendu reconstruit,
en conservant l'exigence de donnée multiligne exacte et les deux modes tmux.
Aucun nouveau test/capture après arrêt, aucun nouveau GO.
