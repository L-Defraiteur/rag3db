"""Execute actual controller entrypoints with entirely fake proc/GDB/filesystem."""
import ast,hashlib,io,json,os,sys,types,unittest
from pathlib import Path
from unittest.mock import patch
import qualify as q
import sdk_contract
HERE=Path(__file__).parent
SOURCE=Path(os.environ.get('FTS_DOUBLE_CONTRACT_SOURCE',HERE))
class Entrypoints(unittest.TestCase):
 def test_cache_quiet_rejects_attribution_observed_before_permission(self):
  source=ast.parse((SOURCE/'qualify.py').read_text())
  functions=[node for node in source.body if isinstance(node,ast.FunctionDef) and node.name in ['cache_quiet','collect_process_paths']]
  class FakePath:
   def __init__(self,path):self.path=str(path)
   def __str__(self):return self.path
   def __truediv__(self,item):return FakePath(self.path+'/'+str(item))
   @property
   def name(self):return self.path.rsplit('/',1)[-1]
   def read_text(self):return '123 (fixture) '+' '.join(['S']+['0']*18+['17'])
   def iterdir(self):return iter([FakePath('/proc/123')]) if self.path=='/proc' else iter([self/'fd1',self/'fd2'])
  for stop in ['cwd','fd2']:
   def readlink(path):
    if path.name==stop:raise PermissionError('synthetic permission')
    return '/target/'+path.name if path.name in ['exe','fd1'] else '/outside'
   ns=dict(q.__dict__,Path=FakePath,OLD=FakePath('/old'),T=FakePath('/target'),Q=FakePath('/control'))
   exec(compile(ast.Module(body=functions,type_ignores=[]),'<actual-cache-quiet>','exec'),ns)
   with self.subTest(stop=stop),patch.object(q.os,'getpid',return_value=1),patch.object(q.os,'readlink',side_effect=readlink),self.assertRaises(AssertionError):ns['cache_quiet']({'source_root':'/source'})
 def test_actual_gdb_entrypoint_refuses_wrong_sdk_before_continue(self):
  code=(SOURCE/'main-profile.py').read_text();digest=lambda raw:hashlib.sha256(raw).hexdigest()
  for mapped in ['/sdk59/librag3db.so','/other/librag3db.so']:
   commands=[];writes={};expected={'RAG3DB_LIBRARY_DIR':'/sdk59'}
   ledger={'elf_sha256':digest(b'elf'),'sdk':{'librag3db.so':digest(b'sdk')}}
   class Quit(Exception):pass
   class FakePath:
    def __init__(self,path):self.path=str(path)
    @classmethod
    def home(cls):return cls('/home/fake')
    def __str__(self):return self.path
    def __truediv__(self,item):return FakePath(self.path+'/'+str(item))
    def read_text(self):
     if self.path.endswith('/profile.json'):return json.dumps({'expected':expected})
     if self.path.endswith('/ledger.json'):return json.dumps(ledger)
     if self.path.endswith('/stat'):return '42 (inferior) '+' '.join(['S']+['0']*18+['17'])
     if self.path.endswith('/maps'):return '100-200 r-xp 0 00:01 42 '+mapped+'\n'
     raise AssertionError(self.path)
    def read_bytes(self):
     if self.path.endswith('/environ'):return b'RAG3DB_LIBRARY_DIR=/sdk59\0'
     if self.path.endswith('/exe'):return b'elf'
     if self.path.endswith('/librag3db.so'):return b'sdk'
     raise AssertionError(self.path)
    def open(self,mode):
     self_path=self.path
     class Stream(io.StringIO):
      def close(self):writes[self_path]=self.getvalue();super().close()
     return Stream()
   def execute(command):
    commands.append(command)
    if command.startswith('quit'):raise Quit()
   fake=types.SimpleNamespace(selected_inferior=lambda:types.SimpleNamespace(pid=42),execute=execute,write=lambda *a:None,STDERR=1,GdbError=RuntimeError)
   before=list(sys.path);original_defaults=sdk_contract.validate_sdk_maps.__defaults__
   try:
    sdk_contract.validate_sdk_maps.__defaults__=(lambda p:p,)
    with patch.dict(sys.modules,{'gdb':fake}),patch('pathlib.Path',FakePath),patch.object(os,'readlink',return_value='/unit-tests'):
     try:exec(compile(code,'<actual-main-profile>','exec'),{});execute('continue')
     except Quit:pass
   finally:sys.path[:]=before;sdk_contract.validate_sdk_maps.__defaults__=original_defaults
   with self.subTest(mapped=mapped):
    if mapped.startswith('/other'):
     self.assertNotIn('continue',commands);self.assertEqual(commands[-2:],['kill','quit 1'])
    else:self.assertEqual(commands,['continue'])
if __name__=='__main__':unittest.main(failfast=True)
