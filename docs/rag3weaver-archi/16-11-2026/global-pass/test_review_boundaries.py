"""Prepublished review counterexamples; only fake processes and temporary source files."""
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import ast,json,re,tempfile,unittest
import broker,d6,evidence,observe,policy
HERE=Path(__file__).parent
class Boundaries(unittest.TestCase):
 def test_actual_d6_model_assignment_matches_the_rust_provider_schema(self):
  tree=ast.parse((HERE/'d6.py').read_text())
  assignment=next(n for n in ast.walk(tree) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Subscript) and ast.unparse(t)=="value['models']['llm']" for t in n.targets))
  ns=dict(value=dict(models={}))
  exec(compile(ast.Module(body=[assignment],type_ignores=[]),'actual-model-declaration','exec'),ns)
  from settings import C
  rust=(C/'src/model_source.rs').read_text();enum=rust.split('pub enum Provider {',1)[1].split('\n}',1)[0]
  variants={v.lower() for v in re.findall(r'^    ([A-Z][a-z]+),',enum,re.M)}
  model=ns['value']['models']['llm'];self.assertIn(model['provider'],variants)
  self.assertEqual(model['provider'],'compatible');self.assertEqual(model['protocol'],'vertex')
  self.assertIn('source.provider == Provider::Compatible && source.protocol.as_deref() == Some("vertex")',rust)
 def test_complete_source_snapshot_tracks_index_and_untracked_files(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);(root/'src').mkdir();(root/'src/backend.rs').write_text('before')
   stage=['100644 '+'a'*40+' 0\tsrc/backend.rs\0'];others=['']
   def git(*args):
    return {'rev-parse':policy.APP,'ls-files':others[0] if '--others' in args else stage[0]}[args[0]]
   before=evidence.source_state(root,git)
   (root/'src/hidden.rs').write_text('included code');others[0]='src/hidden.rs\0'
   self.assertNotEqual(before,evidence.source_state(root,git))
   others[0]='';stage[0]=stage[0].replace('a'*40,'b'*40)
   self.assertNotEqual(before,evidence.source_state(root,git))
 def _refuse_green(self,new_source):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp).resolve();cwd=root/'source';(cwd/'src').mkdir(parents=True);target=root/'targets/task';target.mkdir(parents=True)
   (cwd/'src/backend.rs').write_text(new_source)
   def git(path,*args):
    if args==('rev-parse','HEAD'):return policy.APP
    if args==('diff','--cached','--name-only'):return ''
    if args==('diff','HEAD','--name-only'):return 'extension/rag3weaver/src/backend.rs'
    if args==('diff','--binary'):return 'fixture delta'
    raise AssertionError(args)
   ctx=SimpleNamespace(tick=lambda:None,git=git,config=dict(cargo_exe='/fixture/toolchain/bin/cargo',sdk_root='/fixture/sdk'),save=lambda *a:None,observer=SimpleNamespace(source=lambda *a:None))
   actual=broker.Broker(ctx);actual.cwd=cwd;actual.target=target;actual.completed=[dict(stage='agent-red',judge_sha256=__import__('hashlib').sha256(policy.D6_RED_TEST.encode()).hexdigest(),validator_frame=policy.validator_frame('pub fn valider_le_nom_de_l_agent(nom: &str) -> Result<String, String> { Ok(nom.into()) }\n'+policy.D6_RED_TEST),source_state=dict(head=policy.APP,files={'src/backend.rs':dict(kind='file')},index='fixture'))]
   actual.identity=lambda *a:dict(pid=99,starttime='123')
   request=dict(kind='admit',cwd=str(cwd),target=str(target),argv=policy.d6_command(policy.D6_NAME),pid=99)
   with patch.object(broker,'TASKS',root/'targets'),patch.object(evidence,'source_state',return_value=dict(head=policy.APP,files={'src/backend.rs':dict(kind='file')},index='fixture')):
    with self.assertRaises(RuntimeError):actual.handle(request,(99,42,0))
   self.assertEqual(actual.sessions,{})
 def test_actual_broker_rejects_a_changed_green_test_body(self):
  self._refuse_green('pub fn valider_le_nom_de_l_agent(nom: &str) -> Result<String, String> { Ok(nom.into()) }\n'+policy.D6_RED_TEST.replace('.is_err()', '.is_ok()'))
 def test_actual_broker_rejects_cfg_disabled_judge_and_empty_homonym(self):
  self._refuse_green('pub fn valider_le_nom_de_l_agent(nom: &str) -> Result<String, String> { Err("agent.name".into()) }\n#[cfg(any())]\n'+policy.D6_RED_TEST+'\n#[test] fn agent_name_rejects_an_inner_space() {}')
 def test_runtime_snapshot_archives_changed_identity_before_refusal(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);proc=root/'99';proc.mkdir();(proc/'environ').write_bytes(b'irrelevant=x\0');(proc/'maps').write_text('map')
   observer=observe.Observer.__new__(observe.Observer);observer.phase='d6';observer.end=100.;observer.identity=lambda p:dict(pid=99,starttime='different',uid=42,cgroup='/owned',exe='/elf',exe_sha256='sha',argv=['/elf'],cwd='/c')
   before=dict(pid=99,starttime='old',uid=42,cgroup='/owned',exe='/elf',exe_sha256='sha',argv=['/elf'],cwd='/c')
   path=root/'snapshot.json'
   with self.assertRaises(RuntimeError):observer.runtime_snapshot(proc,before,{},path)
   row=json.loads(path.read_text());self.assertEqual(row['before'],before);self.assertEqual(row['after']['starttime'],'different')
 def test_cleanup_cannot_replace_the_original_failure(self):
  original=ValueError('original refusal');later=RuntimeError('cleanup refusal');latched=[]
  def cleanup():raise later
  try:
   with evidence.preserve_failure(lambda *a:latched.append(a),cleanup):raise original
  except BaseException as error:self.assertIs(error,original)
  else:self.fail('original exception lost')
  self.assertIn('original refusal',str(latched[0]));self.assertIn('cleanup refusal',str(latched[-1]))
 def test_red_requires_the_predeclared_source_assertion_and_cause(self):
  source=policy.D6_RED_TEST
  policy.red_source(source)
  with self.assertRaises(RuntimeError):policy.red_source(source.replace('is_err()','is_ok()'))
  text='running 1 test\ntest '+policy.D6_NAME+' ... FAILED\nthread x panicked at src/backend.rs:1\n'+policy.D6_RED_MESSAGE+'\ntest result: FAILED. 0 passed; 1 failed; 0 ignored;'
  policy.judged(text,policy.D6_NAME,True,1)
  with self.assertRaises(RuntimeError):policy.judged(text.replace(policy.D6_RED_MESSAGE,'another assertion'),policy.D6_NAME,True,1)
if __name__=='__main__':unittest.main(failfast=True)
