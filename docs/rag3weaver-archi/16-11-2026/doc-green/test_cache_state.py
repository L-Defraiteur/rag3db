"""Private filesystem and fake-Cargo checks of the actual initial-cache path."""
from pathlib import Path
import ast, os, shutil, subprocess, tempfile, unittest
from unittest.mock import patch
import cache_state as empty_cache

class EmptyCacheTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)/'session';self.root.mkdir()
        self.old=self.root/'debug';self.old.mkdir();(self.old/'artifact').write_bytes(b'historical ELF proof')
        self.cache=self.root/'native-doc-ea670-v1'
    def test_exclusive_new_directory_has_new_identity_and_no_import(self):
        before=empty_cache.inventory(self.root,self.cache)
        identity=empty_cache.create_new(self.root,self.cache)
        empty_cache.require_identity(self.cache,identity);self.assertFalse(list(self.cache.iterdir()))
        empty_cache.verify_frozen(self.root,self.cache,before)
        with self.assertRaises(AssertionError):empty_cache.create_new(self.root,self.cache)
    def test_nonempty_existing_cache_is_refused(self):
        self.cache.mkdir();(self.cache/'foreign.fingerprint').write_text('old')
        with self.assertRaises(AssertionError):empty_cache.create_new(self.root,self.cache)
    def test_symlink_cache_is_refused(self):
        self.cache.symlink_to(self.old,target_is_directory=True)
        with self.assertRaises(AssertionError):empty_cache.create_new(self.root,self.cache)
    def test_cache_identity_replacement_is_refused(self):
        identity=empty_cache.create_new(self.root,self.cache)
        self.cache.rename(self.root/'kept-original');self.cache.mkdir()
        with self.assertRaises(AssertionError):empty_cache.require_identity(self.cache,identity)
    def test_any_old_artifact_change_is_refused(self):
        before=empty_cache.inventory(self.root,self.cache)
        (self.old/'artifact').write_bytes(b'changed')
        with self.assertRaises(AssertionError):empty_cache.verify_frozen(self.root,self.cache,before)
    def test_combined_old_and_new_limit_is_checked(self):
        with patch.object(empty_cache.subprocess,'check_output',return_value=str(empty_cache.LIMIT+1)+'\t/root'):
            with self.assertRaises(AssertionError):empty_cache.require_size(self.root)
    def test_real_common_initial_guard_never_invokes_cargo_or_imports_old_files(self):
        identity=empty_cache.create_new(self.root,self.cache);before=empty_cache.inventory(self.root,self.cache)
        fakebin=self.root.parent/'bin';fakebin.mkdir();trace=self.root.parent/'cargo-called'
        cargo=fakebin/'cargo';cargo.write_text('#!/bin/sh\necho called > "$CARGO_TRACE"\nexit 99\n');cargo.chmod(0o755)
        source=self.root.parent/'source';source.mkdir()
        common=Path(__file__).resolve().parents[4]/'extension/rag3weaver/scripts/garde_target.sh'
        env=dict(os.environ,PATH=str(fakebin)+':'+os.environ['PATH'],CARGO_TRACE=str(trace))
        env.pop('RAG3WEAVER_TARGET_REPRENDRE',None)
        result=subprocess.run(['bash','-c','. "$1"; garder_le_target "$2" "$3"','fixture',str(common),str(self.cache),str(source)],env=env,capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertFalse(trace.exists());self.assertEqual((self.cache/'.arbre-du-target').read_text().strip(),str(source))
        self.assertEqual({p.name for p in self.cache.iterdir()},{'.arbre-du-target','.sous-modules-du-target'})
        empty_cache.require_identity(self.cache,identity);empty_cache.verify_frozen(self.root,self.cache,before)

    def same_owner_fixture(self):
        source=self.root.parent/'git-source';source.mkdir();(source/'scripts').mkdir()
        original=Path(__file__).resolve().parents[4]/'extension/rag3weaver/scripts/garde_target.sh'
        shutil.copy2(original,source/'scripts/garde_target.sh');(source/'file').write_text('fixture')
        def git(*args):
            return subprocess.run(['git','-C',str(source),*args],check=True,capture_output=True,text=True)
        git('init','-q');git('add','-A');git('-c','user.name=Fixture','-c','user.email=fixture@example.org','commit','-qm','fixture')
        empty_cache.create_new(self.root,self.cache)
        (self.cache/'.arbre-du-target').write_text(str(source)+'\n');(self.cache/'.sous-modules-du-target').write_text('')
        fp=self.cache/'debug/.fingerprint/local-0123456789abcdef';fp.mkdir(parents=True);(fp/'lib.json').write_text('known')
        fakebin=self.root.parent/'fakebin';fakebin.mkdir();cargo=fakebin/'cargo'
        trace=self.root.parent/'metadata-called';cargo.write_text('#!/bin/sh\necho called > "$CARGO_TRACE"\nexit 99\n');cargo.chmod(0o755)
        env=dict(os.environ,PATH=str(fakebin)+':'+os.environ['PATH'],CARGO_TRACE=str(trace));env.pop('RAG3WEAVER_TARGET_REPRENDRE',None)
        tree=ast.parse(Path(__file__).with_name('qualify_green.py').read_text())
        driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
        assignment=next(n for n in ast.walk(driver) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='command' for t in n.targets))
        ns=dict(C=source,T=self.cache)
        exec(compile(ast.Module(body=[assignment],type_ignores=[]),'actual-preflight-command','exec'),ns)
        return source,git,env,trace,ns['command']
    def test_actual_same_owner_preflight_and_guard_preserve_every_fingerprint(self):
        source,git,env,trace,command=self.same_owner_fixture()
        before=empty_cache.inventory(self.cache,None)
        check=subprocess.run(command,env=env,capture_output=True,text=True,timeout=10)
        self.assertEqual(check.returncode,0,check.stdout+check.stderr)
        result=subprocess.run(['bash','-c','. "$1"; garder_le_target "$2" "$3"','fixture',str(source/'scripts/garde_target.sh'),str(self.cache),str(source)],env=env,capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr);self.assertFalse(trace.exists())
        self.assertEqual(empty_cache.inventory(self.cache,None),before)
    def test_actual_preflight_refuses_foreign_build_script_before_any_mutation(self):
        source,git,env,trace,command=self.same_owner_fixture()
        foreign=self.root.parent/'foreign';git('worktree','add','--detach',str(foreign),'HEAD')
        output=self.cache/'debug/build/local-0123456789abcdef/output';output.parent.mkdir(parents=True)
        output.write_text('cargo:rerun-if-changed='+str(foreign/'file')+'\n')
        before=empty_cache.inventory(self.cache,None)
        result=subprocess.run(command,env=env,capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,42,result.stdout+result.stderr)
        self.assertFalse(trace.exists());self.assertEqual(empty_cache.inventory(self.cache,None),before)

if __name__=='__main__':unittest.main(failfast=True)
