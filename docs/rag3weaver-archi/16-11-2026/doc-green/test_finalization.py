"""Execute the real finalization AST with private files, no processes or signals."""
import ast
import contextlib
import hashlib
import io
import json
from pathlib import Path
import sys
import tempfile
import types
import unittest
import cache_state

SOURCE = Path(__file__).with_name('qualify_green.py')


class Finalization(unittest.TestCase):
    def outcome(self, *, scope_code=0, authority_state='termine', bus_exists=False, external_rc=0, times=(50, 50)):
        tree = ast.parse(SOURCE.read_text())
        prepare = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'prepare')
        start = next(i for i,n in enumerate(prepare.body) if isinstance(n, ast.Assign) and any(isinstance(t,ast.Name) and t.id == 'verdicts' for t in n.targets))
        code = compile(ast.Module(body=prepare.body[start:], type_ignores=[]), 'actual-finalization', 'exec')
        with tempfile.TemporaryDirectory(prefix='cx-red-finalization-') as directory:
            p = Path(directory); scope = p/'supervision/validation-fixture'; scope.mkdir(parents=True)
            bus = p/'bus'
            if bus_exists: bus.mkdir()
            (scope/'verdict.json').write_text(json.dumps(dict(verdict='VERT',code=scope_code,restes=[],nettoye=True,prive=str(bus))))
            (scope/'autorite.json').write_text(json.dumps(dict(etat=authority_state,echeance=100)))
            (p/'driver-result.json').write_text(json.dumps(dict(status='GREEN_TARGETED_ONLY')))
            binary=p/'binary'; binary.write_bytes(b'fixture-elf'); (p/'native-fixture-elf').write_bytes(binary.read_bytes())
            digest = lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest()
            (p/'ledger-before-witness.json').write_text(json.dumps(dict(binary=str(binary),sha256=digest(binary))))
            session=p/'session';session.mkdir();cache=session/'active';cache.mkdir()
            old=session/'old';old.write_bytes(b'frozen')
            frozen=cache_state.inventory(session,cache)
            red=p/'red';red.mkdir();(red/'frozen-old-cache.json').write_text(json.dumps(frozen))
            st=cache.lstat();identity=dict(device=st.st_dev,inode=st.st_ino,uid=st.st_uid,path=str(cache))
            moments=iter(times); last=times[-1]
            namespace=dict(P=p,Path=Path,json=json,sys=sys,result=types.SimpleNamespace(returncode=external_rc),
                           time=types.SimpleNamespace(monotonic=lambda:next(moments,last)),
                           cfg={'cache_identity':identity},guard=lambda _:None,sha=digest,empty_cache=cache_state,SESSION_TARGET=session,T=cache,RED=red,
                           save=lambda name,value:(p/name).write_text(json.dumps(value)))
            with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(SystemExit): exec(code,namespace)
            return json.loads((p/'result.json').read_text())

    def test_complete_targeted_green_is_not_full_qualification(self):
        result=self.outcome()
        self.assertEqual(result['status'],'GREEN_TARGETED_ONLY')
        self.assertTrue(result['not_a_full_qualification'])

    def test_wrong_scope_code_is_not_targeted_green(self):
        self.assertNotEqual(self.outcome(scope_code=1)['status'],'GREEN_TARGETED_ONLY')

    def test_active_authority_is_not_targeted_green(self):
        self.assertNotEqual(self.outcome(authority_state='actif')['status'],'GREEN_TARGETED_ONLY')

    def test_remaining_bus_is_not_targeted_green(self):
        self.assertNotEqual(self.outcome(bus_exists=True)['status'],'GREEN_TARGETED_ONLY')

    def test_outer_timeout_is_not_targeted_green(self):
        self.assertNotEqual(self.outcome(external_rc=124)['status'],'GREEN_TARGETED_ONLY')

    def test_expiry_after_hashes_is_not_targeted_green(self):
        self.assertNotEqual(self.outcome(times=(101,101))['status'],'GREEN_TARGETED_ONLY')

    def test_expiry_before_publication_is_not_targeted_green(self):
        self.assertNotEqual(self.outcome(times=(99,101))['status'],'GREEN_TARGETED_ONLY')


if __name__=='__main__': unittest.main()
