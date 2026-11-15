# tty-10 : l'édition sur plusieurs lignes, et Maj-Entrée

Session tty, 15 novembre 2026, au mot de Lucie : « je veux multiligne l'édition de texte, avec shift enter pour passer à la ligne suivante ».

## Ce qui change

- **La saisie s'écrit sur plusieurs lignes.** Chaque ligne se replie à la largeur, et la zone du bas grandit avec elle jusqu'à huit lignes. Au-delà, une fenêtre suit le curseur, et une ligne coupée au bord se dit par `… `. Le curseur de l'écran est sur sa ligne et sa colonne. ↑ et ↓ changent de ligne dans la saisie avant de parcourir l'historique, comme depuis tty-8.
- **La zone change de hauteur.** ratatui ne sait pas changer la hauteur d'une zone inline une fois ouverte. Le terminal l'efface donc, remet le curseur à son début et rouvre une zone à la nouvelle hauteur, en faisant défiler l'écran s'il manque de place en bas, comme à l'ouverture. Cela ne se produit que lorsque le nombre de lignes de la saisie change.
- **Maj-Entrée** passe à la ligne là où l'émulateur annonce le protocole clavier de kitty (kitty, WezTerm, foot, Ghostty…). Le terminal le lui demande (`ESC[>1u`) seulement s'il l'annonce, et le lui rend à la sortie. Un émulateur qui ne l'annonce pas ne reçoit jamais la demande. Un CSI u qu'il enverrait quand même se lit comme une touche, jamais comme du texte.
- **Alt-Entrée** reste le saut de ligne partout.
- **La saisie vide dit comment passer à la ligne**, en sourdine : « Maj-Entrée : à la ligne », « Alt-Entrée : à la ligne », ou sous tmux « sous tmux, Maj-Entrée envoie : Alt-Entrée pour la ligne ».

## Sous tmux, une dégradation dite

tmux, dans sa configuration par défaut, répond lui-même à la demande, n'annonce pas le protocole, et fait de la Maj-Entrée de l'émulateur extérieur une Entrée simple : la saisie part. Le terminal ne peut pas la distinguer d'une vraie Entrée. Sous tmux (variable `TMUX`), l'indication le dit, et Alt-Entrée y passe à la ligne. La fermeture demanderait un réglage de tmux, contraire à « rien à installer ni à configurer en plus de rag3weaver » : le ticket `docs/tty/tickets/2026-11-15-sous-tmux-maj-entree-envoie-au-lieu-d-aller-a-la-ligne.md` garde les deux pistes à essayer.

## Les témoins

Les tests du crate, chacun rouge avant son code :
- la saisie repliée sur plusieurs lignes, la fenêtre qui suit le curseur, les idéogrammes, la ligne pleine qui pousse le curseur à la suivante ;
- la zone qui grandit, redescend après l'envoi et plafonne à huit lignes de saisie ;
- l'indication de la saisie vide, et celle de tmux.

Le crate passe de 63 à 66 tests avec la vue, et en garde 14 sans elle.

Le harnais (`mesures/mesure.py`, séance 1e) répond désormais comme un vrai terminal à la demande d'attributs (`ESC[c`), et, s'il joue un terminal kitty, à `ESC[?u`. Il tape « un », Maj-Entrée en CSI u (`ESC[13;2u`), « deux », Alt-Entrée, « trois », Entrée, devant les deux terminaux.

| Passe | Binaire | Verdict |
|---|---|---|
| `mesures/multiligne-ssh-rouge-avant.json` (ssh) | tty-9 | ROUGE : aucune demande du protocole kitty, aucune indication. Le texte arrivait déjà juste : Alt-Entrée et le CSI u étaient lus depuis tty-8 |
| `mesures/multiligne-ssh.json` | tty-10 | VERT : devant kitty, la demande part et l'indication dit Maj-Entrée ; sans kitty, rien ne part et elle dit Alt-Entrée ; la boîte reçoit `un\ndeux\ntrois` |
| `mesures/multiligne-mosh.json` | tty-10 | VERT : mosh répond à la demande sans annoncer le protocole, mais passe le CSI u, qui fait un saut |
| `mesures/multiligne-tmux-ssh.json`, `multiligne-tmux-local.json` | tty-10 | VERT avec le repli dit : la boîte reçoit `un`, puis `deux\ntrois`, et l'indication de tmux est à l'écran |

Les passes ssh et mosh ont tourné avant le commit de l'indication de tmux, qui ne joue que sous `TMUX`. Les autres séances du harnais restent vertes partout.
