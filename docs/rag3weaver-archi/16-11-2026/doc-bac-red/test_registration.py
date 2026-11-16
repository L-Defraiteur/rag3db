"""Read actual Cargo/main/Rust fixture and execute only the driver's argv AST."""
from pathlib import Path
import ast,json,re,tomllib,unittest
import contracts as proof
import qualify_red as q

HERE=Path(__file__).parent
CRATE=HERE.parents[3]/'extension/rag3weaver'

def registration():
    cargo=tomllib.loads((CRATE/'Cargo.toml').read_text())
    assert cargo['package']['autotests'] is False
    targets=[t for t in cargo['test'] if t['path']=='tests/e2e/main.rs'];assert len(targets)==1
    main=CRATE/targets[0]['path'];text=main.read_text()
    rows=re.findall(r'#\[path\s*=\s*"([^"]+)"\]\s*mod\s+(\w+)\s*;',text)
    modules=[(path,name) for path,name in rows if (main.parent/path).resolve()==(CRATE/'tests/e2e_equipier.rs').resolve()]
    assert len(modules)==1
    return targets[0]['name'],modules[0][1]

def real_build_command():
    tree=ast.parse((HERE/'qualify_red.py').read_text())
    driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
    statements=[n for n in ast.walk(driver) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id in ['selection','command'] for t in n.targets)]
    assert len(statements)==2
    statements.sort(key=lambda n:n.lineno)
    scope=dict(label='e2e',FEATURES=q.FEATURES)
    exec(compile(ast.Module(body=statements,type_ignores=[]),'actual-driver-argv','exec'),scope)
    return scope['command']

class Registration(unittest.TestCase):
    def test_a_parent_and_child_are_the_same_qualified_main_module_name(self):
        _,module=registration();name='documentation_fixture_uses_the_declared_private_sandbox_before_add'
        expected=module+'::'+name
        body=re.search(r'fn '+name+r'\(\)\s*\{(.*?)\n\}',(CRATE/'tests/e2e_equipier.rs').read_text(),re.S).group(1)
        child=re.search(r'witness_declared_sandbox_setup\(\s*"([^"]+)"',body).group(1)
        self.assertEqual(child,expected,'the SAME-ELF child must select the actual registered module test')
        self.assertEqual(proof.NAMES['e2e'],expected)

    def test_b_driver_selects_the_real_cargo_target(self):
        target,_=registration();command=real_build_command()
        self.assertEqual(command[command.index('--test')+1],target)
        self.assertIn('--no-run',command);self.assertIn('--locked',command);self.assertIn('--offline',command)

    def test_c_artifact_json_name_matches_the_same_registered_target(self):
        target,_=registration();tree=ast.parse((HERE/'qualify_red.py').read_text())
        fn=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='artifact')
        assignment=next(n for n in ast.walk(fn) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='name' for t in n.targets))
        scope=dict(label='e2e');exec(compile(ast.Module(body=[assignment],type_ignores=[]),'actual-artifact-name','exec'),scope)
        self.assertEqual(scope['name'],target)

    def test_d_declared_features_cover_the_actual_fixture_cfg(self):
        text=(CRATE/'tests/e2e_equipier.rs').read_text()
        cfg=re.search(r'#!\[cfg\(all\((.*?)\)\)\]',text,re.S).group(1)
        features=set(re.findall(r'feature\s*=\s*"([^"]+)"',cfg))
        self.assertTrue(features and features.issubset(q.FEATURES))

if __name__=='__main__':unittest.main(failfast=True)
