"""Publication raccord: real temporary files/link, fake health/clock/authority."""
from pathlib import Path
import copy
import hashlib
import json
import os
import tempfile
import unittest
from unittest.mock import patch
import lifecycle_grant_producer as producer
import lifecycle_protocol as protocol


def fixture():
    uid = os.getuid()
    ready = dict(protocol.ready_bounds(40., 600.), deadline=600., nonce='fixture',
                 no_embedding_calls_authorized=True, phase=dict(
                     job='lifecycle-provider', pilot='f'*40, allocation='fixture',
                     authority=dict(path='/fixture/authority', pid=123, start='42',
                                    cgroup='/fixture', uid=uid, script_sha256='a'*64),
                     first_red_path='/fixture/first-red.json'))
    phase = dict(clock=protocol.READY_CLOCK,
                 authority=dict(etat='actif', pid=123, debut='42', cgroup='/fixture',
                                echeance=600., script_sha256='a'*64),
                 authority_uid=uid, process=dict(start='42', cgroup='/fixture', uid=uid, state='S'),
                 first_red_exists=False, snapshot_stable=True)
    return ready, phase


class Producer(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.ready, self.phase = fixture()
        self.path = self.root/'READY.json'
        self.path.write_text(json.dumps(self.ready))
        self.sha = hashlib.sha256(self.path.read_bytes()).hexdigest()
        self.dest = self.root/'PROVIDER_GO.json'
        self.now = 80.
        self.healths = 0
        self.after_health = lambda: None
        self.phase_reads = 0

    def snapshot(self, ready):
        self.phase_reads += 1
        return dict(copy.deepcopy(self.phase), snapshot_observed_monotonic=self.now)

    def observe(self, ready):
        self.healths += 1
        grant = dict(nonce=ready['nonce'], ready_sha256=self.sha,
                     no_embedding_calls_authorized=True, observed_monotonic=self.now,
                     valid_until_monotonic=120.)
        self.after_health()
        return grant

    def run_publish(self, **extra):
        return producer.publish_grant(self.path, self.dest, expected_sha256=self.sha,
                                      expected_nonce='fixture', observe_provider=self.observe,
                                      clock=lambda: self.now, phase_reader=self.snapshot, **extra)

    def refused(self):
        with self.assertRaises(ValueError): self.run_publish()
        self.assertFalse(self.dest.exists())
        self.assertEqual(list(self.root.glob('.grant-*')), [])

    def test_timely_publication_is_real_link_and_exclusive(self):
        grant = self.run_publish()
        self.assertEqual(protocol.decode(self.dest.read_bytes()), grant)
        self.assertEqual(self.dest.stat().st_mode & 0o777, 0o600)
        with self.assertRaises(FileExistsError): self.run_publish()
        self.assertEqual(self.healths, 1)

    def test_late_entry_does_not_contact_provider(self):
        self.now = 100.
        self.refused()
        self.assertEqual(self.healths, 0)

    def test_health_reserve_is_not_an_extended_admission(self):
        self.now = 96.
        self.refused()
        self.assertEqual(self.healths, 0)

    def test_health_finishing_after_cutoff_cannot_publish(self):
        self.after_health = lambda: setattr(self, 'now', 100.)
        self.refused()

    def test_delayed_fsync_is_checked_immediately_before_real_link(self):
        original = producer._stage
        def delayed(*args):
            result = original(*args)
            self.now = 100.
            return result
        with patch.object(producer, '_stage', side_effect=delayed): self.refused()

    def test_ready_replacement_or_changed_nonce_cannot_publish(self):
        def change():
            self.path.write_text(json.dumps(dict(self.ready, nonce='replaced')))
        self.after_health = change
        self.refused()

    def test_authority_termination_during_observation_cannot_publish(self):
        self.after_health = lambda: self.phase['authority'].update(etat='termine')
        self.refused()

    def test_first_red_stays_immutable_and_forbids_publication(self):
        red = self.root/'first-red.json'
        def refuse():
            red.write_bytes(b'original refusal')
            self.phase['first_red_exists'] = True
        self.after_health = refuse
        self.refused()
        self.assertEqual(red.read_bytes(), b'original refusal')

    def test_preexisting_first_red_never_contacts_provider(self):
        self.phase['first_red_exists'] = True
        self.refused()
        self.assertEqual(self.healths, 0)

    def test_unknown_authority_is_a_refusal_not_provider_permission(self):
        self.phase['snapshot_stable'] = False
        self.refused()
        self.assertEqual(self.healths, 0)

    def test_nonce_mismatch_does_not_contact_provider(self):
        self.path.write_text(json.dumps(dict(self.ready, nonce='foreign')))
        self.sha = hashlib.sha256(self.path.read_bytes()).hexdigest()
        self.refused()
        self.assertEqual(self.healths, 0)


if __name__ == '__main__': unittest.main()
