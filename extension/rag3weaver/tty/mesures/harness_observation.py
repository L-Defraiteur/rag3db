"""Observation boundary for the tty harness; transport bytes are not an image."""


def screen_contains(raw, rendered, needle):
    # Oracle historique, conservé ici avant le témoin rouge discriminant.
    return needle.encode() in raw
