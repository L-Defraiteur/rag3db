# Le harnais du terminal juge sur les octets sous tmux

- **État** : corrigé. Lot tty-27 (18 nov. 2026), page `docs/tty/18-11-2026/08-tty-27-le-harnais-juge-sur-l-ecran.md`.
- **Famille** : produit
- **Gravité** : faux rouges du harnais : un texte à l'écran dit absent.
- **Atteignable en service** : non, c'est le harnais de mesure du terminal.
- **Touche rag3weaver** : oui (`extension/rag3weaver/tty/mesures/mesure.py`).

## Ce que c'est

Sous tmux, tmux redessine l'écran lui-même et n'envoie que les cellules qui changent : un texte peut être à l'écran sans que ses octets arrivent d'un seul tenant. Le harnais cherchait les octets ; les séances 1h et 1i (18 nov.) l'ont dit absent alors que `capture-pane` le montrait. Codex avait relevé le même point sur sa branche (`cx-tty-english-aliases-7`, ticket « le harnais perd les traces des séances et juge la fin sur les octets »), qui n'est pas sur master.

## Le correctif

`mesures/screen_oracle.py` (`visible`) et `voir()` dans `mesure.py` : sous tmux, l'écran juge ; ailleurs, les octets.
