"""Hermetic argv/scope witnesses: no real poste, Cargo or systemd process."""
import ast,json,os,subprocess,sys,tempfile,unittest
from pathlib import Path
HERE=Path(__file__).resolve().parent
class Launch(unittest.TestCase):
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
   self.assertEqual(row['env'],dict(POSTE_MEM_MAX='32G',POSTE_SWAP_MAX='0',POSTE_CPU_QUOTA='800%',CARGO_BUILD_JOBS='8',RAG3WEAVER_VERROU_TENU=None,RAG3WEAVER_VALIDATION_RECU=None))
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
if __name__=='__main__':unittest.main()
