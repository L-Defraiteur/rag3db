"""Real comparison and actual driver snapshot/refusal block; no Rust processes."""
from pathlib import Path
import ast,importlib.util,json,tempfile,unittest
spec=importlib.util.spec_from_file_location('qualification',Path(__file__).with_name('qualify.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
SUMMARY='test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1s\n'
FIRST='test sample::one ... ok\ntest sample::two - should panic ... ok\ntest sample::skipped ... ignored, unavailable fixture\n'+SUMMARY
EXPECTED=m.summary(FIRST)
class JudgeNames(unittest.TestCase):
    def test_same_counts_with_different_names_are_refused(self):
        second=FIRST.replace('sample::two','sample::foreign')
        self.assertFalse(m.compare_judge_names(FIRST,second,EXPECTED)['qualified'])
    def test_reordered_names_and_different_ignored_reason_are_identical(self):
        second='test sample::skipped ... ignored, other reason\ntest sample::two - should panic ... ok\ntest sample::one ... ok\n'+SUMMARY
        self.assertTrue(m.compare_judge_names(FIRST,second,EXPECTED)['qualified'])
    def test_duplicate_success_names_are_refused_even_on_both_sides(self):
        duplicate=FIRST.replace('sample::two - should panic','sample::one')
        self.assertFalse(m.compare_judge_names(duplicate,duplicate,EXPECTED)['qualified'])
    def test_same_name_cannot_be_both_success_and_ignored(self):
        duplicate=FIRST.replace('sample::skipped','sample::one')
        self.assertFalse(m.compare_judge_names(duplicate,duplicate,EXPECTED)['qualified'])
    def test_missing_parsed_row_is_not_hidden_by_successful_summary(self):
        incomplete=FIRST.replace('test sample::one ... ok\n','')
        self.assertFalse(m.compare_judge_names(incomplete,incomplete,EXPECTED)['qualified'])
    def test_real_driver_archives_comparison_before_refusal(self):
        tree=ast.parse(Path(__file__).with_name('qualify.py').read_text());driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
        index=next(i for i,n in enumerate(driver.body) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='comparison' for t in n.targets))
        block=driver.body[index:index+3]
        with tempfile.TemporaryDirectory(prefix='judge-names-fixture-') as tmp:
            p=Path(tmp);report=dict(qualified=False,only_first=['sample::one'],only_second=['sample::foreign'])
            ns=dict(P=p,first=FIRST,second=FIRST,counts=EXPECTED,compare_judge_names=lambda *args:report,save=m.save,require=m.require)
            with self.assertRaises(RuntimeError):exec(compile(ast.Module(body=block,type_ignores=[]),'actual-driver-names','exec'),ns)
            self.assertEqual(json.loads((p/'judge-names-comparison.json').read_text()),report)
if __name__=='__main__':unittest.main(failfast=True)
