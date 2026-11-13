"""CPU-only composition witnesses: AST loads pure helpers, no subprocess/target."""
from pathlib import Path
import ast,hashlib,json,re,tempfile,time,tomllib,unittest
HERE=Path(__file__).parent

def helpers(*names,**values):
 tree=ast.parse((HERE/'driver.py').read_text());nodes=[n for n in tree.body if isinstance(n,(ast.FunctionDef,ast.AsyncFunctionDef)) and n.name in names]
 assert len(nodes)==len(names)
 env=dict(Path=Path,hashlib=hashlib,json=json,re=re,time=time,**values)
 exec(compile(ast.Module(body=nodes,type_ignores=[]),'isolated-driver-helpers','exec'),env)
 return env

class Composition(unittest.TestCase):
 def test_native_and_cpu_cargo_profiles_remain_separate(self):
  bare=dict(LD_LIBRARY_PATH='rust/lib',RAG3DB_LIB=None);sdk=dict(LD_LIBRARY_PATH='rust/lib:sdk',RAG3DB_LIB='sdk29')
  h=helpers('cargo_expected_for',METADATA_BEFORE_EXPORT=bare,CARGO_PROFILE=sdk)
  for phase in ['cache-reprise','cpu']:self.assertIs(h['cargo_expected_for'](phase),bare)
  for phase in ['features','large','navigateur','nav40']:self.assertIs(h['cargo_expected_for'](phase),sdk)
 def test_real_feature_closures_include_default_only_for_native(self):
  c=HERE.parents[3]/'extension/rag3weaver'
  h=helpers('feature_closure',C=c)
  f='rag3db-native,burn-embedder,burn-ocr,code,daemon,postgres,openai-llm,anthropic-llm'
  self.assertEqual(h['feature_closure']('',False),set())
  large=h['feature_closure'](f);nav=h['feature_closure'](f+',navigateur')
  self.assertEqual(len(large),13);self.assertEqual(len(nav),14);self.assertEqual(nav-large,{'navigateur'});self.assertIn('default',large)
 def test_controller_role_refuses_python_flags_and_pid_reuse(self):
  h=helpers('controller_role')
  role=dict(uid=1000,exe='/python',exe_sha256='abc',argv_tail=['/lancer.py','pilot'],cwd='/var/tmp')
  info=dict(pid=7,starttime='11',ppid=3,cgroup='/c',uid=1000,exe='/python',exe_sha256='abc',argv=['python','/lancer.py','pilot'],cwd='/var/tmp')
  ancestors={7:dict(starttime='11',ppid=3,cgroup='/c')}
  self.assertEqual(h['controller_role'](info,ancestors,{'launcher':role}),'launcher')
  self.assertIsNone(h['controller_role'](dict(info,argv=['python','-B','/lancer.py','pilot']),ancestors,{'launcher':role}))
  self.assertIsNone(h['controller_role'](dict(info,starttime='12'),ancestors,{'launcher':role}))
 def test_poste_fd_exception_never_allows_target_or_old_worktree(self):
  h=helpers('controller_fd_allowed','in_cache_context')
  roles={'poste':dict(script='/new/scripts/poste')}
  self.assertTrue(h['controller_fd_allowed']('poste','/new/scripts/poste',roles,[Path('/old'),Path('/target')]))
  for value in ['/target/fd','/old/file','/new/other']:
   self.assertFalse(h['controller_fd_allowed']('poste',value,roles,[Path('/old'),Path('/target')]))
 def test_first_red_is_immutable_and_expected_lib_panic_is_not_failed(self):
  with tempfile.TemporaryDirectory(prefix='cx-2b-union-cpu-fixture-') as tmp:
   h=helpers('latch_first_red','first_red','first_lib_red',P=Path(tmp),phase='cpu')
   h['latch_first_red']('first');h['latch_first_red']('second')
   self.assertEqual(json.loads((Path(tmp)/'first-red.json').read_text())['reason'],'first')
   self.assertTrue(h['first_red']('thread x panicked at x'))
   self.assertFalse(h['first_lib_red']('thread x panicked at x\ntest x - should panic ... ok'))
   self.assertTrue(h['first_lib_red']('test x ... FAILED'))
 def test_admission_entry_has_no_historical_deadline_and_one_receipt(self):
  s=(HERE/'admettre.py').read_text();tree=ast.parse(s)
  self.assertNotIn('919393',s);self.assertNotIn('914593',s)
  self.assertEqual(len([n for n in ast.walk(tree) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr=='execv']),1)
  self.assertIn("'--delai','4800'",s)

if __name__=='__main__':unittest.main(failfast=True)
