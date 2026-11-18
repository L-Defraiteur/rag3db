"""**What is visible, for the harness** (18 nov. 2026): under tmux, the screen tmux shows (`capture-pane`)
judges, since tmux redraws the screen itself and only sends what changed; elsewhere, the bytes received."""


def visible(text, raw, screen, under_tmux):
    """Is `text` visible: on `screen` (the text of `capture-pane`) under tmux, else in `raw` (the bytes)."""
    return False
