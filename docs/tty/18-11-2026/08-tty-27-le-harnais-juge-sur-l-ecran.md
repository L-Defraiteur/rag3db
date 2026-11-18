# tty-27 : le harnais juge sur l'écran de tmux

Session tty, 18 novembre 2026. L'entrée ajoutée par l'évaluation du travail de Codex (`docs/tty/18-11-2026/01-evaluation-du-travail-de-codex.md`, § 3 : « le harnais instrumenté, en lot à part, s'il sert »). Il a servi deux fois le 18 nov. : les séances 1h (tty-23) et 1i (tty-26) rougissaient sur les octets alors que l'écran montrait le bon texte.

## Le défaut

Sous tmux, c'est tmux qui redessine l'écran : il n'envoie au terminal extérieur que ce qui change, cellule par cellule. Un texte peut donc être à l'écran sans que ses octets arrivent jamais d'un seul tenant, et le harnais, qui cherchait les octets, le disait absent.

## Le correctif

- `mesures/screen_oracle.py`, `visible(text, raw, screen, under_tmux)`. Sous tmux, l'écran (`capture-pane`) juge, et un écran qui ne montre plus le texte n'est pas contredit par des octets passés. Hors tmux, ou sous tmux sans écran lisible (la séance déjà fermée), les octets jugent.
- `mesures/mesure.py`, `voir(t, texte, apres, borne)` attend un texte visible selon cette règle. `ecran_tmux` remonte en tête du script.
- La séance 1f (Ctrl-O) juge ses trois contrôles de visionneuse par `voir` : le dernier appel (« ⟨2/2⟩ »), l'appel d'avant (« let x1 = 1; ») et la fin de l'extrait.
- Les séances de mesure (latence, écho, débit) gardent les octets : elles mesurent leur arrivée.
- Les séances 1h et 1i, en branche, passeront à `voir` quand elles iront sur master.

## Les témoins

- `mesures/test_screen_oracle.py`, quatre cas (l'écran juge sous tmux, les octets hors tmux, le repli sans écran, un écran qui ne montre plus le texte). Vu rouge sur une coquille (88e11387e7, trois cas sur quatre), puis vert.
- Le harnais entier reste VERT sous `tmux-local-ouvert`, sur le binaire de master.

Le ticket `docs/tickets/closed/2026-11-18-le-harnais-du-terminal-juge-sur-les-octets-sous-tmux.md` est fermé par ce lot.
