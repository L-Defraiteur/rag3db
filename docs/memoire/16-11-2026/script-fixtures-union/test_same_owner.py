"""Real common same-owner guard, fake Cargo only, with real private Git fixture."""
from pathlib import Path
import ast,os,shutil,subprocess,tempfile,unittest
import cache_state

class SameOwner(unittest.TestCase):
    def fixture(self,foreign=False):
        with tempfile.TemporaryDirectory(prefix='union-guard-fixture-') as tmp:
            root=Path(tmp);source=root/'source';source.mkdir();(source/'scripts').mkdir();target=root/'target';target.mkdir()
            original=Path(__file__).resolve().parents[4]/'extension/rag3weaver/scripts/garde_target.sh';shutil.copy2(original,source/'scripts/garde_target.sh');(source/'file').write_text('fixture')
            def git(*args):return subprocess.run(['git','-C',str(source),*args],capture_output=True,text=True,check=True)
            git('init','-q');git('add','-A');git('-c','user.name=Lucie Defraiteur','-c','user.email=luciedefraiteur@gmail.com','commit','-qm','fixture')
            (target/'.arbre-du-target').write_text(str(source)+'\n');(target/'.sous-modules-du-target').write_text('');fp=target/'debug/.fingerprint/local';fp.mkdir(parents=True);(fp/'lib.json').write_text('known')
            fakebin=root/'bin';fakebin.mkdir();trace=root/'cargo-called';cargo=fakebin/'cargo';cargo.write_text('#!/bin/sh\necho called > "$CARGO_TRACE"\nexit 99\n');cargo.chmod(0o755)
            env=dict(os.environ,PATH=str(fakebin)+':'+os.environ['PATH'],CARGO_TRACE=str(trace));env.pop('RAG3WEAVER_TARGET_REPRENDRE',None)
            tree=ast.parse(Path(__file__).with_name('qualify.py').read_text());driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
            def command(label):
                call=next(n for n in ast.walk(driver) if isinstance(n,ast.Call) and isinstance(n.func,ast.Name) and n.func.id=='run' and n.args and isinstance(n.args[0],ast.Constant) and n.args[0].value==label)
                return eval(compile(ast.Expression(call.args[1]),'actual-common-guard-command','eval'),dict(C=source,T=target))
            if foreign:
                other=root/'other';git('worktree','add','--detach',str(other),'HEAD');output=target/'debug/build/local/output';output.parent.mkdir(parents=True);output.write_text('cargo:rerun-if-changed='+str(other/'file')+'\n')
            before=cache_state.inventory(target,None);result=subprocess.run(command('same-owner-preflight'),env=env,capture_output=True,text=True,timeout=10)
            self.assertEqual(result.returncode,42 if foreign else 0,result.stdout+result.stderr)
            if not foreign:
                result=subprocess.run(command('same-owner-guard'),env=env,capture_output=True,text=True,timeout=10);self.assertEqual(result.returncode,0,result.stdout+result.stderr)
            self.assertFalse(trace.exists(),'no metadata or Cargo allowed in same-owner fixture');self.assertEqual(cache_state.inventory(target,None),before)
    def test_actual_same_owner_guard_preserves_complete_inventory_without_cargo(self):self.fixture()
    def test_actual_preflight_refuses_foreign_buildrs_without_mutation(self):self.fixture(True)
if __name__=='__main__':unittest.main(failfast=True)
