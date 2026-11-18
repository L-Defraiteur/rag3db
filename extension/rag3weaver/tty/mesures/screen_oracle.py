"""**What is visible, for the harness** (18 nov. 2026): under tmux, the screen tmux shows (`capture-pane`)
judges, since tmux redraws the screen itself and only sends what changed; elsewhere, the bytes received."""


def visible(text, raw, screen, under_tmux):
    """Is `text` visible: on `screen` (the text of `capture-pane`) under tmux, else in `raw` (the bytes).
    Under tmux without a screen (the session already closed), the bytes say what they can."""
    if under_tmux and screen is not None:
        return text in screen
    return text.encode() in raw
