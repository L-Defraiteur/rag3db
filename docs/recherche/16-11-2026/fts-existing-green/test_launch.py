"""Hermetic argv/scope witnesses: no real poste, Cargo or systemd process."""
import ast,json,os,subprocess,sys,tempfile,unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
class Launch(unittest.TestCase):
 def test_real_store_resolver_uses_explicit_store_under_private_xdg(self):
  repo=HERE.parents[3]
  script=repo/'extension/rag3weaver/scripts/lib_du_magasin.sh'
  with tempfile.TemporaryDirectory(prefix='fts-store-fixture-') as tmp:
   root=Path(tmp);store=root/'real-store';lib=store/'pinned-lib';(lib/'include/common').mkdir(parents=True)
   (lib/'librag3db.so').write_bytes(b'fixture-not-ELF');(lib/'include/common/build_id.h').write_bytes(b'fixture')
   command='source "$1"; python3() { return 0; }; lib_du_magasin pinned-lib'
   env=dict(os.environ,XDG_CACHE_HOME=str(root/'private-cache'));env.pop('RAG3DB_LIBS_DIR',None)
   refused=subprocess.run(['bash','-c',command,'resolver-fixture',str(script)],env=env,capture_output=True,text=True)
   self.assertNotEqual(refused.returncode,0)
   env['RAG3DB_LIBS_DIR']=str(store)
   accepted=subprocess.run(['bash','-c',command,'resolver-fixture',str(script)],env=env,capture_output=True,text=True)
   self.assertEqual(accepted.returncode,0,accepted.stderr);self.assertEqual(accepted.stdout.strip(),str(lib))

 def test_actual_launcher_calls_one_poste_then_common_supervisor(self):
  with tempfile.TemporaryDirectory(prefix='fts-launch-fixture-') as tmp:
   home=Path(tmp);poste=home/'.cache/rag3weaver-build/poste';poste.parent.mkdir(parents=True)
   poste.write_text('#!'+sys.executable+'\nimport json,os,sys\nopen(os.environ["FIXTURE_OUTPUT"],"w").write(json.dumps(dict(argv=sys.argv[1:],env={k:os.environ.get(k) for k in ["POSTE_MEM_MAX","POSTE_SWAP_MAX","POSTE_CPU_QUOTA","CARGO_BUILD_JOBS","RAG3WEAVER_VERROU_TENU","RAG3WEAVER_VALIDATION_RECU"]})))\n')
   poste.chmod(0o700);output=home/'argv.json'
   env=dict(os.environ,HOME=tmp,FIXTURE_OUTPUT=str(output))
   for key in ['RAG3WEAVER_VERROU_TENU','RAG3WEAVER_VALIDATION_RECU']:env.pop(key,None)
   subprocess.run(['bash',str(HERE/'launch.sh'),'/fixture/source','/fixture/proof','/fixture/control'],env=env,check=True,capture_output=True)
   row=json.loads(output.read_text())
   self.assertEqual(row['argv'],['lourd','python3','/fixture/control/confiner_validation.py','lancer','--delai','1200','--preuves','/fixture/proof-supervisor','--','python3','-B','/fixture/control/driver.py','/fixture/source','/fixture/proof'])
   self.assertEqual(row['env'],dict(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',POSTE_CPU_QUOTA=None,CARGO_BUILD_JOBS='8',RAG3WEAVER_VERROU_TENU=None,RAG3WEAVER_VALIDATION_RECU=None))
 def test_real_attestation_refuses_each_missing_scope_guarantee(self):
  tree=ast.parse((HERE/'driver.py').read_text());node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='attest_scope')
  ns={};exec(compile(ast.Module(body=[node],type_ignores=[]),'actual-scope-attestation','exec'),ns)
  class Scope:
   def __init__(self,values):self.values=values
   def __truediv__(self,key):
    class Field:
     def read_text(inner):return self.values[key]
    return Field()
  good={'memory.max':'34359738368','memory.high':'max','memory.swap.max':'0','cpu.max':'800000 100000'}
  self.assertEqual(ns['attest_scope'](Scope(good)),good)
  for key,bad in [('memory.max','42949672960'),('memory.high','1000000000'),('memory.swap.max','4294967296'),('cpu.max','max 100000'),('cpu.max','900000 100000')]:
   with self.subTest(key=key,bad=bad),self.assertRaises(AssertionError):ns['attest_scope'](Scope(dict(good,**{key:bad})))

class FirstRed(unittest.TestCase):
 def test_real_runner_stops_only_current_popen_on_failed_line(self):
  import re,time,types
  tree=ast.parse((HERE/'driver.py').read_text());functions=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in ['run','save']]
  children=[];chunks=iter([b'test catalog::tests::fixture ... FAILED\n',b''])
  class Child:
   pid=123
   stdout=types.SimpleNamespace(fileno=lambda:77)
   def __init__(self,*args,**kwargs):self.terminations=0;children.append(self)
   def terminate(self):self.terminations+=1
   def poll(self):return -15
   def wait(self,timeout):return -15
  class Selector:
   active=True
   def register(self,*args):pass
   def get_map(self):return {77:True} if self.active else {}
   def select(self,*args):return [(types.SimpleNamespace(fileobj=Child.stdout),None)]
   def unregister(self,*args):self.active=False
   def close(self):pass
  with tempfile.TemporaryDirectory(prefix='fts-first-red-fixture-') as tmp:
   ns=dict(P=Path(tmp),C=Path('/fixture'),END=time.monotonic()+100,time=time,json=json,re=re,selectors=types.SimpleNamespace(DefaultSelector=Selector,EVENT_READ=1),os=types.SimpleNamespace(read=lambda *args:next(chunks)),subprocess=types.SimpleNamespace(Popen=Child,PIPE=-1,STDOUT=-2))
   exec(compile(ast.Module(body=functions,type_ignores=[]),'actual-runner','exec'),ns)
   self.assertNotEqual(ns['run']('module',['fixture'],10),0)
   self.assertEqual(len(children),1);self.assertEqual(children[0].terminations,1)
   self.assertEqual(json.loads((Path(tmp)/'first-red.json').read_text())['child_pid'],123)
   self.assertEqual((Path(tmp)/'module.log').read_bytes(),b'test catalog::tests::fixture ... FAILED\n')

if __name__=='__main__':unittest.main()
