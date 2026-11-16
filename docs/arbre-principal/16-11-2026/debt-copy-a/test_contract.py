"""Hermetic transport/control fixtures only; no native Database, compiler or model."""
from pathlib import Path
import collections,importlib.util,io,json,struct,unittest
import typed_rows as t
import audit
spec=importlib.util.spec_from_file_location('qualification',Path(__file__).with_name('qualify.py'));q=importlib.util.module_from_spec(spec);spec.loader.exec_module(q)
def integer(n):return struct.pack('<Q',n)
def binary(v):return integer(len(v))+v
def value(body,typename=b'STRING'):return binary(typename)+b'1s'+binary(body)
def frame(rows,columns=1):return b'R3TYP002'+integer(columns)+integer(len(rows))+b''.join(binary(r) for r in rows)
class Contracts(unittest.TestCase):
 def test_raw_text_preserves_null_newlines_utf8_and_delimiters(self):
  body='é\n\x00|'.encode();rows=[];t.consume(io.BytesIO(frame([value(body)])),rows.append);self.assertEqual(t.text(rows[0][0]),body.decode())
 def test_null_and_empty_are_distinct(self):
  null=binary(b'STRING')+b'0';self.assertNotEqual(t.consume(io.BytesIO(frame([null])))['rows'],t.consume(io.BytesIO(frame([value(b'')])))['rows'])
 def test_duplicates_preserved_and_order_irrelevant(self):
  a=value(b'a');b=value(b'b');self.assertEqual(t.consume(io.BytesIO(frame([a,b,a]))),t.consume(io.BytesIO(frame([b,a,a]))));self.assertEqual(sorted(t.consume(io.BytesIO(frame([a,b,a])))['rows'].values()),[1,2])
 def test_bad_framing_or_trailing_bytes_fail(self):
  for raw in [b'',frame([value(b'x')])[:-1],frame([value(b'x')])+b'x',frame([value(b'x')+b'x'])]:
   with self.subTest(length=len(raw)),self.assertRaises(ValueError):t.consume(io.BytesIO(raw))
 def test_float_bits_and_nested_values(self):
  floating=binary(b'FLOAT')+b'1f'+struct.pack('<f',1.25);nested=binary(b'FLOAT[1]')+b'1a'+integer(1)+floating;rows=[];t.consume(io.BytesIO(frame([nested])),rows.append);self.assertEqual(rows,[[('FLOAT[1]',[('FLOAT',1.25)])]])
 def test_unexpected_type_or_duplicate_metadata_rejected(self):
  with self.assertRaises(ValueError):t.text(('FLOAT',1.0))
  with self.assertRaises(ValueError):audit.strict_json('{"a":1,"a":2}')
 def test_exact_manifest_prevents_rescan_and_uses_existing_body(self):
  c=json.loads(Path(__file__).with_name('backend.json').read_text());options=c['tools']['payer_retard_dense']['bindings']['options'];self.assertEqual(options['signals'],[]);self.assertEqual(options['exige'],['data','dense']);self.assertFalse(c['embeddings']['catch_up_on_open']);self.assertEqual(c['workspace']['commands'],'off');self.assertTrue(c['workspace']['read_only']);self.assertFalse(c['reactions']);self.assertFalse(c['watches']);self.assertFalse(c['tools']['payer_retard_dense']['policy']['read_files']);graph=Path(__file__).with_name('payer-retard-dense.mmd').read_text();self.assertIn('source.meta',graph);self.assertNotIn('IndexNode(',graph);self.assertNotIn('VectorSearchNode',graph)
 def test_source_meta_warnings_or_partial_fail_before_shutdown(self):
  meta=dict(warnings=[],partial=False,pending_count=0,signals=[]);self.assertEqual(q.verify_meta(dict(ok=True,result=dict(result=meta))),meta)
  for key,v in [('warnings',['refus']),('partial',True),('pending_count',1),('signals',['vector'])]:
   with self.subTest(key=key),self.assertRaises(ValueError):q.verify_meta(dict(ok=True,result=dict(result=dict(meta,**{key:v}))))
 def test_stale_or_wrong_provider_grant_refused(self):
  grant=dict(provider='127.0.0.1:7878',model='granite-278m',dimension=768,c7_reference='c7',embar_reference='embar',restart_authorized=False,observed_monotonic=100,valid_until_monotonic=1000,identity=dict(service='rag3weaver-embeddings',modele='granite-278m',dim=768,factice=False,dual=False,sparse=False,max_chars_per_text=5000,executable='bin@mtime',precision='Flex32'))
  self.assertEqual(q.validate_grant(grant,101,4255),grant['identity'])
  for x,now,maximum in [(grant,300,4255),(grant,101,5001),(dict(grant,model='other'),101,4255),(dict(grant,restart_authorized=True),101,4255)]:
   with self.subTest(now=now,cap=maximum),self.assertRaises(ValueError):q.validate_grant(x,now,maximum)
 def test_vector_judges_reject_dimension_nonfinite_stale_marker_or_index(self):
  good=[('STRING',b'id'),('FLOAT[768]',[('FLOAT',0.25)]*768),('STRING',b'h'),('STRING',b'h'),('STRING',None)]
  class Reader:
   def __init__(self,row,index='READY'):self.row=row;self.index=index
   def visit(self,sql,callback):
    for row in self.collect(sql):callback(row)
   def collect(self,sql):
    if 'show_indexes' in sql:return [[('STRING',b'ScopeContent_Chunk'),('STRING',b'ScopeContent_Chunk_vec__granite_278m'),('STRING',self.index.encode()),('BOOL','True')]]
    if 'RETURN n._uuid,n.embedding' in sql:return [self.row]
    return []
  self.assertEqual(audit.finished(Reader(good),['id'])['debt'],0)
  variants=[]
  for col,value in [(1,('FLOAT[768]',[('FLOAT',0.25)]*767)),(1,('FLOAT[768]',[('FLOAT',float('nan'))]*768)),(3,('STRING',b'other'))]:
   row=list(good);row[col]=value;variants.append(Reader(row))
  variants.append(Reader(good,'NOTED'))
  for reader in variants:
   with self.assertRaises(ValueError):audit.finished(reader,['id'])
if __name__=='__main__':unittest.main(failfast=True)
