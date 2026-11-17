"""Contre-exemples prépubliés : vrai Relay D45, aucun noyau factice ni syscall réel.

La capacité plan explicite est volontairement absente avant sa préparation.
Un TypeError exact sur ce paramètre est rendu FAIL nommé, tout autre ERROR reste
une erreur de fixture à arrêter. Aucun test de ce fichier n'est encore joué.
"""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
LEGACY_TEST = HERE.parent / 'rust-probe-relay/test_relay.py'
spec = importlib.util.spec_from_file_location('publication_baseline_fixtures', LEGACY_TEST)
fixtures = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixtures)
legacy = fixtures.module


class PublicationCounterexamples(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.clock = fixtures.Clock()
        self.transport = fixtures.FakeTransport(self.clock)
        self.relay = legacy.Relay(self.transport, self.root, self.clock, self.clock.sleep,
                                  queue_budget=300, transfer_cap=30, control_cap=5)

    def test_default_rust_build_rejects_extra_payload_before_any_ack(self):
        original = self.transport.contents
        def contents(kind):
            ledger, files = original(kind)
            if kind == 'build':
                files['undeclared-elf'] = b'extra file not in the strict Rust plan'
            return ledger, files
        self.transport.contents = contents
        state = self.transport.snapshot(5)
        with self.assertRaises(AssertionError):
            self.relay.publish('build', state)
        self.assertNotIn('ack:build', self.transport.calls)

    def test_inherited_runtime_elf_requires_prior_ack_not_only_a_matching_copy(self):
        self.transport.stage = 'runtime'
        folder = self.root / 'build'
        folder.mkdir()
        (folder / 'elf').write_bytes(self.transport.payload)
        state = self.transport.snapshot(5)
        with self.assertRaises(AssertionError):
            self.relay.publish('runtime', state)
        self.assertNotIn('ack:runtime', self.transport.calls)

    def test_explicit_four_stage_plan_is_not_silently_reduced_to_two(self):
        # Forme d'entrée de demande ; l'adaptateur de construction sera porté
        # vers les types validés, sans changer ces labels/cardinalités/ordre.
        plan_request = {'protocol_version': 1, 'stages': [
            {'label': 'global', 'executable_count': 5},
            {'label': 'agent-red1', 'executable_count': 1},
            {'label': 'agent-green1', 'executable_count': 1},
            {'label': 'review1', 'executable_count': 1}]}
        try:
            relay = legacy.Relay(self.transport, self.root, self.clock, self.clock.sleep,
                                  plan=plan_request)
        except TypeError as error:
            if "unexpected keyword argument 'plan'" in str(error):
                self.fail('EXPLICIT_ORDERED_PLAN_UNSUPPORTED_BY_REAL_BASELINE: ' + str(error))
            raise
        self.assertEqual(relay.run(), 0)
        stages = [row.removeprefix('ack:') for row in self.transport.calls if row.startswith('ack:')]
        self.assertEqual(stages, ['global', 'agent-red1', 'agent-green1', 'review1'])
        # Les huit occurrences doivent toutes avoir été transférées avant leurs ACK.
        transfers = [row for row in self.transport.calls if row.startswith('fetch:') and ':elf' in row]
        self.assertEqual(len(transfers), 8)


if __name__ == '__main__':
    unittest.main()
