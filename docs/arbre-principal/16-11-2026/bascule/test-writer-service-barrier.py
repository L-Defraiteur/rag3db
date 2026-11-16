"""Exercise the actual nested run barrier with fake management stdout and Popen; no native process."""
from pathlib import Path
import ast,copy,importlib.util,json,tempfile,types,unittest
FOLDER=Path(__file__).parent
def load(name,file):
 spec=importlib.util.spec_from_file_location(name,FOLDER/file);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
m=load('writer','migrate-closed-base.py');g=load('guard','cold-service-guard.py');fixtures=load('guard_fixtures','test-cold-service-guard.py')
TREE=ast.parse((FOLDER/'migrate-closed-base.py').read_text());MAIN=next(n for n in TREE.body if isinstance(n,ast.FunctionDef) and n.name=='main');RUN=next(n for n in MAIN.body if isinstance(n,ast.FunctionDef) and n.name=='run')
class ActualWriterBarrier(unittest.TestCase):
 def run_fixture(self,raw_override=None):
  with tempfile.TemporaryDirectory() as directory:
   root=Path(directory);launched=[];request,reply,raw=fixtures.fixture()
   if raw_override is not None:raw['units_service']=raw_override
   def validate():return g.validate_reply(request,reply,'c'*64,13.,raw)
   def fetch(*args):
    validate();return dict(request=request,reply=reply,request_sha256='c'*64,response_sha256='d'*64,reply_path=str(root/'reply.json'),raw_units=raw)
   class FakePopen:
    def __init__(self,*args,**kwargs):launched.append(args);self.pid=999
    def poll(self):return 0
    def wait(self):return 0
    def kill(self):raise AssertionError('finished fake must not be killed')
   def save(name,value):(root/name).write_text(json.dumps(value))
   ns=dict(vars(m));ns.update(args=types.SimpleNamespace(target_kind='rehearsal'),root=root,base=root/'target.rag3db',canonical=root/'canonical.rag3db',source=root,expected=[],inventory_live=lambda *a:[],fs=types.SimpleNamespace(file_inventory=lambda *a:[]),guard=types.SimpleNamespace(fetch_phase_guard=fetch,final_phase_check=lambda *a:validate(),validate_reply=g.validate_reply),common=object(),start=10.,end=1000.,env={},save=save,subprocess=types.SimpleNamespace(Popen=FakePopen),time=types.SimpleNamespace(monotonic=lambda:13.,sleep=lambda *a:None))
   factory=ast.FunctionDef(name='factory',args=ast.arguments(posonlyargs=[],args=[],kwonlyargs=[],kw_defaults=[],defaults=[]),body=[ast.Assign(targets=[ast.Name(id='writer_started',ctx=ast.Store())],value=ast.Constant(False)),copy.deepcopy(RUN),ast.Return(value=ast.Name(id='run',ctx=ast.Load()))],decorator_list=[])
   exec(compile(ast.fix_missing_locations(ast.Module(body=[factory],type_ignores=[])),'actual-writer-run','exec'),ns)
   run=ns['factory']()
   with self.assertRaises(ValueError) as error:run(['/fixed/backend','migration','versions'],'versions',900,True)
   return len(launched),str(error.exception)
 def test_inactive_json_cannot_cross_writer_on_empty_real_stdout(self):
  launched,error=self.run_fixture('');self.assertEqual(launched,0);self.assertIn('unit not loaded',error)
 def test_nonzero_main_pid_stops_actual_writer_callsite(self):
  launched,error=self.run_fixture(fixtures.RAW_SERVICE.replace('MainPID=0','MainPID=4'));self.assertEqual(launched,0)
 def test_valid_guard_reaches_exactly_one_fake_process_then_runtime_gate(self):
  launched,error=self.run_fixture();self.assertEqual(launched,1);self.assertIn('native runtime not observed',error)
if __name__=='__main__':unittest.main(failfast=True)
