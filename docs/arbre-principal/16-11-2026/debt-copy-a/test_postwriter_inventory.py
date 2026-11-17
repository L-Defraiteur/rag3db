"""Actual driver inventory gate, synthetic inventories; no writer or DB.

Negative inventory cases can be rejected by the old wrong guard first;
their passing alone never qualifies a future post-writer branch.
"""
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import ast,copy,hashlib,json,tempfile,unittest
import lifecycle as c

class PostwriterInventory(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
  self.root=Path(self.tmp.name);self.base=self.root/'code-principal.rag3db'
  def row(name,tag='a'):return dict(path=name,size=100,mode=420,sha256=tag*64)
  self.before=[row(self.base.name),row(self.base.name+'.wal'),row(self.base.name+'.ecrivain'),row('fts/shard_0')]
  self.after=[row(self.base.name,'b'),row('fts/shard_0')]
  self.dirs=[dict(path='fts',mode=493)]
 def gate(self,after=None,before=None,dirs=None,evidence_mutator=None,clock_mode=None):
  evidence=Path(tempfile.mkdtemp(prefix='observation-',dir=self.root))
  control=evidence/'Q';control.mkdir();now=[100.1 if clock_mode=='expired' else 99.]
  nonce='fixture-nonce';identity=dict(pid=42,start='123',state='R',exe=str(c.q.CLI),exe_sha256=c.q.PINS[c.q.CLI])
  profile=dict(LD_LIBRARY_PATH=str(c.q.LIB),RAG3DB_BUFFER_POOL_SIZE='1073741824',RAG3WEAVER_DB_THREADS='4')
  for name in ['lifecycle.json','lifecycle.mmd']:(control/name).write_bytes(Path(c.__file__).with_name(name).read_bytes())
  hashes={n:hashlib.sha256((control/n).read_bytes()).hexdigest() for n in ['lifecycle.json','lifecycle.mmd']}
  response=json.loads(Path(c.__file__).with_name('writer-meta-response.json').read_bytes());response['result']['result']['query']=nonce
  raw=json.dumps(response).encode()+b'\n';observation=c.protocol.public_call(raw,nonce)
  publication=dict(observation=observation,process=identity,manifest_sha256=hashes['lifecycle.json'],graph_sha256=hashes['lifecycle.mmd'],database=str(self.base),request=dict(op='call',name='observer_cycle_vie',arguments=dict(nonce=nonce)))
  def write(name,value):(evidence/name).write_text(json.dumps(value))
  write('writer-process.json',identity);write('response-publication.json',publication)
  write('shutdown-request.json',dict(process=identity,request=dict(op='shutdown'),publication_sha256=hashlib.sha256((evidence/'response-publication.json').read_bytes()).hexdigest()))
  write('writer-closed.json',dict(process=identity,state='CLOSED',rc=0,run=observation['run'],nonce=nonce,shutdown_acknowledged=True))
  write('writer-runtime-profile.json',dict(pid=42,start='123',start_after='123',exe_sha256=identity['exe_sha256'],exe_after=identity['exe'],cgroup='0::/fixture\n',cgroup_after='0::/fixture\n',expected=profile,actual=profile,diff={},maps=str(c.q.LIB/'librag3db.so')+'\n'+str(c.q.LIB/'libvector.rag3db_extension')+'\n'))
  (evidence/'writer.stdout').write_bytes(raw+b'{"ok":true,"result":{"closed":true}}\n');(evidence/'writer.stderr').write_bytes(b'')
  if evidence_mutator is not None:evidence_mutator(evidence)
  if clock_mode=='first-red':write('first-red.json',dict(reason='original refusal'))
  def directories(_):
   if clock_mode=='expire-during-collect':now[0]=100.1
   if clock_mode=='red-during-collect' and not (evidence/'first-red.json').exists():write('first-red.json',dict(reason='original refusal'))
   return self.dirs if dirs is None else dirs
  tree=ast.parse(Path(c.__file__).read_text());driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
  start=next(i for i,n in enumerate(driver.body) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='after_writer' for t in n.targets))
  end=next(i for i,n in enumerate(driver.body[start:],start) if isinstance(n,ast.Expr) and isinstance(n.value,ast.Call) and isinstance(n.value.func,ast.Name) and n.value.func.id=='save' and n.value.args and isinstance(n.value.args[0],ast.Constant) and n.value.args[0].value=='copy-after-shutdown-quiet.json')
  body=driver.body[start:end]+[ast.Return(value=ast.Constant(True))]
  function=ast.FunctionDef(name='actual_postwriter_inventory_gate',args=ast.arguments(posonlyargs=[],args=[],vararg=None,kwonlyargs=[],kw_defaults=[],kwarg=None,defaults=[]),body=body,decorator_list=[],type_params=[])
  namespace=dict(vars(c),BASE=self.base,Q=control,nonce=nonce,end=100.,env=profile,profile=profile,cfg=dict(files=hashes,active_cgroup='/fixture'),before_writer=copy.deepcopy(self.before if before is None else before),initial=dict(fin_dirs=self.dirs),fs=SimpleNamespace(file_inventory=lambda _:copy.deepcopy(self.after if after is None else after)))
  exec(compile(ast.fix_missing_locations(ast.Module(body=[function],type_ignores=[])),'<actual-postwriter-inventory>','exec'),namespace)
  with patch.object(c,'P',evidence),patch.object(c,'Q',control),patch.object(c.a,'P',evidence),patch.object(c.a,'BASE',self.base),patch.object(c.q,'P',evidence),patch.object(c.q,'ACTION_GUARD',None),patch.object(c.time,'monotonic',side_effect=lambda:now[0]),patch.object(c.q,'directories',side_effect=directories):
   try:return namespace['actual_postwriter_inventory_gate'](),None
   except ValueError as error:return False,str(error)
 def test_exact_writer_auxiliary_removal_is_not_a_readonly_mutation(self):
  accepted,reason=self.gate();self.assertTrue(accepted,reason)
 def test_unrelated_fts_removal_is_refused(self):
  accepted,_=self.gate(after=self.after[:1]);self.assertFalse(accepted)
 def test_unrelated_fts_bytes_or_mode_change_is_refused(self):
  for key,value in [('sha256','c'*64),('mode',384)]:
   with self.subTest(field=key):
    after=copy.deepcopy(self.after);after[1][key]=value;accepted,_=self.gate(after=after);self.assertFalse(accepted)
 def test_residual_or_new_wal_is_refused(self):
  for suffix in ['.wal','.wal.unexpected']:
   with self.subTest(suffix=suffix):
    after=copy.deepcopy(self.after)+[dict(self.before[1],path=self.base.name+suffix)];accepted,_=self.gate(after=after);self.assertFalse(accepted)
 def test_unrelated_addition_or_directory_change_is_refused(self):
  accepted,_=self.gate(after=self.after+[dict(self.before[1],path='unexpected')]);self.assertFalse(accepted)
  accepted,_=self.gate(dirs=[]);self.assertFalse(accepted)
 def test_duplicate_inventory_paths_cannot_be_silently_collapsed(self):
  # Unlike the nominal writer case this keeps all auxiliaries: the existing
  # dictionary conversion silently collapses the duplicate instead of refusing.
  accepted,_=self.gate(before=self.before,after=self.before+[copy.deepcopy(self.before[-1])]);self.assertFalse(accepted)
 def test_neutral_inventory_with_complete_writer_evidence_is_a_positive_control(self):
  accepted,reason=self.gate(before=self.after,after=self.after);self.assertTrue(accepted,reason)
 def test_missing_or_discordant_writer_evidence_cannot_hide_behind_inventory_refusal(self):
  cases=[('writer-closed.json','shutdown_acknowledged',False),('writer-closed.json','rc',1),('writer-closed.json','nonce','foreign'),('writer-closed.json','run','foreign'),('shutdown-request.json','publication_sha256','0'*64),('writer-runtime-profile.json','pid',43),('writer-runtime-profile.json','start','foreign'),('writer-runtime-profile.json','exe_sha256','0'*64),('writer-runtime-profile.json','cgroup','0::/foreign\n'),('writer-runtime-profile.json','maps',''),('writer-runtime-profile.json','diff',{'unexpected':True})]
  for name,key,value in cases:
   with self.subTest(file=name,field=key):
    def mutate(root):
     path=root/name;record=json.loads(path.read_text());record[key]=value;path.write_text(json.dumps(record))
    accepted,_=self.gate(before=self.after,after=self.after,evidence_mutator=mutate);self.assertFalse(accepted)
  accepted,_=self.gate(before=self.after,after=self.after,evidence_mutator=lambda root:(root/'writer-closed.json').unlink());self.assertFalse(accepted)
 def test_bad_shutdown_ack_raw_is_refused_with_neutral_inventory(self):
  def mutate(root):
   path=root/'writer.stdout';lines=path.read_bytes().splitlines(keepends=True);path.write_bytes(lines[0]+b'{"ok":true,"result":{"closed":false}}\n')
  accepted,_=self.gate(before=self.after,after=self.after,evidence_mutator=mutate);self.assertFalse(accepted)
 def test_deadline_or_first_red_before_or_during_inventory_collection_refuses(self):
  for mode in ['expired','first-red','expire-during-collect','red-during-collect']:
   with self.subTest(mode=mode):
    accepted,_=self.gate(before=self.after,after=self.after,clock_mode=mode);self.assertFalse(accepted)
 def test_primary_database_missing_or_mode_changed_is_refused(self):
  accepted,_=self.gate(before=self.after,after=self.after[1:]);self.assertFalse(accepted)
  after=copy.deepcopy(self.after);after[0]['mode']=384
  accepted,_=self.gate(before=self.after,after=after);self.assertFalse(accepted)
 def test_residual_writer_mark_and_invalid_readers_are_refused(self):
  accepted,_=self.gate(before=self.after,after=self.after+[self.before[2]]);self.assertFalse(accepted)
  readers=dict(path=self.base.name+'.readers',size=0,mode=420,sha256=hashlib.sha256(b'').hexdigest())
  for change in [dict(size=1),dict(mode=384),dict(sha256='f'*64)]:
   with self.subTest(change=change):
    accepted,_=self.gate(before=self.after,after=self.after+[dict(readers,**change)]);self.assertFalse(accepted)
  accepted,_=self.gate(before=self.after+[readers],after=self.after);self.assertFalse(accepted)
 def test_initially_absent_zero_byte_readers_is_allowed(self):
  readers=dict(path=self.base.name+'.readers',size=0,mode=420,sha256=hashlib.sha256(b'').hexdigest())
  accepted,reason=self.gate(before=self.after,after=self.after+[readers]);self.assertTrue(accepted,reason)

if __name__=='__main__':unittest.main()
