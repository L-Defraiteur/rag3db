"""Actual public response fixture, pure transport state machine; no subprocess."""
from pathlib import Path
import copy,json,unittest
import lifecycle_protocol as p

class PublicLifecycle(unittest.TestCase):
 def setUp(self):
  self.response=json.loads(Path(__file__).with_name('writer-meta-response.json').read_bytes());self.nonce=self.response['result']['result']['query'];self.raw=json.dumps(self.response).encode()
 def test_publication_precedes_shutdown_then_ack_and_same_process_rc(self):
  session=p.Session(self.nonce);published=[];record=session.call_response(self.raw,published.append)
  self.assertEqual(published,[record]);self.assertEqual(record['nonce'],self.nonce);self.assertEqual(record['run'],self.response['result']['run'])
  self.assertEqual(session.shutdown_request(),b'{"op":"shutdown"}\n');session.closed_response(b'{"ok":true,"result":{"closed":true}}');self.assertEqual(session.finish(0)['state'],'CLOSED')
 def test_failed_publication_never_enables_shutdown(self):
  session=p.Session(self.nonce)
  def fail(record):raise OSError('fixture fsync refused')
  with self.assertRaises(OSError):session.call_response(self.raw,fail)
  self.assertEqual(session.state,'CALL_SENT')
  with self.assertRaises(ValueError):session.shutdown_request()
 def test_call_refusals_nonce_aliases_and_public_types(self):
  variants=[]
  for key,value in [('query','foreign'),('pending_count',0),('pendingCount',False),('partial',0),('signals',['vector']),('warnings',{}),('vectorCount',1),('bm25Count',False)]:
   item=copy.deepcopy(self.response);item['result']['result'][key]=value;variants.append(item)
  for key,value in [('ok',False),('mustReopen',True)]:
   item=copy.deepcopy(self.response);item[key]=value;variants.append(item)
  item=copy.deepcopy(self.response);item['result']['validation']['accepted']=1;variants.append(item)
  item=copy.deepcopy(self.response);item['result']['delivery']['ok']=1;variants.append(item)
  for i,item in enumerate(variants):
   with self.subTest(case=i),self.assertRaises(ValueError):p.public_call(json.dumps(item).encode(),self.nonce)
 def test_duplicates_wrong_order_bad_ack_and_nonzero_exit_refused(self):
  with self.assertRaises(ValueError):p.decode(b'{"ok":true,"ok":false}')
  with self.assertRaises(ValueError):p.decode(b'{"value":NaN}')
  with self.assertRaises(ValueError):p.Session('')
  session=p.Session(self.nonce)
  with self.assertRaises(ValueError):session.closed_response(b'{"ok":true,"result":{"closed":true}}')
  session.call_response(self.raw,lambda value:None)
  with self.assertRaises(ValueError):session.call_response(self.raw,lambda value:None)
  session.shutdown_request()
  for raw in [b'{"ok":true,"result":{"closed":1}}',b'{"ok":1,"result":{"closed":true}}',b'{"ok":false}',b'{"ok":true,"result":{"closed":true},"extra":0}']:
   with self.subTest(raw=raw),self.assertRaises(ValueError):session.closed_response(raw)
  session.closed_response(b'{"ok":true,"result":{"closed":true}}')
  for rc in [1,None,False]:
   with self.subTest(rc=rc),self.assertRaises(ValueError):session.finish(rc)

if __name__=='__main__':unittest.main(failfast=True)
