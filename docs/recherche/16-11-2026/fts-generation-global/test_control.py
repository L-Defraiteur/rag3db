"""Synthetic global admission contracts; no process, service or database."""
import copy,unittest
import qualify as q
class GlobalContracts(unittest.TestCase):
 def grant(self):
  providers=[]
  for address,model,dim in [('127.0.0.1:7878','granite-278m',768),('127.0.0.1:7879','bge-m3',1024)]:
   providers.append(dict(address=address,pid=123,start='456',exe_sha256='a'*64,identity=dict(service='rag3weaver-embeddings',modele=model,dim=dim,factice=False,dual=False,sparse=False,precision='Flex32',executable='binary',max_chars_per_text=12000)))
  return dict(c7_reference='c7',embar_reference='embar',observed_monotonic=100,valid_until_monotonic=7000,restart_authorized=False,providers=providers,postgres=dict(host='127.0.0.1',port=5433,database='rag3weaver_test',ready=True))
 def test_outer_covers_whole_composition_after_poste(self):
  argv=q.outer_command(dict(timeout='/timeout'))
  self.assertEqual(argv[1:6],['lourd','/timeout','--signal=TERM','--kill-after=2s','5998s']);self.assertEqual(argv[-1],'--compose')
 def test_default_suite_profile_does_not_reduce_or_repeat_five_tdd_tests(self):
  env=q.canonical_overrides();self.assertNotIn('SUITES',env);self.assertNotIn('RAG3WEAVER_MOTEUR_ANCIEN',env)
  self.assertEqual(env['SUITE_DU_TEMOIN'],'lib');self.assertEqual(len(env['SUITES_EN_PLUS'].split()),5)
  self.assertEqual(q.WITNESS,'code_sync::fts_guard_double_tests::generations_sync_removes_a_committed_fts_ghost')
 def test_fresh_real_service_grant_covers_receipt(self):
  self.assertEqual(len(q.validate_services(self.grant(),110,6000)['providers']),2)
  variants=[]
  a=self.grant();a['restart_authorized']=True;variants.append(a)
  a=self.grant();a['valid_until_monotonic']=500;variants.append(a)
  a=self.grant();a['observed_monotonic']=-1000;variants.append(a)
  a=self.grant();a['providers'][1]['address']='127.0.0.1:7970';variants.append(a)
  a=self.grant();a['providers'][0]['identity']['factice']=True;variants.append(a)
  a=self.grant();a['postgres']['ready']=False;variants.append(a)
  for a in variants:
   with self.assertRaises(AssertionError):q.validate_services(a,110,6000)
 def test_failed_detection_preserves_expected_should_panic(self):
  for raw in ['test x ... FAILED','test result: FAILED.','FAILED']:self.assertTrue(q.FAILED.search(raw))
  self.assertFalse(q.FAILED.search("thread 'expected' panicked at fixture\ntest expected - should panic ... ok"))
if __name__=='__main__':unittest.main(failfast=True)
