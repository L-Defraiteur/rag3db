"""Témoins préparés avant sonde, jamais de vrai proc/verrou/processus."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch
import test_poste_authority as poste_fixtures
import poste_lock_diagnostic as diagnostic


class LockDiagnosticFixtures(unittest.TestCase):
    def setUp(self):
        poste_fixtures.PosteAuthorityFixtures.setUp(self)
        self.out=self.base/'capture';self.out.mkdir()
    def snapshot(self):
        return diagnostic.collect_snapshot(self.common,self.table,str(self.root),self.proc)
    def test_snapshot_keeps_stat_fstat_raw_identity_and_clock(self):
        capture,raw=self.snapshot()
        self.assertEqual(raw,self.lockline.encode())
        for name in ['root','fiche','lock','poste_fd9']:
            self.assertEqual(capture[name]['stat'],capture[name]['fstat'])
        self.assertEqual(capture['poste']['pid'],4242)
        self.assertGreaterEqual(capture['ended_monotonic'],capture['started_monotonic'])
    def test_mismatch_is_published_before_comparison_and_preserved(self):
        capture,raw=self.snapshot();capture['lock']['stat']['inode']+=1
        with self.assertRaises(ValueError):diagnostic.publish_and_judge(self.out,capture,raw,deadline=1e30)
        saved=json.loads((self.out/'capture.json').read_text())
        self.assertEqual(saved['lock']['stat']['inode'],capture['lock']['stat']['inode'])
        self.assertEqual((self.out/'fdinfo.raw').read_bytes(),raw)
        self.assertTrue((self.out/'first-red.json').exists())
    def test_oversized_raw_keeps_prefix_and_refuses(self):
        (self.proc/'4242/fdinfo/9').write_bytes(b'x'*(diagnostic.MAX_FDINFO+1))
        capture,raw=self.snapshot()
        self.assertEqual(len(raw),diagnostic.MAX_FDINFO)
        self.assertTrue(capture['fdinfo_truncated'])
        with self.assertRaises(ValueError):diagnostic.publish_and_judge(self.out,capture,raw,deadline=1e30)
    def test_missing_kernel_lock_cannot_be_green(self):
        (self.proc/'4242/fdinfo/9').write_text('pos: 0\n')
        capture,raw=self.snapshot()
        with self.assertRaises(ValueError):diagnostic.publish_and_judge(self.out,capture,raw,deadline=1e30)
    def test_refusal_has_no_second_observation_or_comparison(self):
        capture,raw=self.snapshot()
        def refuse(value,body):
            self.assertTrue((self.out/'capture.json').exists())
            self.assertTrue((self.out/'fdinfo.raw').exists())
            raise ValueError('FIRST-CAUSE-FIXTURE')
        with patch.object(diagnostic,'compare_snapshot',side_effect=refuse) as compare:
            with self.assertRaisesRegex(ValueError,'FIRST-CAUSE-FIXTURE'):
                diagnostic.publish_and_judge(self.out,capture,raw,deadline=1e30)
            compare.assert_called_once()
        self.assertIn('FIRST-CAUSE-FIXTURE',(self.out/'first-red.json').read_text())
    def test_late_publication_refuses_without_deleting_capture(self):
        capture,raw=self.snapshot()
        with self.assertRaises(ValueError):diagnostic.publish_and_judge(self.out,capture,raw,deadline=0.)
        self.assertTrue((self.out/'capture.json').exists())
        self.assertTrue((self.out/'first-red.json').exists())


if __name__=='__main__':unittest.main(verbosity=2)
