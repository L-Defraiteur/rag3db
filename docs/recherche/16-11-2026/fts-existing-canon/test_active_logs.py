"""Run the actual watcher AST; no subprocess, Cargo or real /proc."""
import ast,hashlib,json,os,re,tempfile,time,unittest
from pathlib import Path
class ActiveLogs(unittest.TestCase):
    def test_only_current_invocation_outputs_can_stop_the_pass(self):
        tree=ast.parse(Path(__file__).with_name('driver.py').read_text())
        names={'active_logs','watch_logs','first_red'}
        body=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names]
        failed=next(n for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='FAILED' for t in n.targets))
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp); ns=dict(P=p,re=re,time=time,os=os,json=json,hashlib=hashlib)
            exec(compile(ast.Module(body=[failed,*body],type_ignores=[]),'real-watcher','exec'),ns)
            old=p/'cache/builtin-templates/resultats';old.mkdir(parents=True)
            (old/'llm.log').write_text('test result: FAILED. 0 passed; 1 failed;\n')
            (p/'official').mkdir(); lib=p/'official/lib.log'
            lib.write_text('a literal FAILED in a value\nthread expected panicked at deliberate panic\n')
            ns['watch_logs']('official-pass')
            self.assertFalse((p/'first-red.json').exists())
            lib.write_text('test foo::bar ... FAILED\n')
            with self.assertRaises(AssertionError):ns['watch_logs']('official-pass')
            record=json.loads((p/'watcher-first-match.json').read_text())
            self.assertEqual(record['path'],str(lib));self.assertEqual(record['matched'],'test foo::bar ... FAILED')
            self.assertEqual(record['sha256'],hashlib.sha256(lib.read_bytes()).hexdigest())
            first=(p/'first-red.json').read_bytes();ns['first_red']('later')
            self.assertEqual(first,(p/'first-red.json').read_bytes())


    def test_added_code_sync_and_tx_outputs_are_active(self):
        tree=ast.parse(Path(__file__).with_name('driver.py').read_text())
        names={'active_logs','watch_logs','first_red'}
        body=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names]
        failed=next(n for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='FAILED' for t in n.targets))
        for suite in ['e2e_code_sync','e2e_tx']:
            with self.subTest(suite=suite), tempfile.TemporaryDirectory() as tmp:
                p=Path(tmp);(p/'official').mkdir();log=p/'official'/(suite+'.log');log.write_text('test fixture ... FAILED\n')
                ns=dict(P=p,re=re,time=time,os=os,json=json,hashlib=hashlib)
                exec(compile(ast.Module(body=[failed,*body],type_ignores=[]),'actual-active-suites','exec'),ns)
                with self.assertRaises(AssertionError):ns['watch_logs']('official-pass')
                self.assertEqual(json.loads((p/'watcher-first-match.json').read_text())['path'],str(log))
if __name__=='__main__':unittest.main(failfast=True)
