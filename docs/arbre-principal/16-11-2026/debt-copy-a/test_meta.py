"""Public metadata contract from the actual refused response; no process/DB."""
from pathlib import Path
import copy,json,unittest
import qualify as q

class MetaContract(unittest.TestCase):
 def setUp(self):
  self.response=json.loads(Path(__file__).with_name('writer-meta-response.json').read_bytes())
 def test_real_camelcase_response_is_accepted(self):
  self.assertEqual(q.verify_meta(self.response),self.response['result']['result'])
 def test_pending_count_missing_snake_or_conflicting_is_refused(self):
  original=self.response['result']['result']
  for mode in ['missing','snake-only','conflicting','duplicate-alias']:
   r=copy.deepcopy(self.response);m=r['result']['result']
   if mode in ['missing','snake-only']:m.pop('pendingCount')
   if mode!='missing':m['pending_count']=1 if mode=='conflicting' else 0
   with self.subTest(mode=mode),self.assertRaises(ValueError):q.verify_meta(r)
 def test_public_refusals_remain_strict(self):
  for key,value in [('pendingCount',1),('pendingCount',False),('pendingCount',None),('partial',True),('warnings',['refus']),('signals',['vector'])]:
   r=copy.deepcopy(self.response);r['result']['result'][key]=value
   with self.subTest(key=key,value=value),self.assertRaises(ValueError):q.verify_meta(r)
  r=copy.deepcopy(self.response);r['ok']=False
  with self.assertRaises(ValueError):q.verify_meta(r)

if __name__=='__main__':unittest.main(failfast=True)
