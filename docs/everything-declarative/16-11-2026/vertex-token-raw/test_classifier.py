"""PURE immutable RAW and mutated strings; no executable, service or database."""
import copy
import json
import os
from pathlib import Path
import unittest
import classify_raw as c

ROOT = Path(os.environ.get('VERTEX_RAW_ARCHIVE', str(Path.home()/'.cache/rag3weaver-build/cx-preuves-ed-vertex-token-union-red-v1-local')))


class Classifier(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = {name: (ROOT/'client/official'/f'{name}.log').read_text() for name in c.LOG_NAMES}
        cls.inventory = json.loads(Path(__file__).with_name('observed-inventory.json').read_text())

    def reject(self, name, old, new):
        raw = dict(self.raw)
        self.assertIn(old, raw[name])
        raw[name] = raw[name].replace(old, new, 1)
        with self.assertRaises(ValueError): c.classify(raw, self.inventory)

    def test_real_complete_raw_two_e2e_phases_and_normal_nav_filter(self):
        value = c.classify(self.raw, self.inventory)
        self.assertEqual(value['phases']['lib']['summary'], [2612, 0, 72, 0, 0])
        self.assertEqual(value['phases']['navigateur']['summary'], [45, 0, 0, 0, 2641])
        for name, count, filtered in [('e2e_code', 36, 908), ('e2e_lectures', 27, 917)]:
            self.assertEqual(value['phases'][name+'/filets']['summary'], [2, 0, 0, 0, 942])
            self.assertEqual(value['phases'][name+'/suite']['summary'], [count, 0, 0, 0, filtered])

    def test_missing_duplicate_unknown_names_and_status_are_rejected(self):
        name = self.inventory['phases']['temoins'][0]['name']
        line = f'test {name} ... ok'
        for replacement in ['', line+'\n'+line, line.replace(name, name+'_alien'), line.replace('ok', 'FAILED')]:
            with self.subTest(replacement=replacement): self.reject('temoins', line, replacement)

    def test_ignored_and_should_panic_annotations_cannot_be_relabelled(self):
        name = self.inventory['phases']['temoins'][0]['name']
        self.reject('temoins', f'test {name} ... ok', f'test {name} ... ignored')
        self.reject('temoins', f'test {name} ... ok', f'test {name} - should panic ... ok')

    def test_filets_cannot_be_missing_duplicated_or_replaced_with_suite(self):
        marker = '     Running tests/e2e_filets.rs (binaire e2e unique, ses propres filets)'
        for replacement in ['', marker+'\n'+marker, marker.replace('e2e_filets', 'e2e_code')]:
            with self.subTest(replacement=replacement): self.reject('e2e_code', marker, replacement)
        self.reject('e2e_code', 'test chaque_suite_est_montee ... ok', 'test e2e_code::fake ... ok')

    def test_all_summary_fields_and_running_count_are_checked(self):
        old = 'test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 908 filtered out;'
        for replacement in [old.replace('36 passed', '35 passed'), old.replace('0 failed', '1 failed'), old.replace('0 ignored', '1 ignored'), old.replace('0 measured', '1 measured'), old.replace('908 filtered', '0 filtered'), old.replace('ok.', 'FAILED.')]:
            with self.subTest(replacement=replacement): self.reject('e2e_code', old, replacement)
        self.reject('e2e_code', 'running 36 tests', 'running 35 tests')

    def test_duplicate_summary_and_unknown_test_event_are_refused(self):
        old = 'test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 2677 filtered out; finished in 0.00s'
        self.reject('temoins', old, old+'\n'+old)
        self.reject('temoins', 'running 7 tests', 'running 7 tests\ntest forged ... perhaps')

    def test_lib_must_be_unfiltered_nav_has_its_own_inventory_and_total(self):
        self.reject('lib', '0 filtered out;', '1 filtered out;')
        self.reject('navigateur', '2641 filtered out;', '0 filtered out;')

    def test_inventory_types_duplicates_and_statuses_are_strict(self):
        for mutation in ['name_type', 'panic_type', 'status', 'duplicate', 'missing_phase']:
            inv = copy.deepcopy(self.inventory)
            row = inv['phases']['temoins'][0]
            if mutation == 'name_type': row['name'] = 123
            elif mutation == 'panic_type': row['should_panic'] = 0
            elif mutation == 'status': row['status'] = 'skipped'
            elif mutation == 'duplicate': inv['phases']['temoins'].append(dict(row))
            else: del inv['phases']['e2e_code/filets']
            with self.subTest(mutation=mutation), self.assertRaises(ValueError): c.classify(self.raw, inv)

    def test_feature_games_and_final_canonical_verdict_are_checked(self):
        self.reject('check_features', 'CHECK_FEATURES VERT 8/8', 'CHECK_FEATURES VERT 7/8')
        self.reject('check_features', 'VERT  sans-feature', 'VERT  daemon')
        self.reject('resume', 'suites jouées : e2e_lectures e2e_code', 'suites jouées : e2e_code')
        self.reject('resume', '\nVERT\n', '\nROUGE\n')

    def test_resume_must_match_each_phase_and_proof_marker(self):
        self.reject('resume', '27 passed;', '26 passed;')
        self.reject('resume', 'preuve : rag3weaver revérifié', 'preuve : absente')

    def test_elapsed_raw_and_long_running_diagnostics_are_not_test_results(self):
        raw = dict(self.raw)
        raw['navigateur'] = raw['navigateur'].replace('over 60 seconds', 'over 120 seconds')
        c.classify(raw, self.inventory)
        name = self.inventory['phases']['temoins'][0]['name']
        raw = dict(self.raw)
        raw['temoins'] += f'\ntest {name} ... ok\n'
        with self.assertRaises(ValueError): c.classify(raw, self.inventory)


if __name__ == '__main__': unittest.main(failfast=True)
