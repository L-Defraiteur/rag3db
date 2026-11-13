"""Execute real observe/run AST with virtual /proc and Popen only."""
from pathlib import Path
import ast,contextlib,io,json,re,tempfile,time,types,unittest
HERE=Path(__file__).parent
class Proc:
 def __init__(self,path):self.path=str(path)
 @property
 def name(self):return self.path.rsplit('/',1)[-1]
 def __str__(self):return self.path
 def __truediv__(self,part):return Proc(self.path+'/'+str(part))
 def iterdir(self):return [Proc('/proc/99')] if self.path=='/proc' else []
 def read_text(self):
  if self.path.endswith('/stat'):
   parts=['R','7',*['0']*17,'123'];return '99 (cargo) '+' '.join(parts)
  if self.path.endswith('/cgroup'):return '0::/owned\n'
  if self.path.endswith('/memory.max'):return '34359738368\n'
  raise AssertionError(self.path)

def fixture_path(path):
 return Proc(path) if str(path).startswith(('/proc','/sys/fs/cgroup')) else Path(path)

class ObserverRefusal(unittest.TestCase):
 def test_real_observe_refusal_is_fatal_and_never_launches_next_phase(self):
  tree=ast.parse((HERE/'driver.py').read_text());names={'observe','run','require','latch_first_red','first_red','first_lib_red'}
  functions=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names];self.assertEqual(len(functions),len(names))
  code=compile(ast.Module(body=functions,type_ignores=[]),'real-observe-run-fixture','exec')
  for error_type in [PermissionError,FileNotFoundError,ProcessLookupError]:
   with self.subTest(error_type=error_type),tempfile.TemporaryDirectory(prefix='cx-observe-refusal-fixture-') as tmp:
    popens=[];original=error_type('fixture capture refused')
    def refuse(*args,**kwargs):raise original
    def popen(*args,**kwargs):
     popens.append(args);return types.SimpleNamespace(poll=lambda:0,wait=lambda:0,returncode=0)
    ns=dict(Path=fixture_path,json=json,re=re,time=time,P=Path(tmp),C=Path('/fixture/crate'),T=Path('/fixture/target'),CG='/owned',W=Path('/fixture/source'),CONTROL=Path('/fixture/control'),Q=Path('/fixture/controller'),END=time.monotonic()+100,active_limit=time.monotonic()+100,owned={},seen={},maps={},protected={},devices={},phase='features',child=None,last_size=time.monotonic(),main_large=None,E={},phases=[],cargo_capture_counter=0,cargo_captures={},CARGO_EXE='/cargo',CARGO_SHA='abc',cargo_expected_for=lambda phase:{},cargo_probe=types.SimpleNamespace(observe=refuse),record_attempt=lambda *a:None,guard=lambda:None,memory=lambda:None,evidence=lambda:None,state=lambda *a:None,subprocess=types.SimpleNamespace(Popen=popen,check_output=lambda *a,**kw:'0 /fixture'),os=types.SimpleNamespace(getpid=lambda:7,getuid=lambda:42,readlink=lambda p:'/cargo'))
    exec(code,ns)
    with contextlib.redirect_stdout(io.StringIO()),self.assertRaises(RuntimeError) as caught:
     ns['run']('features',['fixture-current-command'],10)
     ns['run']('large',['fixture-next-command-MUST-NOT-LAUNCH'],10)
    self.assertIs(caught.exception.__cause__,original)
    self.assertEqual(len(popens),1,'only the current fixture invocation may exist')
    first=(Path(tmp)/'first-red.json').read_bytes();self.assertIn(b'coherent Cargo capture refused',first)
    ns['latch_first_red']('later failure must not overwrite')
    self.assertEqual((Path(tmp)/'first-red.json').read_bytes(),first)
if __name__=='__main__':unittest.main(failfast=True)
