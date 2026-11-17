from pathlib import Path
import copy,importlib.util,unittest
spec=importlib.util.spec_from_file_location('return_gate',Path(__file__).with_name('return-proof-gate.py'));g=importlib.util.module_from_spec(spec);spec.loader.exec_module(g)
WAL='test_future_wal_and_all_new_neighbours_leave_the_live_set'
def fixture():
 return dict(schema_version=1,verdict='VERT',qualification_scope='private_filesystem_fixtures_no_real_restore',base='code-principal',original_receipt=dict(path='/proof/original.json',sha256='a'*64),snapshot_root='/snap/root',snapshot_inventory=dict(tree_sha256='b'*64),original_receipt_unchanged=True,snapshot_data_unchanged=True,live_restore_executed=False,cli_authorized=False,retention_hours_minimum=72,implementation=dict(git_commit='c'*40,repository_path='restore.py',sha256='d'*64),fixture_suite=dict(git_commit='c'*40,repository_path='test_restore.py',sha256='e'*64,passed=5,failed=0,skipped=0),return_script=dict(path='/snap/root/RETOUR_v2.sh'))
def validate(record):return g.validate_record(record,'code-principal',Path('/proof/original.json'),'a'*64,Path('/snap/root'),'b'*64)
class ReturnProof(unittest.TestCase):
 def test_complete_record_keeps_scope_bounded(self):self.assertFalse(validate(fixture())['live_restore_executed'])
 def test_changed_snapshot_or_original_receipt_refused(self):
  for field,value in [('snapshot_root','/other'),('original_receipt_unchanged',False),('snapshot_data_unchanged',False)]:
   r=fixture();r[field]=value
   with self.assertRaises(ValueError):validate(r)
 def test_skipped_failed_or_zero_fixtures_refused(self):
  for field,value in [('passed',0),('failed',1),('skipped',1)]:
   r=fixture();r['fixture_suite'][field]=value
   with self.assertRaises(ValueError):validate(r)
 def test_old_launcher_or_short_retention_refused(self):
  r=fixture();r['return_script']['path']='/snap/root/RETOUR.sh'
  with self.assertRaises(ValueError):validate(r)
  r=fixture();r['retention_hours_minimum']=24
  with self.assertRaises(ValueError):validate(r)
 def test_live_authority_cannot_be_claimed_by_fixture(self):
  r=fixture();r['cli_authorized']=True
  with self.assertRaises(ValueError):validate(r)
 def test_actual_log_requires_unique_wal_witness(self):
  names=[WAL,'test_one','test_two','test_three','test_four'];text=''.join(n+' (__main__.Tests.'+n+') ... ok\n' for n in names)+'\nRan 5 tests in 0.1s\n\nOK\n'
  self.assertEqual(g.validate_fixture_log(text,5),names)
  with self.assertRaises(ValueError):g.validate_fixture_log(text.replace(WAL,'test_no_wal'),5)
  with self.assertRaises(ValueError):g.validate_fixture_log(text.replace('test_four','test_one'),5)
if __name__=='__main__':unittest.main(failfast=True)
