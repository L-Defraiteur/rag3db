"""Pure actual Cargo artifact selector; no Cargo/ELF/filesystem invocation."""
from pathlib import Path
import copy,json,unittest
import build_private_cli as b

class ArtifactSelection(unittest.TestCase):
 def setUp(self):
  self.crate=Path('/fixture/extension/rag3weaver')
  self.events=[dict(reason='compiler-artifact',target=dict(name=name,kind=['bin'],src_path=str(self.crate/'src/bin'/(name+'.rs'))),profile=dict(test=False,opt_level='3',debug_assertions=False),features=list(b.FEATURES),manifest_path=str(self.crate/'Cargo.toml'),executable='/fixture/target/release/'+name) for name in b.NAMES]
 def select(self,events):return b.select_artifacts('\n'.join(json.dumps(x) for x in events),self.crate)
 def test_two_unique_real_bin_events_are_selected(self):self.assertEqual(set(self.select(self.events)),set(b.NAMES))
 def test_missing_bin_refuses(self):
  with self.assertRaises(ValueError):self.select(self.events[:1])
 def test_duplicate_bin_refuses_instead_of_last_match(self):
  with self.assertRaises(ValueError):self.select(self.events+[self.events[0]])
 def test_feature_closure_must_be_exact(self):
  for features in [[],b.FEATURES+['burn-embedder'],[x for x in b.FEATURES if x!='default']]:
   with self.subTest(features=features):
    events=copy.deepcopy(self.events);events[0]['features']=features
    with self.assertRaises(ValueError):self.select(events)
 def test_test_artifact_or_non_bin_refuses(self):
  for field in ['kind','test']:
   with self.subTest(field=field):
    events=copy.deepcopy(self.events)
    if field=='kind':events[0]['target']['kind']=['lib']
    else:events[0]['profile']['test']=True
    with self.assertRaises(ValueError):self.select(events)
 def test_foreign_manifest_or_source_refuses(self):
  for field in ['manifest_path','src_path']:
   with self.subTest(field=field):
    events=copy.deepcopy(self.events)
    if field=='manifest_path':events[0][field]='/foreign/Cargo.toml'
    else:events[0]['target'][field]='/foreign/main.rs'
    with self.assertRaises(ValueError):self.select(events)
 def test_error_diagnostic_refuses_even_with_two_artifacts(self):
  with self.assertRaises(ValueError):self.select(self.events+[dict(reason='compiler-message',message=dict(level='error'))])
 def test_non_json_log_refuses(self):
  with self.assertRaises(ValueError):b.select_artifacts('not Cargo JSON',self.crate)

if __name__=='__main__':unittest.main()
