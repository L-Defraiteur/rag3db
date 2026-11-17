"""Real common-launch AST -> real controller -> real supervisor, fake OS/children."""
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import ast,json,os,unittest
import lifecycle as c
from test_grant_watch_controller import FakeWorld

class ControllerEntry(unittest.TestCase):
 def execute(self,scenario):
  world=FakeWorld(scenario);self.addCleanup(world.tmp.cleanup);world.cfgpath.unlink()
  for name in ['lifecycle.py','lifecycle_health_policy.py','lifecycle-provider-expectations.json']:
   source=Path(c.__file__).with_name(name);(world.q/name).write_bytes(source.read_bytes())
  prepared=dict(pilot='f'*40,python='/fixture/python',python_sha256='c'*64,files={n:world.sha(world.q/n) for n in ['grant_watch_actor.py','lifecycle.py','lifecycle_health_policy.py','lifecycle-provider-expectations.json']})
  (world.q/'prepared.json').write_text(json.dumps(prepared));(world.q/'allocation.json').write_text(json.dumps(dict(reference='C7-fixture')))
  world.controller_process=dict(world.snapshot,pid=321,ppid=123,start='31',argv=['/fixture/python','-B',str(world.q/'lifecycle.py'),'--controller'],script_path=str(world.q/'lifecycle.py'),script_sha256=prepared['files']['lifecycle.py'])
  scope=dict(pid=123,debut='42',cgroup='/fixture',script_sha256='e'*64,echeance=800.)
  tree=ast.parse(Path(c.__file__).read_text());compose=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='compose');entry=next(n for n in ast.walk(compose) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='rc' for t in n.targets))
  def launch(argv,**kwargs):
   self.assertEqual(argv[-1],'--controller','the common entry must not execute DB directly')
   return SimpleNamespace(returncode=world.run(entry=c.controller))
  namespace=dict(vars(c),cfg=prepared,Q=world.q,S=world.root/'scopes')
  with patch.object(c,'P',world.root),patch.object(c,'Q',world.q),patch.object(c.a,'P',world.root),patch.object(c.a,'Q',world.q),patch.object(c.q,'P',world.root),patch.object(c,'guard'),patch.object(c.a,'bind'),patch.object(c,'active_scope',return_value=(scope,800.,{})),patch.object(c.watch_actor,'python_role_snapshot',side_effect=lambda pid,*a:world.process(pid)),patch.object(c.subprocess,'run',side_effect=launch),patch.dict(c.os.environ,{'RAG3WEAVER_VALIDATION_RECU':str(world.root/'authority.json')}):
   exec(compile(ast.Module(body=[entry],type_ignores=[]),'<actual-common-controller-entry>','exec'),namespace)
  return world,namespace['rc']
 def test_real_common_entry_cannot_start_db_before_arm(self):
  world,rc=self.execute('never-armed');self.assertNotEqual(rc,0);self.assertEqual(world.actions,[]);self.assertTrue(world.path('first_red').exists())
 def test_real_common_entry_reaches_driver_only_after_arm_and_releases(self):
  world,rc=self.execute('valid');self.assertEqual(rc,0);self.assertEqual(world.actions,['DB','READY','HEALTH','WRITER']);self.assertTrue(world.path('release').exists());self.assertFalse(world.path('first_red').exists())

if __name__=='__main__':unittest.main()
