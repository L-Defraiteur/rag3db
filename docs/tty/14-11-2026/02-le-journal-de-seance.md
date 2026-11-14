# tty-7 : le journal de séance, et ce que dit le terminal sous mosh

Session tty, 14 novembre 2026. Ce lot ferme le ticket ouvert par tty-6 : `docs/tickets/closed/2026-11-14-sous-mosh-l-historique-du-terminal-se-perd.md`.

## Le défaut

La vue inline écrit l'historique (le texte de l'agent, les outils, la boîte) dans le défilement du terminal, et ne redessine que trois lignes en bas. mosh ne synchronise que l'écran et ne garde aucun historique de défilement : sous mosh, ce qui sort de l'écran ne se relit plus. Les gens n'installent rien en plus de rag3weaver, donc le repli doit venir du terminal lui-même, et il doit se dire.

## Ce qui change

- **`--journal <fichier>`** écrit en texte brut, sans couleurs, chaque ligne envoyée dans l'historique : le texte replié à la largeur, les blocs de code, les outils et la boîte. En `--plain`, il reçoit le même texte que la sortie. L'option marche partout : ssh, tmux ou terminal local.
- **Sous mosh, un journal s'ouvre de lui-même** quand `--journal` n'est pas donné, dans `$XDG_STATE_HOME/rag3weaver/tty/` (`~/.local/state/rag3weaver/tty/` par défaut, `%LOCALAPPDATA%` sous Windows), sous le nom `seance-<secondes>-<pid>.txt`.
- **La commande de relecture s'affiche.** Sous mosh, la ligne du haut de la zone vivante dit, en sourdine, `sous mosh, l'historique ne se relit pas au défilement : less <journal>` tant qu'aucun texte n'y passe. La ligne d'état reste à ce que l'équipier attend. Quand la largeur manque, c'est la fin, la commande, qui reste. En `--plain`, la même phrase part une fois sur la sortie d'erreur.
- **La détection.** mosh ne pose aucune variable d'environnement sûre. On cherche donc un ancêtre du processus nommé `mosh-server`, par `/proc` sous Linux. Ailleurs (macOS), ou sous un tmux lancé dans mosh (l'ancêtre est alors le serveur tmux, qui garde son propre historique), rien n'est détecté et rien n'est dit : `--journal` reste disponible.
- **Le dossier ne grossit pas sans fin.** Chaque séance sous mosh ouvre son journal : la première passe du harnais en a laissé 36 à luciepc, un par séance, les séances courtes comprises. Ouvrir un journal de lui-même efface d'abord les `seance-*.txt` du dossier modifiés depuis plus de sept jours (`journal::GARDE`), et rien d'autre. Le test est rouge sans ce rangement, vert avec. Le harnais efface à la fin les journaux qu'il a fait naître.
- Un journal qui ne s'ouvre pas de lui-même (aucun dossier d'état) se dit, et la séance continue. Un `--journal` qui ne s'ouvre pas est une erreur au lancement.

Le code : `extension/rag3weaver/tty/src/journal.rs` (le journal, le dossier d'état, la détection), `vue.rs` (`Vue::repli`), `tui.rs` (`dessiner` écrit au journal ce qu'il envoie à l'historique), `main.rs` (l'option, le choix du journal, le texte brut).

## Le témoin, rouge puis vert

Le harnais `extension/rag3weaver/tty/mesures/mesure.py` gagne deux séances :

- **1b, sous tous les lanceurs** : le terminal avec `--journal <dossier>/seance-journal.txt`. Le journal, lu sur le poste distant, doit porter les 200 mots du pas scripté (`mot0` à `mot199`) et son bloc de code. Sous mosh, l'écran doit dire `less <journal>`. Ailleurs, il ne doit pas parler de mosh.
- **1c, sous mosh seul** : le terminal sans `--journal`. Un seul journal neuf doit paraître dans `~/.local/state/rag3weaver/tty/`, complet, et l'écran doit le nommer. Le harnais l'efface ensuite.

| Passe (mosh portable → luciepc) | Binaire | Verdict |
|---|---|---|
| `mesures/mosh-rouge-avant-journal.json` | tty-6 (`ce4a3140a`) | ROUGE : `--journal` refusé, aucun journal, rien de dit, aucun journal ouvert de lui-même |
| `mesures/mosh-journal.json` | tty-7 | VERT : 0 mot manquant dans les deux journaux, la commande dite, un journal neuf |
| `mesures/ssh-journal.json` (ssh seul) | tty-7 | VERT : journal complet, rien ne parle de mosh |
| `mesures/mosh-journal-range.json` | tty-7, avec le rangement | VERT ; le dossier d'état de luciepc est vide après la passe |

Les autres séances du harnais (touches, redimensionnement, couleurs des cinq thèmes) restent vertes sous les deux lanceurs.

Les tests du crate passent de 39 à 45 avec la vue, et de 10 à 14 sans elle. Ils couvrent la lecture de `/proc/<pid>/stat` avec un nom à parenthèses, la remontée des ancêtres (mosh trouvé, sshd non, un cycle s'arrête), le journal en ajout avec un chemin cité pour le shell, le rangement des seuls vieux journaux de séance, les lignes de l'historique écrites au journal sans couleurs et repliées à la largeur, et le repli en sourdine qui cède la ligne au texte en cours.

## La latence, départagée

La première passe verte sous mosh donnait 80,7 ms de médiane d'affichage, contre 47,6 au rouge. Rejouée aussitôt, binaire neuf, puis ancien, puis neuf :

| | Affichage, médiane / p95 | Écho, médiane |
|---|---|---|
| tty-7 | 47,1 / 51,5 ms | 17,4 ms |
| tty-6 | 48,3 / 60,6 ms | 18,1 ms |
| tty-7 | 47,7 / 49,5 ms | 16,8 ms |

Le journal ne coûte rien de visible. L'écart de la première passe venait du poste : la charge de luciepc était de près de 10 pendant cette passe, et d'environ 3 au rejeu (`mesures/mosh-latence-*.json`).
