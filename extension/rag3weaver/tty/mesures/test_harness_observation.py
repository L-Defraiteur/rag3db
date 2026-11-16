"""Hermetic screen oracles: no terminal, host, process or Cargo needed."""
import unittest
import json
from pathlib import Path
import tempfile
from harness_observation import screen_contains, approval_and_end, ObservationArchive


class ScreenOracleTests(unittest.TestCase):
    def test_correct_fragmented_frame_is_visible_without_contiguous_raw_text(self):
        needle = 'Maj-Entrée : à la ligne'
        raw = 'Maj-Entr'.encode() + b'\x1b[90m' + 'ée : à la ligne'.encode()
        self.assertNotIn(needle.encode(), raw)
        self.assertTrue(screen_contains(raw, '› ' + needle, needle))

    def test_stale_raw_text_does_not_make_a_wrong_current_screen_green(self):
        needle = "LE CERCLE T'ÉCOUTE"
        self.assertFalse(screen_contains(needle.encode(), 'attend ton accord', needle))


class ObservationArchiveTests(unittest.TestCase):
    def test_next_phase_never_erases_raw_input_screen_or_fake_host_journal(self):
        with tempfile.TemporaryDirectory() as temp:
            archive = ObservationArchive(temp)
            first = archive.new_phase()
            first.output('t1', b'frame\x1b[1G')
            first.input('t1', b'o', 1)
            (first.path / 'journal.jsonl').write_text('{"kind":"answer"}\n')
            first.screen('t1', 'screen one', 1, 0)
            first.checkpoint('assertion-failed')
            before = {p.name: p.read_bytes() for p in first.path.iterdir()}
            second = archive.new_phase()
            second.output('t2', b'next')
            second.checkpoint('stop')
            self.assertEqual(before, {p.name: p.read_bytes() for p in first.path.iterdir()})
            with self.assertRaises(FileExistsError):
                ObservationArchive(temp)

    def test_protocol_and_visible_screen_are_independent_judges(self):
        posts = [{'chemin':'/ide/teammate/mailbox', 'corps':{'kind':'answer','approved':True,'reply_to':'a1'}}]
        mailbox = [{'kind':'ended'}]
        self.assertTrue(approval_and_end(posts, mailbox, 'ended'))
        self.assertFalse(approval_and_end([], mailbox, 'ended'))
        self.assertFalse(approval_and_end(posts, [], 'ended'))
        self.assertFalse(approval_and_end(posts, mailbox, 'running'))
        self.assertFalse(screen_contains(b'finished', 'still waiting', 'finished'))


if __name__ == '__main__':
    unittest.main()
