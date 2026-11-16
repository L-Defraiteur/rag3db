"""Real driver environment AST and exec function, with execve replaced only."""
from pathlib import Path
import ast,importlib.util,json,os,tempfile,unittest
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('qualification',Path(__file__).with_name('qualify.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)

class ExecutionProfile(unittest.TestCase):
    def fixture(self,mode):
        with tempfile.TemporaryDirectory(prefix='second-exec-fixture-') as tmp:
            root=Path(tmp);p=root/'proofs';q=root/'control';crate=root/'crate';p.mkdir();q.mkdir();crate.mkdir();binary=root/'unit-tests';binary.write_bytes(b'\x7fELFfixture')
            tree=ast.parse(Path(__file__).with_name('qualify.py').read_text());driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
            update=next(n.value for n in driver.body if isinstance(n,ast.Expr) and isinstance(n.value,ast.Call) and isinstance(n.value.func,ast.Attribute) and isinstance(n.value.func.value,ast.Name) and n.value.func.value.id=='env' and n.value.func.attr=='update' and any(k.arg=='CARGO_MANIFEST_DIR' for k in n.value.keywords))
            namespace={**m.__dict__,'env':{},'C':crate,'P':p,'R':root,'short':{'path':str(root/'short')}}
            eval(compile(ast.Expression(update),'real-driver-env-update','eval'),namespace)
            profile=namespace['env'];self.assertEqual(profile['CARGO_MANIFEST_DIR'],str(crate))
            auth=root/'authority';auth.write_text(json.dumps(dict(echeance=1000)));profile['RAG3WEAVER_VALIDATION_RECU']=str(auth)
            m.save(q,'configuration.json',dict(pilot='fixture-pilot'));m.save(p,'profile.json',profile);m.save(p,'ledger-before-second-lib.json',dict(binary=str(binary),copy=str(binary),sha256=m.sha(binary)))
            sdk=str(root.parent/'rag3db-libs'/m.PIN);environment=dict(profile,RAG3DB_SHARED='1',RAG3DB_LIBRARY_DIR=sdk,RAG3DB_INCLUDE_DIR=sdk,LD_LIBRARY_PATH=sdk)
            if mode=='missing':environment.pop('CARGO_MANIFEST_DIR')
            if mode=='foreign':environment['CARGO_MANIFEST_DIR']=str(root/'wrong-crate')
            calls=[]
            with patch.object(m,'P',p),patch.object(m,'Q',q),patch.object(m,'C',crate),patch.object(m,'R',root),patch.object(m,'guard'),patch.object(m.contract,'OLD_ELF_SHA',m.sha(binary)),patch.object(m.time,'monotonic',return_value=99),patch.dict(os.environ,environment,clear=True),patch.object(m.os,'execve',side_effect=lambda *args:calls.append(args)):
                if mode=='valid':m.exec_second()
                else:
                    with self.assertRaises(RuntimeError):m.exec_second()
            snapshot=json.loads((p/'sdk-exec-profile.json').read_text())
            self.assertTrue((p/'sdk-exec-profile.json').exists(),'snapshot must precede any rejection')
            if mode=='valid':
                self.assertEqual(snapshot['diff'],{});self.assertEqual(len(calls),1);self.assertEqual(calls[0][0],str(binary));self.assertEqual(calls[0][1],[str(binary)]);self.assertEqual(calls[0][2]['CARGO_MANIFEST_DIR'],str(crate))
            else:
                self.assertEqual(calls,[]);self.assertIn('CARGO_MANIFEST_DIR',snapshot['diff'])
    def test_actual_driver_env_reaches_real_exec_without_any_filter(self):self.fixture('valid')
    def test_missing_runtime_manifest_is_archived_then_refused_without_exec(self):self.fixture('missing')
    def test_foreign_runtime_manifest_is_archived_then_refused_without_exec(self):self.fixture('foreign')
    def test_real_command_exports_sdk_once_and_never_calls_cargo(self):
        command=m.runtime_command();self.assertEqual(command[:2],['bash','-c']);self.assertEqual(command[2].count('exporter_la_lib'),1)
        self.assertEqual(command[-1],'--exec-second');self.assertNotIn('cargo',command);self.assertNotIn('--lib',command);self.assertNotIn('--exact',command)
if __name__=='__main__':unittest.main(failfast=True)
