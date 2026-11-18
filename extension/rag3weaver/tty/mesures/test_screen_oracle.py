#!/usr/bin/env python3
"""The harness's oracle for what is visible (18 nov. 2026): under tmux, tmux redraws the screen itself and
only sends what changed, so a text can be on screen without its bytes ever arriving whole; the harness judges
on the screen tmux shows (`capture-pane`). Elsewhere there is no screen to read, and the bytes received judge.

    python3 test_screen_oracle.py
"""
import unittest

from screen_oracle import visible


class VisibleTest(unittest.TestCase):
    def test_under_tmux_the_screen_judges_not_the_bytes(self):
        # tmux sent the line in pieces: the bytes never hold it whole, the screen does.
        raw = b"\x1b[3;1Hseconde \x1b[3;9Hligne du message"
        screen = "⟨3/4⟩ message\n› Message de @ED\nseconde ligne du message\n"
        self.assertTrue(visible("seconde ligne du message", raw, screen, under_tmux=True))
        self.assertFalse(visible("seconde ligne du message", raw, None, under_tmux=False))

    def test_without_tmux_the_bytes_judge(self):
        self.assertTrue(visible("à toi", "› à toi".encode(), None, under_tmux=False))
        self.assertFalse(visible("absent", b"rien", None, under_tmux=False))

    def test_under_tmux_without_a_screen_falls_back_on_the_bytes(self):
        # capture-pane failed (the session already closed): the bytes still say what they can.
        self.assertTrue(visible("fini", b"... fini", None, under_tmux=True))

    def test_a_screen_that_does_not_show_it_is_not_overruled_by_the_bytes(self):
        # The bytes passed once, but the screen no longer shows the text: not visible.
        self.assertFalse(visible("vieux texte", b"vieux texte", "un autre écran\n", under_tmux=True))


if __name__ == "__main__":
    unittest.main()
