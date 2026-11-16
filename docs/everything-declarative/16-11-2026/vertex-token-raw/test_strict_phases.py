"""Three grouped review counterexamples on the real classifier and frozen RAW."""
import json
import os
from pathlib import Path
import re
import unittest
import classify_raw as c

ROOT = Path(os.environ.get('VERTEX_RAW_ARCHIVE', str(Path.home()/'.cache/rag3weaver-build/cx-preuves-ed-vertex-token-union-red-v1-local')))


class StrictPhases(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = {n:(ROOT/'client/official'/f'{n}.log').read_text() for n in c.LOG_NAMES}
        cls.inventory = json.loads(Path(__file__).with_name('observed-inventory.json').read_text())

    def test_all_running_headers_are_consumed_including_zero_and_malformed(self):
        for extra in ['running 0 tests', 'running malformed', 'running 7 tests']:
            for where in ['before', 'within', 'after']:
                raw = dict(self.raw)
                if where == 'before': raw['temoins'] = extra+'\n'+raw['temoins']
                elif where == 'within': raw['temoins'] = raw['temoins'].replace('running 7 tests', 'running 7 tests\n'+extra, 1)
                else: raw['temoins'] += '\n'+extra+'\n'
                with self.subTest(extra=extra, where=where), self.assertRaises(ValueError): c.classify(raw, self.inventory)

    def test_resume_summaries_and_markers_belong_to_their_actual_phase(self):
        raw = dict(self.raw)
        lines = raw['resume'].splitlines()
        summaries = [line for line in lines if line.startswith('test result:')]
        moved = [line for line in lines if not line.startswith('test result:')]
        at = moved.index('== témoins navigateur')+1
        moved[at:at] = summaries
        raw['resume'] = '\n'.join(moved)+'\n'
        with self.assertRaises(ValueError): c.classify(raw, self.inventory)
        for marker in ['CHECK_FEATURES VERT 8/8', 'preuve : rag3weaver revérifié']:
            raw = dict(self.raw)
            lines = raw['resume'].splitlines()
            lines.remove(marker)
            lines.insert(lines.index('== témoins navigateur')+1, marker)
            raw['resume'] = '\n'.join(lines)+'\n'
            with self.subTest(marker=marker), self.assertRaises(ValueError): c.classify(raw, self.inventory)

    def test_proof_requires_real_crate_path_and_expected_dev_profile(self):
        raw = dict(self.raw)
        finished = next(line for line in raw['preuve'].splitlines() if line.startswith('    Finished'))
        raw['preuve'] = finished+'\n'
        with self.assertRaises(ValueError): c.classify(raw, self.inventory)
        for old, new in [('Compiling rag3weaver ', 'Compiling another_crate '),
                         ('rag3db-cx-ed-vertex-token-union-r1', 'rag3db-another-worktree'),
                         ('[unoptimized + debuginfo]', '[optimized]')]:
            raw = dict(self.raw)
            self.assertIn(old, raw['preuve'])
            raw['preuve'] = raw['preuve'].replace(old, new, 1)
            with self.subTest(new=new), self.assertRaises(ValueError): c.classify(raw, self.inventory)


if __name__ == '__main__': unittest.main(failfast=True)
