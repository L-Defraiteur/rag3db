"""Additional actual-boundary witnesses, prepared before correcting remaining R2.

No driver entry import, OS child, native ELF or provider. Virtual ELF bytes
model catalogue, actual copied executable and a source path restored afterwards.
"""
import hashlib
import importlib.util
from pathlib import Path
import re
import sys
import unittest

spec = importlib.util.spec_from_file_location('original_eight', Path(__file__).with_name('test_qualification_review.py'))
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)

GAMES = ['sans-feature', 'daemon', 'rag3db-native,daemon',
    'rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres',
    'rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm',
    'rag3db-native,code,daemon,openai-llm', 'rag3db-native,code', 'rag3db-native']


class ProvenanceWitnesses(unittest.TestCase):
    def run_results(self, executed, after=None, restored=None):
        original = b'catalogued ELF contents'
        path, copy = '/target/e2e-fixture', '/archive/executed-elf'
        sha = lambda raw: hashlib.sha256(raw).hexdigest()
        virtual_files = {path: restored if restored is not None else (after or executed), copy: executed}
        evidence = dict(catalogue=dict(path=path,sha256=sha(original)),
            execution=dict(source_path=path,executed_path=copy,before_sha256=sha(executed),
                after_sha256=sha(after if after is not None else executed),copy_sha256=sha(executed)))
        parser = base.actual('results', dict(re=re, INVENTORIES={'e2e_tty':[base.FILETS,base.TTY]},
            ELF_EXPECTED={'e2e_tty':path}, ELF_PROVENANCE={'e2e_tty':evidence},
            digest=lambda name: sha(virtual_files[str(name)])))
        raw = f'▸ binaire e2e : {path} (42 Mo)\n' + base.block('Running tests/e2e_filets.rs',base.FILETS) + base.block('Running tests/e2e_tty.rs',base.TTY)
        parser(raw,passed=4)

    def test_same_catalogued_and_executed_contents_are_a_positive_control(self):
        self.run_results(b'catalogued ELF contents')

    def test_same_path_and_names_cannot_replace_the_catalogued_elf(self):
        with self.assertRaises(AssertionError):
            self.run_results(b'other executable with identical test names')

    def test_restoring_source_path_cannot_erase_wrong_executed_contents(self):
        with self.assertRaises(AssertionError):
            self.run_results(b'other executable with identical test names',restored=b'catalogued ELF contents')

    def test_content_changed_after_execution_is_refused(self):
        with self.assertRaises(AssertionError):
            self.run_results(b'catalogued ELF contents',after=b'changed after execution')

    def test_exact_eight_feature_games_are_a_positive_control(self):
        parser=base.actual('feature_results',dict(re=re))
        parser('\n'.join(['VERT  '+game for game in GAMES]+['CHECK_FEATURES VERT 8/8']),GAMES)

    def test_extra_feature_game_cannot_hide_before_the_eight_of_eight_tail(self):
        parser=base.actual('feature_results',dict(re=re))
        with self.assertRaises(AssertionError):
            parser('\n'.join(['VERT  '+game for game in GAMES]+['VERT  extra-native-game','CHECK_FEATURES VERT 8/8']),GAMES)


if __name__=='__main__':
    suite=unittest.TestSuite([unittest.defaultTestLoader.loadTestsFromTestCase(base.ReviewWitnesses),
                             unittest.defaultTestLoader.loadTestsFromTestCase(ProvenanceWitnesses)])
    result=unittest.TextTestRunner(verbosity=2).run(suite)
    sys.exit(not result.wasSuccessful())
