"""Prepublished reader witnesses: temporary JSON only, no transport, native or /proc."""
from pathlib import Path
import copy,hashlib,json,tempfile,unittest
from types import SimpleNamespace
import provider_reader as p

def sha(raw):return hashlib.sha256(raw).hexdigest()
def encoded(v):return json.dumps(v,sort_keys=True,separators=(',',':'),allow_nan=False).encode()

def fixture(root):
 binding=dict(source='a'*40,roles={'host':dict(target_name='rag3weaver-backend',target_kind='bin',profile_test=False,features=['default'],manifest='main')})
 binding_sha=sha(encoded(binding));config=dict(schema_version=1,kind='archi-publication-reader',source='a'*40,pilot='b'*40,execution_id='run-one',phase_id='common-one',prepared_sha256='c'*64,plan_sha256='d'*64,root=str(root),owner_uid=__import__('os').getuid(),bindings={'global':binding},stages=[dict(label='global',sequence=0,binding_sha256=binding_sha,files=['ledger','host'],artifacts=['host'],window_file='global-window.json',ledger_file='global-ledger.json',artifact_files={'host':'host-elf'},source_contract='frozen-app')])
 artifact=dict(copy=str(root/'host-elf'),sha256='e'*64,size=3,features=['default'],artifact=dict(manifest_path='/source/Cargo.toml',target=dict(name='rag3weaver-backend',kind=['bin']),profile=dict(test=False),features=['default']))
 ledger=dict(source='a'*40,pilot='b'*40,created=15.,artifacts={'host':artifact})
 (root/'host-elf').write_bytes(b'ELF');raw=encoded(ledger);(root/'global-ledger.json').write_bytes(raw)
 window=dict(schema_version=1,label='global',sequence=0,source='a'*40,pilot='b'*40,execution_id='run-one',phase_id='common-one',prepared_sha256='c'*64,plan_sha256='d'*64,binding_sha256=binding_sha,ledger_sha256=sha(raw),opened_monotonic=16.,deadline_monotonic=106.,operation_end_monotonic=200.,owner=dict(pid=90,start='12',script_sha256='f'*64))
 (root/'global-window.json').write_bytes(encoded(window))
 phase=SimpleNamespace(execution_id='run-one',phase_id='common-one',prepared_sha256='c'*64)
 return config,ledger,window,phase

class ReadOnly(unittest.TestCase):
 def test_reader_binds_exact_roles_original_ledger_bytes_and_phase(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);config,ledger,window,phase=fixture(root);value=p.Reader(config).read('global',phase)
   self.assertEqual(value['ledger_bytes_sha256'],sha(encoded(ledger)));self.assertEqual(set(value['files']),{'ledger','host'})
   self.assertEqual(value['window_sha256'],sha(encoded(window)))
 def test_unknown_label_or_changed_source_cannot_define_the_plan(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);config,ledger,window,phase=fixture(root);reader=p.Reader(config)
   with self.assertRaises(ValueError):reader.read('another',phase)
   phase.phase_id='another'
   with self.assertRaises(ValueError):reader.read('global',phase)
 def test_plan_is_frozen_against_mutation_by_caller(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);config,ledger,window,phase=fixture(root);reader=p.Reader(config);config['source']='z'*40
   self.assertEqual(reader.read('global',phase)['ledger']['source'],'a'*40)
 def test_missing_extra_or_changed_artifact_is_refused(self):
  for mode in ['extra','missing','features']:
   with self.subTest(mode=mode),tempfile.TemporaryDirectory() as tmp:
    root=Path(tmp);config,ledger,window,phase=fixture(root)
    if mode=='extra':ledger['artifacts']['alien']=copy.deepcopy(ledger['artifacts']['host'])
    elif mode=='missing':ledger['artifacts'].pop('host')
    else:ledger['artifacts']['host']['artifact']['features']=[]
    raw=encoded(ledger);(root/'global-ledger.json').write_bytes(raw);window['ledger_sha256']=sha(raw);(root/'global-window.json').write_bytes(encoded(window))
    with self.assertRaises(ValueError):p.Reader(config).read('global',phase)
 def test_changed_ledger_bytes_refuse_even_if_json_value_is_equal(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);config,ledger,window,phase=fixture(root);(root/'global-ledger.json').write_bytes(encoded(ledger)+b'\n')
   with self.assertRaises(ValueError):p.Reader(config).read('global',phase)
 def test_symlink_ledger_and_parent_paths_are_not_followed(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);config,ledger,window,phase=fixture(root);(root/'global-ledger.json').rename(root/'actual.json');(root/'global-ledger.json').symlink_to(root/'actual.json')
   with self.assertRaises((ValueError,OSError)):p.Reader(config).read('global',phase)
 def test_duplicate_keys_and_nonfinite_window_refuse(self):
  for raw in [b'{"label":"global","label":"global"}',b'{"deadline_monotonic":NaN}']:
   with self.subTest(raw=raw),tempfile.TemporaryDirectory() as tmp:
    root=Path(tmp);config,ledger,window,phase=fixture(root);(root/'global-window.json').write_bytes(raw)
    with self.assertRaises(ValueError):p.Reader(config).read('global',phase)
 def test_event_does_not_extend_operation_or_select_an_arbitrary_path(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);config,ledger,window,phase=fixture(root);window['deadline_monotonic']=201.;(root/'global-window.json').write_bytes(encoded(window))
   with self.assertRaises(ValueError):p.Reader(config).read('global',phase)
   config['stages'][0]['ledger_file']='../outside'
   with self.assertRaises(ValueError):p.Reader(config)
 def test_runtime_liveness_or_transport_is_not_claimed_by_the_reader(self):
  tree=__import__('ast').parse(Path(p.__file__).read_text())
  self.assertNotIn('subprocess',__import__('ast').unparse(tree));self.assertNotIn('/proc',Path(p.__file__).read_text())
  self.assertFalse(hasattr(p.Reader,'publish_ack'));self.assertFalse(hasattr(p.Reader,'export'));self.assertFalse(hasattr(p.Reader,'run'))
if __name__=='__main__':unittest.main(failfast=True)
