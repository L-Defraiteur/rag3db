"""Exercise the actual driver environment under a smaller poste allocation."""
import ast, tempfile, types, unittest
from pathlib import Path

class Environment(unittest.TestCase):
    def environment(self, fils):
        tree=ast.parse(Path(__file__).with_name('qualify_green.py').read_text())
        driver=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='driver')
        start=next(i for i,n in enumerate(driver.body) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='env' for t in n.targets))
        end=next(i for i,n in enumerate(driver.body) if isinstance(n,ast.FunctionDef) and n.name=='run')
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)
            ns=dict(P=p,R=p,T=p/'target',C=p/'source',LIB='fixture-lib',
                    os=types.SimpleNamespace(environ={'HOME':str(p),'PATH':'/usr/bin','POSTE_FILS':str(fils)}),save=lambda *_:None)
            exec(compile(ast.Module(body=driver.body[start:end],type_ignores=[]),'actual-driver-environment','exec'),ns)
            return ns['env']
    def test_smaller_poste_allocation_bounds_real_cargo_jobs(self):
        env=self.environment(2)
        self.assertEqual(env['CARGO_BUILD_JOBS'],'2')
        self.assertEqual(env['POSTE_FILS'],'2')
    def test_zero_poste_allocation_is_refused(self):
        with self.assertRaises(AssertionError):self.environment(0)
    def test_larger_allocation_keeps_the_eight_job_cap(self):
        self.assertEqual(self.environment(16)['CARGO_BUILD_JOBS'],'8')

if __name__=='__main__':unittest.main(failfast=True)
