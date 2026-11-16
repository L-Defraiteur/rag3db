"""Pure pilot guard checks: AST slices only, fake files/clock/processes, no Rust/proc scan."""
import ast
from pathlib import Path
from types import SimpleNamespace
import unittest

SOURCE_PATH = Path(__file__).with_name('qualify-derived-admission-red.py')
TREE = ast.parse(SOURCE_PATH.read_text())
MAIN = next(n for n in TREE.body if isinstance(n, ast.FunctionDef) and n.name == 'main')
PUBLISH = next(n for n in MAIN.body if isinstance(n, ast.FunctionDef) and n.name == 'publish')
POST_INDEX = next(i for i, n in enumerate(MAIN.body) if isinstance(n, ast.Assign)
                  and any(isinstance(t, ast.Name) and t.id == 'post' for t in n.targets))
TAIL = ast.FunctionDef(name='finish', args=ast.arguments(posonlyargs=[], args=[], kwonlyargs=[], kw_defaults=[], defaults=[]),
                       body=MAIN.body[POST_INDEX:], decorator_list=[])
TAIL_MODULE = ast.fix_missing_locations(ast.Module(body=[TAIL], type_ignores=[]))


class FakePath:
    def __init__(self, value): self.value = str(value)
    def __truediv__(self, name): return FakePath(self.value + '/' + str(name))
    def __str__(self): return self.value
    def iterdir(self): return iter(())
    def is_absolute(self): return self.value.startswith('/')
    @property
    def parts(self): return tuple(self.value.split('/'))


def final_context(mutation=None):
    targets = ['test_derived_fixture_admission', 'test_derived_in_ram', 'test_sfx_v3_ground_truth']
    artifacts = {name: dict(path='elf-' + name, sha256='hash-' + name) for name in targets}
    hashes = {row['path']: row['sha256'] for row in artifacts.values()}
    hashes.update({'T/validation-provenance.json': 'marker', 'Q/frozen.py': 'copy', 'Q/preflight.json': 'preflight'})
    if mutation == 'elf': hashes[artifacts[targets[0]]['path']] = 'mutated'
    if mutation == 'copy': hashes['Q/frozen.py'] = 'mutated'
    published = []
    def check_output(argv, **kwargs):
        if 'rev-parse' in argv: return 'changed-head\n' if mutation == 'head' else 'source\n'
        return ' M modified.rs\n' if mutation == 'dirty' else ''
    ns = dict(data={'target_creator_sha256': 'marker', 'copy_hashes': {'frozen.py': 'copy'}},
              scratch=FakePath('scratch'), target=FakePath('T'), package=FakePath('Q'), root=FakePath('W'),
              preflight=FakePath('Q/preflight.json'), preflight_sha256='preflight',
              artifacts=artifacts, TARGETS=targets, SOURCE='source', WITNESS='witness', deadline=600,
              Path=FakePath, subprocess=SimpleNamespace(check_output=check_output),
              checked_path=lambda value, **kwargs: FakePath(value),
              sha=lambda path: hashes[str(path)], budget=lambda: None,
              quiescence=lambda data, phase: {'admitted': True},
              publish=lambda name, value: published.append((name, value)))
    exec(compile(TAIL_MODULE, str(SOURCE_PATH), 'exec'), ns)
    return ns['finish'], published


class PublicationGuards(unittest.TestCase):
    def test_expiry_during_open_prevents_dump(self):
        clock = [0]
        dumped = []
        class File:
            def __enter__(self): clock[0] = 19; return self
            def __exit__(self, *args): pass
            def flush(self): pass
            def fileno(self): return 1
        class Output:
            def __truediv__(self, name): return self
            def open(self, mode): return File()
        def budget():
            assert 20 - clock[0] > 5, 'deadline expired'
        ns = dict(output=Output(), budget=budget,
                  json=SimpleNamespace(dump=lambda *args, **kwargs: dumped.append(True)),
                  os=SimpleNamespace(fsync=lambda fd: None))
        exec(compile(ast.Module(body=[PUBLISH], type_ignores=[]), str(SOURCE_PATH), 'exec'), ns)
        with self.assertRaises(AssertionError): ns['publish']('result.json', {})
        self.assertEqual(dumped, [])

    def test_unchanged_final_state_can_publish(self):
        finish, published = final_context()
        self.assertEqual(finish(), 101)
        self.assertEqual([name for name, _ in published], ['expected-red.json'])

    def test_changed_elf_is_refused_before_final_result(self):
        finish, published = final_context('elf')
        with self.assertRaises(AssertionError): finish()
        self.assertEqual(published, [])

    def test_changed_copy_is_refused_before_final_result(self):
        finish, published = final_context('copy')
        with self.assertRaises(AssertionError): finish()
        self.assertEqual(published, [])

    def test_changed_head_is_refused_before_final_result(self):
        finish, published = final_context('head')
        with self.assertRaises(AssertionError): finish()
        self.assertEqual(published, [])

    def test_dirty_source_is_refused_before_final_result(self):
        finish, published = final_context('dirty')
        with self.assertRaises(AssertionError): finish()
        self.assertEqual(published, [])


if __name__ == '__main__':
    unittest.main()
