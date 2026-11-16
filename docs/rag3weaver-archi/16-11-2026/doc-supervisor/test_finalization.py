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

SOURCE = Path(__file__).with_name('qualify_red.py')


class Finalization(unittest.TestCase):
    def outcome(self, *, scope_code=101, authority_state='termine', bus_exists=False, external_rc=1, times=(50, 50)):
        tree = ast.parse(SOURCE.read_text())
        prepare = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'prepare')
        start = next(i for i,n in enumerate(prepare.body) if isinstance(n, ast.Assign) and any(isinstance(t,ast.Name) and t.id == 'verdicts' for t in n.targets))
        code = compile(ast.Module(body=prepare.body[start:], type_ignores=[]), 'actual-finalization', 'exec')
        with tempfile.TemporaryDirectory(prefix='cx-red-finalization-') as directory:
            p = Path(directory); scope = p/'supervision/validation-fixture'; scope.mkdir(parents=True)
            bus = p/'bus'
            if bus_exists: bus.mkdir()
            (scope/'verdict.json').write_text(json.dumps(dict(verdict='ROUGE',code=scope_code,restes=[],nettoye=True,prive=str(bus))))
            (scope/'autorite.json').write_text(json.dumps(dict(etat=authority_state,echeance=100)))
            (p/'driver-result.json').write_text(json.dumps(dict(status='EXPECTED_RED')))
            binary=p/'binary'; binary.write_bytes(b'fixture-elf'); (p/'native-fixture-elf').write_bytes(binary.read_bytes())
            digest = lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest()
            (p/'ledger-before-witness.json').write_text(json.dumps(dict(binary=str(binary),sha256=digest(binary))))
            moments=iter(times); last=times[-1]
            namespace=dict(P=p,Path=Path,json=json,sys=sys,result=types.SimpleNamespace(returncode=external_rc),
                           time=types.SimpleNamespace(monotonic=lambda:next(moments,last)),
                           cfg={},guard=lambda _:None,sha=digest,
                           save=lambda name,value:(p/name).write_text(json.dumps(value)))
            with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(SystemExit): exec(code,namespace)
            return json.loads((p/'result.json').read_text())

    def test_complete_red_is_still_a_red(self):
        result=self.outcome()
        self.assertEqual(result['status'],'EXPECTED_RED')
        self.assertTrue(result['not_a_green_qualification'])

    def test_wrong_scope_code_is_not_expected_red(self):
        self.assertNotEqual(self.outcome(scope_code=1)['status'],'EXPECTED_RED')

    def test_active_authority_is_not_expected_red(self):
        self.assertNotEqual(self.outcome(authority_state='actif')['status'],'EXPECTED_RED')

    def test_remaining_bus_is_not_expected_red(self):
        self.assertNotEqual(self.outcome(bus_exists=True)['status'],'EXPECTED_RED')

    def test_outer_timeout_is_not_expected_red(self):
        self.assertNotEqual(self.outcome(external_rc=124)['status'],'EXPECTED_RED')

    def test_expiry_after_hashes_is_not_expected_red(self):
        self.assertNotEqual(self.outcome(times=(101,101))['status'],'EXPECTED_RED')

    def test_expiry_before_publication_is_not_expected_red(self):
        self.assertNotEqual(self.outcome(times=(99,101))['status'],'EXPECTED_RED')


if __name__=='__main__': unittest.main()
