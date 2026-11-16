"""Actual writer controller with synthetic Popen/proc only, no process or signal."""
from pathlib import Path as RealPath
import copy,json,tempfile,time,unittest
from unittest.mock import patch
import lifecycle as c

class Proc:
 def __init__(self,key):self.key=key
 def __truediv__(self,key):return Proc(self.key+'/'+str(key))
 def read_text(self):
  if self.key.endswith('/stat'):return '123 (fake) '+' '.join(['R']+['0']*18+['42'])
  if self.key.endswith('/maps'):return str(c.q.LIB/'librag3db.so')+'\n'+str(c.q.LIB/'libvector.rag3db_extension')+'\n'
  if self.key.endswith('/cgroup'):return '0::/fake\n'
  raise AssertionError(self.key)
 def read_bytes(self):return b'HOME=/fixture\0'

class WriterController(unittest.TestCase):
 def run_fake(self,failure=None):
  fixture=json.loads(RealPath(__file__).with_name('writer-meta-response.json').read_bytes());trace=[];children=[];nonce='request-nonce'
  with tempfile.TemporaryDirectory() as d:
   root=RealPath(d);p=root/'P';p.mkdir();q=root/'Q';q.mkdir()
   for name in ['lifecycle.json','lifecycle.mmd']:(q/name).write_text('{}')
   class Child:
    pid=123
    def __init__(self,args,**kwargs):self.out=kwargs['stdout'];self.stdin=self;self.closed=False;children.append(self)
    def write(self,raw):
     request=json.loads(raw);trace.append(request['op'])
     if request['op']=='call':
      response=copy.deepcopy(fixture);response['result']['result']['query']=request['arguments']['nonce'];self.out.write(json.dumps(response).encode()+b'\n');self.out.flush()
     elif request['op']=='shutdown':
      if not (p/'response-publication.json').exists():raise AssertionError('shutdown before persisted publication')
      trace.append('publication-already-present');self.out.write(b'{"ok":true,"result":{"closed":true}}\n');self.out.flush();self.closed=True
    def flush(self):pass
    def close(self):pass
    def poll(self):return 0 if self.closed else None
    def wait(self):return 0
   original_sha=c.q.sha;original_save=c.q.save
   def sha(path):return c.q.PINS[c.q.CLI] if isinstance(path,Proc) else original_sha(path)
   def save(path,value,*args,**kwargs):
    if failure and path.name=='response-publication.json':raise failure
    return original_save(path,value,*args,**kwargs)
   identity=dict(pid=123,start='42',state='R',exe='/fixture/cli',exe_sha256=c.q.PINS[c.q.CLI])
   with patch.object(c,'P',p),patch.object(c,'Q',q),patch.object(c,'Path',lambda value:Proc('/proc') if str(value)=='/proc' else RealPath(value)),patch.object(c.subprocess,'Popen',Child),patch.object(c.q,'process_identity',return_value=identity),patch.object(c.q,'sha',sha),patch.object(c.q,'save',save),patch.object(c.os,'readlink',return_value='/fixture/cli'),patch.object(c.time,'sleep',lambda duration:None):
    if failure:
     with self.assertRaises(type(failure)) as caught:c.writer(dict(active_cgroup='/fake'),{},dict(HOME='/fixture'),dict(valid_until_monotonic=time.monotonic()+300),time.monotonic()+900,nonce)
     self.assertIs(caught.exception,failure)
    else:
     result=c.writer(dict(active_cgroup='/fake'),{},dict(HOME='/fixture'),dict(valid_until_monotonic=time.monotonic()+300),time.monotonic()+900,nonce);self.assertEqual(result['rc'],0)
     self.assertTrue(json.loads((p/'writer-closed.json').read_text())['shutdown_acknowledged'])
   self.assertEqual(len(children),1)
   return trace
 def test_actual_controller_publishes_before_shutdown_and_waits_same_child(self):
  self.assertEqual(self.run_fake(),['call','shutdown','publication-already-present'])
 def test_publication_errors_escape_and_never_send_shutdown_or_spawn_again(self):
  for cls in [OSError,PermissionError,ProcessLookupError]:
   with self.subTest(error=cls.__name__):self.assertEqual(self.run_fake(cls('publication fixture refused')),['call'])

if __name__=='__main__':unittest.main(failfast=True)
