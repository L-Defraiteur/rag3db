"""Pure witnesses; only private temporary files, no Cargo, signals or /proc."""
import importlib.util,json,tempfile,unittest,types
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
        self.assertIsNone(m.failure_match("thread 'normal_should_panic' panicked at fixture\ntest normal_should_panic ... ok\n"))
    def test_real_failure_is_first_red(self):
        for text in ['test fixture ... FAILED\n','FAILED\n','test result: FAILED. 0 passed; 1 failed;']:
            with self.subTest(text=text):self.assertIsNotNone(m.failure_match(text))
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
            m.save(v,'enfants.json',[]);m.save(p,'short-tmp.json',{'path':str(root)});m.save(p,'driver-result.json',dict(status='GREEN'))
            binary=root/'elf';binary.write_bytes(b'\x7fELFfixture');m.save(p,'ledger-before-second-lib.json',dict(binary=str(binary),copy=str(binary),sha256=m.sha(binary)))
            observed=dict(found=[],unknown=[],owned_alive=[],owned_unknown=[])
            with patch.object(m.contract,'check_same_short'),patch.object(m,'P',p),patch.object(m,'S',s),patch.object(m,'guard'),patch.object(m,'scan',return_value=observed),patch.object(m.time,'monotonic',return_value=101):
                self.assertEqual(m.finalize({},0),1)
            result=json.loads((p/'composed-result.json').read_text());self.assertEqual(result['status'],'RED');self.assertIn('allocation deadline reached',result['error'])
    def test_real_finalize_refuses_publication_when_open_crosses_deadline(self):
        with tempfile.TemporaryDirectory(prefix='cx-late-publication-') as tmp:
            root=Path(tmp);p=root/'proofs';p.mkdir();s=root/'scope';v=s/'validation-test';v.mkdir(parents=True)
            m.save(v,'verdict.json',dict(verdict='VERT',code=0,restes=[],nettoye=True,prive=str(root/'absent-bus')))
            m.save(v,'autorite.json',dict(etat='termine',echeance=100));m.save(v,'enfants.json',[])
            m.save(p,'driver-result.json',dict(status='GREEN'));m.save(p,'short-tmp.json',dict(path=str(root)))
            binary=root/'elf';binary.write_bytes(b'\x7fELFfixture');m.save(p,'ledger-before-second-lib.json',dict(binary=str(binary),copy=str(binary),sha256=m.sha(binary)))
            clock=[99.];original_open=Path.open;composed=p/'composed-result.json'
            def delayed_open(path,*args,**kwargs):
                stream=original_open(path,*args,**kwargs)
                if path==composed:clock[0]=101.
                return stream
            observed=dict(found=[],unknown=[],owned_alive=[],owned_unknown=[])
            with patch.object(m,'P',p),patch.object(m,'S',s),patch.object(m,'guard'),patch.object(m,'scan',return_value=observed),patch.object(m.contract,'check_same_short'),patch.object(m.time,'monotonic',side_effect=lambda:clock[0]),patch.object(Path,'open',delayed_open):
                rc=m.finalize({'pilot':'fixture-pilot'},0)
            self.assertEqual(rc,1,'an open delayed past END must refuse GREEN')
            result=json.loads(composed.read_text());self.assertEqual(result['status'],'RED');self.assertIn('deadline',result['error'])
    def exercise_real_run(self,historical,active):
        with tempfile.TemporaryDirectory(prefix='cx-watcher-test-') as tmp:
            root=Path(tmp);proofs=root/'proofs';proofs.mkdir();cache=proofs/'cache/builtin';cache.mkdir(parents=True)
            (cache/'llm-gemini.log').write_text(historical)
            calls=[]
            def popen(args,**kwargs):
                calls.append(args);kwargs['stdout'].write(active);kwargs['stdout'].flush()
                return types.SimpleNamespace(poll=lambda:0,wait=lambda:0)
            with patch.object(m,'P',proofs),patch.object(m,'T',root/'absent-target'),patch.object(m,'C',root),patch.object(m.subprocess,'Popen',side_effect=popen):
                try:
                    result=m.run('official-pass',['current-command'],{},m.time.monotonic()+100,60)
                    m.run('second-lib',['next-command'],{},m.time.monotonic()+100,60)
                    return result,len(calls)
                except (RuntimeError,AssertionError):
                    first=(proofs/'first-red.json').read_bytes();m.first_red('later failure');self.assertEqual((proofs/'first-red.json').read_bytes(),first)
                    self.assertEqual(len(calls),1,'failure must stop before next invocation')
                    raise
    def test_real_run_ignores_embedded_historical_failed_log(self):
        result,calls=self.exercise_real_run('test historical ... FAILED\n','current output\n')
        self.assertEqual(result,'current output\n');self.assertEqual(calls,2)
    def test_real_run_detects_active_failed_log_before_next_invocation(self):
        with self.assertRaises((RuntimeError,AssertionError)):self.exercise_real_run('','test current ... FAILED\n')
    def test_real_run_preserves_active_expected_should_panic(self):
        _,calls=self.exercise_real_run('',"thread 'fixture' panicked at expected\ntest fixture ... ok\n")
        self.assertEqual(calls,2)
if __name__=='__main__':unittest.main(failfast=True)
