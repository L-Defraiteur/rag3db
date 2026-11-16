"""Pure witnesses; only private temporary files, no Cargo, signals or /proc."""
import importlib.util,json,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('fixture_qualification',Path(__file__).with_name('qualify.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class Qualification(unittest.TestCase):
    def test_full_library_summary_preserves_ignored_as_unplayed(self):
        self.assertEqual(m.summary('test result: ok. 2595 passed; 0 failed; 72 ignored; 0 measured; 0 filtered out; finished in 1s\n'),dict(passed=2595,failed=0,ignored=72,filtered=0))
    def test_zero_failed_or_duplicate_summary_never_qualifies(self):
        for text in ['test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;', 'test result: FAILED. 1 passed; 1 failed; 0 ignored;', 'test result: ok. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out;']:
            with self.subTest(text=text),self.assertRaises(RuntimeError):m.summary(text)
        row='test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;\n'
        with self.assertRaises(RuntimeError):m.summary(row+row)
    def test_expected_panic_is_not_first_red(self):
        self.assertIsNone(m.first_red("thread 'normal_should_panic' panicked at fixture\ntest normal_should_panic ... ok\n"))
    def test_real_failure_is_first_red(self):
        for text in ['test fixture ... FAILED\n','FAILED\n','test result: FAILED. 0 passed; 1 failed;']:
            with self.subTest(text=text):self.assertIsNotNone(m.first_red(text))
    def test_large_elf_exact_path_and_magic(self):
        with tempfile.TemporaryDirectory(prefix='cx-qualification-test-') as tmp:
            root=Path(tmp);binary=root/'large-tests';binary.write_bytes(b'\x7fELFfixture')
            self.assertEqual(m.large_elf(f'Running unittests src/lib.rs ({binary})\n',root),binary)
            with self.assertRaises(RuntimeError):m.large_elf(f'Running unittests src/lib.rs ({binary})\n',root/'other')
            link=root/'alias';link.symlink_to(binary)
            with self.assertRaises(RuntimeError):m.large_elf(f'Running unittests src/lib.rs ({link})\n',root)
    def test_expired_deadline_refuses_equality(self):
        m.deadline(10,9.999)
        for now in [10,10.001]:
            with self.subTest(now=now),self.assertRaises(RuntimeError):m.deadline(10,now)
    def test_first_red_evidence_is_immutable(self):
        with tempfile.TemporaryDirectory(prefix='cx-qualification-test-') as tmp:
            root=Path(tmp);m.save(root,'first-red.json',{'error':'first'});original=(root/'first-red.json').read_bytes()
            with self.assertRaises(FileExistsError):m.save(root,'first-red.json',{'error':'later'})
            self.assertEqual((root/'first-red.json').read_bytes(),original)
    def test_final_deadline_after_scan_refuses_green(self):
        with tempfile.TemporaryDirectory(prefix='cx-qualification-test-') as tmp:
            root=Path(tmp);p=root/'proofs';p.mkdir();s=root/'scope';v=s/'validation-test';v.mkdir(parents=True)
            m.save(v,'verdict.json',dict(verdict='VERT',code=0,restes=[],nettoye=True,prive=str(root/'absent-bus')))
            m.save(v,'autorite.json',dict(etat='termine',echeance=100))
            m.save(v,'enfants.json',[]);m.save(p,'driver-result.json',dict(status='GREEN'))
            binary=root/'elf';binary.write_bytes(b'\x7fELFfixture');m.save(p,'ledger-before-second-lib.json',dict(binary=str(binary),copy=str(binary),sha256=m.sha(binary)))
            observed=dict(found=[],unknown=[],owned_alive=[],owned_unknown=[])
            with patch.object(m,'P',p),patch.object(m,'S',s),patch.object(m,'guard'),patch.object(m,'scan',return_value=observed),patch.object(m.time,'monotonic',return_value=101):
                self.assertEqual(m.finalize({},0),1)
            self.assertEqual(json.loads((p/'composed-result.json').read_text())['status'],'RED')
if __name__=='__main__':unittest.main(failfast=True)
