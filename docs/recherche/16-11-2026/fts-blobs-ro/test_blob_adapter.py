"""Synthetic binary transport witnesses, never a database or Lucivy index."""
import struct,tempfile,unittest
from pathlib import Path
from blob_adapter import decode_frames,materialize
INDEX='Lucivy_ScopeContent'
def frames(rows):
 return b'RFTSB001'+struct.pack('<Q',len(rows))+b''.join(struct.pack('<QQ',len(key),len(data))+key+data for key,data in rows)
def fixture():
 return {INDEX+'/_shard_config.json':b'{"shards":1}',INDEX+'/root.bin':b'\x00\xff\\\n','Lucivy_'+INDEX+'/shard_0/meta.json':b'{}'}
class BlobAdapter(unittest.TestCase):
 def test_binary_values_survive_framing(self):
  data=b'\x00\xff\\\n"';self.assertEqual(decode_frames(frames([(b'index/blob',data)])),{'index/blob':data})
 def test_transport_refuses_invalid_frames(self):
  valid=frames([(b'x',b'\x00\xff')])
  for raw in [b'',b'BADMAGIC'+valid[8:],valid[:-1],valid+b'x',frames([(b'x',b'a'),(b'x',b'b')]),frames([(b'bad\x00key',b'x')]),frames([(b'\xff',b'x')])]:
   with self.subTest(raw=raw),self.assertRaises(ValueError):decode_frames(raw)
 def test_materialization_preserves_bytes_and_exact_double_prefix(self):
  with tempfile.TemporaryDirectory(prefix='fts-blob-adapter-') as tmp:
   root=Path(tmp)/'snapshot';manifest=materialize(fixture(),INDEX,root)
   self.assertEqual((root/'root.bin').read_bytes(),b'\x00\xff\\\n');self.assertEqual((root/'shard_0/meta.json').read_bytes(),b'{}');self.assertEqual(len(manifest),3)
 def test_bad_paths_config_or_inventory_refuse_before_creation(self):
  cases=[]
  for key in [INDEX+'/../escape',INDEX+'/nested/file','Lucivy_'+INDEX+'/shard_1/meta.json','Lucivy_'+INDEX+'/shard_0/../escape']:
   value=fixture();value[key]=b'x';cases.append(value)
  for raw in [b'{',b'{"shards":0}',b'{"shards":true}',b'{"shards":2}',b'{"shards":"1"}']:
   value=fixture();value[INDEX+'/_shard_config.json']=raw;cases.append(value)
  value=fixture();del value['Lucivy_'+INDEX+'/shard_0/meta.json'];cases.append(value)
  for value in cases:
   with self.subTest(value=value),tempfile.TemporaryDirectory(prefix='fts-bad-blob-adapter-') as tmp:
    root=Path(tmp)/'snapshot'
    with self.assertRaises(ValueError):materialize(value,INDEX,root)
    self.assertFalse(root.exists())
 def test_existing_output_or_alias_index_cannot_be_overwritten(self):
  with tempfile.TemporaryDirectory(prefix='fts-no-overwrite-') as tmp:
   root=Path(tmp);sentinel=root/'sentinel';sentinel.write_bytes(b'preserved')
   with self.assertRaises(ValueError):materialize(fixture(),INDEX,root)
   with self.assertRaises(ValueError):materialize(fixture(),'../alias',root/'new')
   self.assertEqual(sentinel.read_bytes(),b'preserved')
if __name__=='__main__':unittest.main()
