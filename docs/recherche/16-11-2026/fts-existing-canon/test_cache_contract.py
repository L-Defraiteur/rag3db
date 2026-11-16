"""Cache provenance and publication witnesses; no Cargo, /proc or real process."""
import ast,copy,json,os,stat,tempfile,time,types,unittest
from pathlib import Path
from cache_contract import verify_cache_contract,publish_composed
class CacheContract(unittest.TestCase):
 def baseline(self):
  regular=dict(device=1,inode=3,mode=stat.S_IFREG|0o600,uid=os.getuid())
  return dict(target=dict(device=1,inode=2,mode=stat.S_IFDIR|0o700,uid=os.getuid()),marker=dict(identity=regular,owner='/old/crate'),previous_files={'supervisor/autorite.json':'auth','supervisor/verdict.json':'scope','driver/ledger.json':'ledger','manifest.json':'manifest'},manifest_entries={'supervisor/autorite.json':'auth','supervisor/verdict.json':'scope','driver/ledger.json':'ledger'},live_previous_files={name:dict(identity=regular,sha256=value) for name,value in [('authority','auth'),('verdict','scope')]},authority=dict(etat='termine',pid=4),scope=dict(verdict='VERT',code=0,restes=[],nettoye=True),elf=dict(identity=regular,path='/target/elf',sha256='elf',ledger_sha256='elf'),old_worktree=dict(head='04634cfed667711192efd313ff1418ef8dfd5c26',status=''))
 def test_fresh_contract_accepts_exact_snapshot_and_refuses_each_divergence(self):
  baseline=self.baseline();verify_cache_contract(baseline,baseline)
  for field,key,value in [('target','inode',99),('target','device',99),('marker','owner','/other'),('authority','etat','actif'),('scope','verdict','ROUGE'),('previous_files','manifest.json','changed'),('elf','sha256','changed'),('old_worktree','status',' M source')]:
   with self.subTest(field=field,key=key),self.assertRaises(AssertionError):
    current=copy.deepcopy(baseline);current[field][key]=value;verify_cache_contract(current,baseline)
 def test_mutating_both_archived_and_original_receipt_cannot_bypass_qualified_manifest(self):
  current=self.baseline();current['previous_files']['supervisor/autorite.json']='modified';current['live_previous_files']['authority']['sha256']='modified'
  with self.assertRaises(AssertionError):verify_cache_contract(current,current)
 def test_actual_runner_publishes_snapshot_before_refusal_and_never_starts_guard(self):
  tree=ast.parse(Path(__file__).with_name('driver.py').read_text());node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='run')
  with tempfile.TemporaryDirectory(prefix='fts-cache-contract-fixture-') as tmp:
   children=[];ns=dict(check=lambda:None,END=time.monotonic()+100,time=time,P=Path(tmp),CONFIG={'previous_contract':{}},json=json,capture_cache_contract=lambda *args:dict(capture_error='fixture old receipt unreadable'),verify_cache_contract=verify_cache_contract,digest=lambda *args:None,subprocess=types.SimpleNamespace(Popen=lambda *args,**kwargs:children.append(args)))
   exec(compile(ast.Module(body=[node],type_ignores=[]),'actual-run-cache-contract','exec'),ns)
   with self.assertRaises(AssertionError):ns['run']('target-guard',['fixture'],{})
   self.assertEqual(json.loads((Path(tmp)/'cache-contract-after-poste.json').read_text())['capture_error'],'fixture old receipt unreadable')
   self.assertEqual(children,[])
 def test_publication_rechecks_deadline_inside_exclusive_file_open(self):
  with tempfile.TemporaryDirectory(prefix='fts-publication-fixture-') as tmp:
   root=Path(tmp)
   green=publish_composed(root/'green.json',dict(verdict='VERT'),10,now=lambda:9)
   red=publish_composed(root/'red.json',dict(verdict='VERT'),10,now=lambda:10)
   self.assertEqual(green['verdict'],'VERT');self.assertEqual(red['verdict'],'ROUGE')
   self.assertEqual(json.loads((root/'red.json').read_text())['verdict'],'ROUGE')
   with self.assertRaises(FileExistsError):publish_composed(root/'red.json',dict(verdict='VERT'),10,now=lambda:1)
if __name__=='__main__':unittest.main()
