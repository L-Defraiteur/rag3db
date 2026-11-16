"""Hermetic screen oracles: no terminal, host, process or Cargo needed."""
import unittest
from harness_observation import screen_contains


class ScreenOracleTests(unittest.TestCase):
    def test_correct_fragmented_frame_is_visible_without_contiguous_raw_text(self):
        needle = 'Maj-Entrée : à la ligne'
        raw = 'Maj-Entr'.encode() + b'\x1b[90m' + 'ée : à la ligne'.encode()
        self.assertNotIn(needle.encode(), raw)
        self.assertTrue(screen_contains(raw, '› ' + needle, needle))

    def test_stale_raw_text_does_not_make_a_wrong_current_screen_green(self):
        needle = "LE CERCLE T'ÉCOUTE"
        self.assertFalse(screen_contains(needle.encode(), 'attend ton accord', needle))


if __name__ == '__main__':
    unittest.main()
