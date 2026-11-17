"""Prepublished counterexamples for the real Reader, temp files only, no lifecycle."""
from pathlib import Path
import copy,json,shutil,tempfile,unittest
from unittest.mock import patch
import provider_reader as p
from test_provider import fixture,encoded,sha

def broker_fixture(root):
 config,ledger,window,phase=fixture(root);identity='task-1-123';label='agent-red1'
 role=config['bindings']['global']['roles']['host'];role.update(ledger_key='task',target_name='rag3weaver',target_kind='lib',profile_test=True,features=['code','daemon','rag3db-native'])
 binding=dict(source=config['source'],roles={label:role});binding_sha=sha(encoded(binding))
 config['bindings']={label:binding};config['broker_sources']={label:dict(cwd='/private/task',manifest='/private/task/Cargo.toml')}
 config['stages']=[dict(label=label,sequence=0,binding_sha256=binding_sha,files=['ledger',label],artifacts=[label],window_file=label+'-window.json',ledger_file='broker-ledger',artifact_files={label:'broker-elf'},broker_bound=True,source_contract='acquired agent source')]
 artifact=ledger['artifacts']['host'];artifact['copy']=str(root/(identity+'-elf'));artifact['cargo_artifact']=artifact.pop('artifact');artifact['cargo_artifact'].update(manifest_path='/private/task/Cargo.toml',target=dict(name='rag3weaver',kind=['lib']),profile=dict(test=True),features=role['features']);artifact['features']=role['features']
 ledger.update(stage='agent-red',source_head=config['source'],source_patch_sha256='f'*64,source_state=dict(head=config['source'],index='fixture',files={}),artifacts={'task':artifact})
 admission=dict(identity=identity,stage='agent-red',source_head=config['source'],source_state=copy.deepcopy(ledger['source_state']),patch_sha256='f'*64,cwd='/private/task')
 (root/'host-elf').rename(root/(identity+'-elf'))
 window.update(label=label,binding_sha256=binding_sha,admission_identity=identity)
 def write():
  raw=encoded(ledger);window['ledger_sha256']=sha(raw)
  (root/(identity+'-ledger.json')).write_bytes(raw);(root/(identity+'-admission.json')).write_bytes(encoded(admission));(root/(label+'-window.json')).write_bytes(encoded(window))
 write();return config,ledger,window,phase,admission,write

class Reserves(unittest.TestCase):
 def test_real_reader_accepts_consistent_broker_metadata_without_physical_admission(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);config,ledger,window,phase,admission,write=broker_fixture(root)
   value=p.Reader(config).read('agent-red1',phase)
   self.assertEqual(value['admission'],admission);self.assertFalse(value['physical_admission'])
 def test_real_reader_refuses_foreign_admission_head(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);config,ledger,window,phase,admission,write=broker_fixture(root);admission['source_head']='f'*40;write()
   with self.assertRaises(ValueError):p.Reader(config).read('agent-red1',phase)
 def test_real_reader_refuses_matching_states_with_foreign_head(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);config,ledger,window,phase,admission,write=broker_fixture(root);ledger['source_state']['head']='f'*40;admission['source_state']['head']='f'*40;write()
   with self.assertRaises(ValueError):p.Reader(config).read('agent-red1',phase)
 def test_real_reader_refuses_bad_broker_identity_and_stage(self):
  for key,bad in [('identity','task-2-123'),('stage','agent-green')]:
   with self.subTest(key=key),tempfile.TemporaryDirectory() as tmp:
    root=Path(tmp);config,ledger,window,phase,admission,write=broker_fixture(root);admission[key]=bad;write()
    with self.assertRaises(ValueError):p.Reader(config).read('agent-red1',phase)
 def test_real_reader_refuses_manifest_chosen_from_another_admission_cwd(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);config,ledger,window,phase,admission,write=broker_fixture(root);admission['cwd']='/other/task';ledger['artifacts']['task']['cargo_artifact']['manifest_path']='/other/task/Cargo.toml';write()
   with self.assertRaises(ValueError):p.Reader(config).read('agent-red1',phase)
 def test_real_reader_refuses_admission_replaced_during_artifact_read(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);config,ledger,window,phase,admission,write=broker_fixture(root);reader=p.Reader(config);original=reader._artifact
   def mutate(name):
    value=original(name);admission['source_head']='f'*40;write();return value
   with patch.object(reader,'_artifact',side_effect=mutate),self.assertRaises(ValueError):reader.read('agent-red1',phase)
 def test_real_reader_refuses_parent_symlink_substituted_after_construction(self):
  with tempfile.TemporaryDirectory() as tmp:
   base=Path(tmp);parent=base/'parent';root=parent/'root';root.mkdir(parents=True);config,ledger,window,phase=fixture(root);reader=p.Reader(config)
   alternate=base/'alternate';shutil.copytree(parent,alternate);parent.rename(base/'original');parent.symlink_to(alternate,target_is_directory=True)
   with self.assertRaises(ValueError):reader.read('global',phase)
 def test_real_reader_refuses_same_uid_root_directory_replaced_after_construction(self):
  with tempfile.TemporaryDirectory() as tmp:
   base=Path(tmp);root=base/'root';root.mkdir();config,ledger,window,phase=fixture(root);reader=p.Reader(config)
   shutil.copytree(root,base/'replacement');root.rename(base/'original');(base/'replacement').rename(root)
   with self.assertRaises(ValueError):reader.read('global',phase)
if __name__=='__main__':unittest.main(failfast=True)
