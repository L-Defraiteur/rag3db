"""Actual nested command(), exercised with fake subprocess results; no native program."""
from pathlib import Path
import ast,importlib.util,json,subprocess,tempfile,time,unittest
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('qualification',Path(__file__).with_name('qualify.py'));q=importlib.util.module_from_spec(spec);spec.loader.exec_module(q)
def actual_command(root):
 tree=ast.parse(Path(__file__).with_name('qualify.py').read_text());driver=next(x for x in tree.body if isinstance(x,ast.FunctionDef) and x.name=='driver');fn=next(x for x in driver.body if isinstance(x,ast.FunctionDef) and x.name=='command')
 ns=dict(vars(q));ns.update(subprocess=subprocess,require_time=lambda *x:None,end=time.monotonic()+100,env={},P=root,save=q.save,time=time,cfg={'compiler':'/fake/c++'},binary=root/'read-typed')
 exec(compile(ast.fix_missing_locations(ast.Module(body=[fn],type_ignores=[])),'actual-command-AST','exec'),ns)
 return ns['command']
class PreflightLogs(unittest.TestCase):
 def test_nonzero_compile_keeps_raw_output_and_return_code_before_refusal(self):
  with tempfile.TemporaryDirectory(dir=Path.home()/'.cache/rag3weaver-build') as d:
   root=Path(d);run=actual_command(root);result=subprocess.CompletedProcess(['/fake/c++'],1,b'compiler output\n',b'ABI refused\n')
   with patch.object(subprocess,'run',return_value=result),self.assertRaises(Exception):run(['/fake/c++','unit.cpp'],90)
   self.assertEqual((root/'compile.stdout').read_bytes(),result.stdout);self.assertEqual((root/'compile.stderr').read_bytes(),result.stderr);self.assertEqual(json.loads((root/'compile-process.json').read_text())['rc'],1)
 def test_timeout_keeps_partial_output_and_timeout_status(self):
  with tempfile.TemporaryDirectory(dir=Path.home()/'.cache/rag3weaver-build') as d:
   root=Path(d);run=actual_command(root);error=subprocess.TimeoutExpired(['cp'],1,output=b'partial\n',stderr=b'copy stalled\n')
   with patch.object(subprocess,'run',side_effect=error),self.assertRaises(Exception):run(['cp','source','copy'],120)
   self.assertEqual((root/'copy.stdout').read_bytes(),error.stdout);self.assertEqual((root/'copy.stderr').read_bytes(),error.stderr);record=json.loads((root/'copy-process.json').read_text());self.assertTrue(record['timeout']);self.assertIsNone(record['rc'])
 def test_successful_native_golden_has_raw_logs_and_status(self):
  with tempfile.TemporaryDirectory(dir=Path.home()/'.cache/rag3weaver-build') as d:
   root=Path(d);run=actual_command(root);result=subprocess.CompletedProcess(['selftest'],0,b'golden\0\xff',b'')
   with patch.object(subprocess,'run',return_value=result):self.assertIs(run([str(root/'read-typed'),'--self-test'],15),result)
   self.assertEqual((root/'typed-selftest.stdout').read_bytes(),result.stdout);self.assertEqual((root/'typed-selftest.stderr').read_bytes(),b'');self.assertEqual(json.loads((root/'typed-selftest-process.json').read_text())['rc'],0)
if __name__=='__main__':unittest.main(failfast=True)
